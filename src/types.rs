use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::wal::Wal;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) node_id: String,
    pub(crate) store: Arc<Mutex<HashMap<String, String>>>,
    pub(crate) seq: Arc<Mutex<u64>>,
    pub(crate) wal: Arc<Mutex<Wal>>,
}

#[derive(Deserialize)]
pub(crate) struct PutBody {
    pub(crate) value: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Operation {
    #[serde(default)]
    pub(crate) node_id: String,
    pub(crate) seq: u64,
    pub(crate) op: String,
    pub(crate) key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) value: Option<String>,
}

pub(crate) struct Config {
    pub(crate) node_id: String,
    pub(crate) port: u16,
    pub(crate) data_dir: PathBuf,
}

pub(crate) fn config() -> Result<Config, String> {
    let mut node_id = env::var("KV_NODE_ID").unwrap_or_else(|_| "node-1".into());
    let mut port = env::var("KV_PORT")
        .unwrap_or_else(|_| "8000".into())
        .parse::<u16>()
        .map_err(|_| "KV_PORT must be a valid port number".to_string())?;
    let mut data_dir = PathBuf::from(env::var("KV_DATA_DIR").unwrap_or_else(|_| "./data".into()));

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return Err("Usage: key-store [--node-id ID] [--port PORT] [--data-dir DIR]".into());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {arg}"))?;
        match arg.as_str() {
            "--node-id" => node_id = value,
            "--port" => {
                port = value
                    .parse()
                    .map_err(|_| "--port must be a valid port number".to_string())?
            }
            "--data-dir" => data_dir = PathBuf::from(value),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if node_id.is_empty()
        || node_id == "."
        || node_id == ".."
        || node_id.contains('/')
        || node_id.contains('\\')
    {
        return Err("node ID must be a non-empty directory name".into());
    }
    Ok(Config {
        node_id,
        port,
        data_dir,
    })
}
