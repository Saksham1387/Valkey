mod types;
mod wal;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde_json::Value;
use std::{
    fs,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use types::{AppState, Operation, PutBody, config};
use wal::Wal;

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(serde_json::json!({"status": "ok", "node_id": state.node_id}))
}

async fn get_key(Path(key): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    match state.store.lock().unwrap().get(&key).cloned() {
        Some(value) => (
            StatusCode::OK,
            Json(serde_json::json!({"key": key, "value": value})),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"detail": "not found"})),
        )
            .into_response(),
    }
}

async fn put_key(
    Path(key): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<PutBody>,
) -> impl IntoResponse {
    let mut seq = state.seq.lock().unwrap();
    *seq += 1;
    let operation = Operation {
        node_id: state.node_id.clone(),
        seq: *seq,
        op: "put".into(),
        key: key.clone(),
        value: Some(body.value.clone()),
    };
    if let Err(error) = state.wal.lock().unwrap().append(&operation) {
        *seq -= 1;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"detail": error})),
        )
            .into_response();
    }
    state
        .store
        .lock()
        .unwrap()
        .insert(key.clone(), body.value.clone());
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "key": key,
            "value": body.value,
            "node_id": state.node_id
        })),
    )
        .into_response()
}

async fn delete_key(Path(key): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let mut seq = state.seq.lock().unwrap();
    *seq += 1;
    let operation = Operation {
        node_id: state.node_id.clone(),
        seq: *seq,
        op: "delete".into(),
        key: key.clone(),
        value: None,
    };
    if let Err(error) = state.wal.lock().unwrap().append(&operation) {
        *seq -= 1;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"detail": error})),
        )
            .into_response();
    }
    state.store.lock().unwrap().remove(&key);
    (
        StatusCode::OK,
        Json(serde_json::json!({"deleted": key, "node_id": state.node_id})),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config().map_err(std::io::Error::other)?;
    let node_dir = config.data_dir.join(&config.node_id);
    fs::create_dir_all(&node_dir)?;
    let wal_path = node_dir.join("wal.log");
    let (wal, store, seq) = Wal::open(&wal_path).map_err(std::io::Error::other)?;
    let state = AppState {
        node_id: config.node_id.clone(),
        store: Arc::new(Mutex::new(store)),
        seq: Arc::new(Mutex::new(seq)),
        wal: Arc::new(Mutex::new(wal)),
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/keys/{key}", get(get_key).put(put_key).delete(delete_key))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("node {} listening on {}", config.node_id, addr);
    axum::serve(listener, app).await?;
    Ok(())
}
