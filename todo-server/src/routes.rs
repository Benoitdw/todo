use crate::{
    db::{now, Database},
    models::{CreateItem, CreateList, Item, List, SyncRequest, SyncResponse, UpdateItem, UpdateList},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderValue, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use std::{convert::Infallible, sync::{Arc, Mutex}};
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use uuid::Uuid;

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn sse_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.broadcast.subscribe();
    let stream = BroadcastStream::new(rx)
        .map(|_| Ok::<_, Infallible>(Event::default().data("invalidate")));
    let sse = Sse::new(stream).keep_alive(KeepAlive::default());
    let mut resp = sse.into_response();
    // Tell nginx/Synology reverse proxy not to buffer this streaming response
    resp.headers_mut().insert("X-Accel-Buffering", HeaderValue::from_static("no"));
    resp
}

pub async fn sync_handler(
    State(state): State<AppState>,
    Json(req): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, StatusCode> {
    let db: Arc<Mutex<Database>> = state.db;
    let guard = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let changed = !req.lists.is_empty() || !req.items.is_empty();

    guard
        .apply_sync_changes(&req.lists, &req.items)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (lists, items) = guard
        .get_changes_since(&req.since)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if changed {
        let _ = state.broadcast.send(());
    }

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
    let _ = state.broadcast.send(());
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
    let _ = state.broadcast.send(());
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
    let _ = state.broadcast.send(());
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
    let _ = state.broadcast.send(());
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
    let _ = state.broadcast.send(());
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
    let _ = state.broadcast.send(());
    Ok(StatusCode::NO_CONTENT)
}
