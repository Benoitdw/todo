use crate::{
    db::{now, Database},
    models::{SyncRequest, SyncResponse},
    AppState,
};
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::{Arc, Mutex};

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn sync_handler(
    State(state): State<AppState>,
    Json(req): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, StatusCode> {
    let db: Arc<Mutex<Database>> = state.db;
    let guard = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    guard
        .apply_sync_changes(&req.lists, &req.items)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (lists, items) = guard
        .get_changes_since(&req.since)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SyncResponse {
        lists,
        items,
        server_time: now(),
    }))
}
