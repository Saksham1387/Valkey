# Rust key-value store

This project is a small HTTP key-value store with an append-only write-ahead
log (WAL). Each WAL record carries a SHA-256 checksum. On startup, replay stops
at the first incomplete, malformed, or checksum-invalid record and truncates
the file there before accepting new writes. Existing bare-operation records
from the previous Python version are still readable.

## Run a node

```sh
cargo run -- --node-id node-a --port 8001 --data-dir ./data
```

The node writes to `./data/node-a/wal.log`. Start more independent nodes with a
different node ID and port; they can share a data directory because each has a
separate subdirectory.

Configuration is also available through `KV_NODE_ID`, `KV_PORT`, and
`KV_DATA_DIR`. Command-line arguments override the matching environment
variables. For example:

```sh
KV_NODE_ID=node-b KV_PORT=8002 KV_DATA_DIR=./data cargo run
```

`GET /health` returns the node ID. Use `PUT /keys/{key}` with JSON
`{"value":"..."}`, `GET /keys/{key}`, and `DELETE /keys/{key}` to access keys.

These node settings create separate store instances; they do not replicate or
coordinate data between nodes.
