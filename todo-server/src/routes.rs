use crate::{
    db::{now, Database},
    models::{CreateItem, CreateList, Item, List, SyncRequest, SyncResponse, UpdateItem, UpdateList},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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

pub async fn get_lists(
    State(state): State<AppState>,
) -> Result<Json<Vec<List>>, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let lists = guard.get_lists().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(lists))
}

pub async fn create_list(
    State(state): State<AppState>,
    Json(body): Json<CreateList>,
) -> Result<Json<List>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let list = guard
        .create_list(&id, &body.title, body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(list))
}

pub async fn update_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateList>,
) -> Result<StatusCode, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .update_list(&id, body.title.as_deref(), body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .delete_list(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_items(
    State(state): State<AppState>,
    Path(list_id): Path<String>,
) -> Result<Json<Vec<Item>>, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = guard.get_items(&list_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(items))
}

pub async fn create_item(
    State(state): State<AppState>,
    Json(body): Json<CreateItem>,
) -> Result<Json<Item>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let item = guard
        .create_item(&id, &body.list_id, &body.text, body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(item))
}

pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateItem>,
) -> Result<StatusCode, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .update_item(&id, body.text.as_deref(), body.checked, body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .delete_item(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
