use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
};

use crate::types::Operation;

pub(crate) struct Wal {
    file: File,
}

#[derive(Serialize)]
struct WalRecord<'a> {
    record: &'a Value,
    checksum: String,
}

impl Wal {
    pub(crate) fn open(path: &Path) -> Result<(Self, HashMap<String, String>, u64), String> {
        let (store, seq) = replay(path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)
            .map_err(|e| format!("cannot open WAL: {e}"))?;
        Ok((Self { file }, store, seq))
    }

    pub(crate) fn append(&mut self, operation: &Operation) -> Result<(), String> {
        let value = serde_json::to_value(operation).map_err(|e| e.to_string())?;
        let record = WalRecord {
            record: &value,
            checksum: checksum(&value)?,
        };
        let mut bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let start = self.file.metadata().map_err(|e| e.to_string())?.len();
        if let Err(error) = self
            .file
            .write_all(&bytes)
            .and_then(|()| self.file.sync_all())
        {
            // Keep later records from being appended behind a partial line.
            self.file
                .set_len(start)
                .and_then(|()| self.file.sync_all())
                .map_err(|truncate_error| {
                    format!("WAL append failed ({error}); rollback failed ({truncate_error})")
                })?;
            return Err(error.to_string());
        }
        Ok(())
    }
}

fn checksum(record: &Value) -> Result<String, String> {
    // serde_json's default object map is key-sorted, matching the canonical
    // JSON used by the Python WAL implementation.
    let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn replay(path: &Path) -> Result<(HashMap<String, String>, u64), String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((HashMap::new(), 0)),
        Err(e) => return Err(format!("cannot read WAL: {e}")),
    };
    let mut reader = BufReader::new(file);
    let mut offset = 0_u64;
    let mut good_end = 0_u64;
    let mut damaged = false;
    let mut store = HashMap::new();
    let mut seq = 0;
    let mut line = Vec::new();

    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        let start = offset;
        offset += read as u64;
        if !line.ends_with(b"\n") {
            damaged = true;
            good_end = start;
            break;
        }

        let parsed: Value = match serde_json::from_slice(&line) {
            Ok(value) => value,
            Err(_) => {
                damaged = true;
                good_end = start;
                break;
            }
        };
        let operation = if parsed.get("record").is_some() || parsed.get("checksum").is_some() {
            let record = match parsed.get("record") {
                Some(v) => v,
                None => {
                    damaged = true;
                    good_end = start;
                    break;
                }
            };
            let expected = match checksum(record) {
                Ok(v) => v,
                Err(_) => {
                    damaged = true;
                    good_end = start;
                    break;
                }
            };
            if parsed.get("checksum").and_then(Value::as_str) != Some(expected.as_str()) {
                damaged = true;
                good_end = start;
                break;
            }
            record.clone()
        } else {
            // Read pre-checksum operation records written by the Python version.
            parsed
        };

        let op = match serde_json::from_value::<Operation>(operation) {
            Ok(op) => op,
            Err(_) => {
                damaged = true;
                good_end = start;
                break;
            }
        };
        seq = seq.max(op.seq);
        match op.op.as_str() {
            "put" => {
                if let Some(value) = op.value {
                    store.insert(op.key, value);
                } else {
                    damaged = true;
                    good_end = start;
                    break;
                }
            }
            "delete" => {
                store.remove(&op.key);
            }
            _ => {
                damaged = true;
                good_end = start;
                break;
            }
        }
        good_end = offset;
    }

    if damaged {
        let file = OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.set_len(good_end)
            .map_err(|e| format!("cannot truncate damaged WAL: {e}"))?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    Ok((store, seq))
}
