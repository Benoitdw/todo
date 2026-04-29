use crate::{
    config::{self, Config},
    models::{Item, List},
    sync::{run_sync, SyncClient},
    AppState,
};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

// ── Lists ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_lists(state: State<AppState>) -> Result<Vec<List>, String> {
    state.db.lock().unwrap().get_lists().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_list(state: State<AppState>, title: String, pos: f64) -> Result<List, String> {
    state
        .db
        .lock()
        .unwrap()
        .create_list(&new_id(), &title, pos)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_list(state: State<AppState>, id: String, title: String) -> Result<(), String> {
    state.db.lock().unwrap().update_list(&id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_list(state: State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_list(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_list(state: State<AppState>, id: String, pos: f64) -> Result<(), String> {
    state.db.lock().unwrap().reorder_list(&id, pos).map_err(|e| e.to_string())
}

// ── Items ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_items(state: State<AppState>, list_id: String) -> Result<Vec<Item>, String> {
    state.db.lock().unwrap().get_items(&list_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_item(
    state: State<AppState>,
    list_id: String,
    text: String,
    pos: f64,
) -> Result<Item, String> {
    state
        .db
        .lock()
        .unwrap()
        .create_item(&new_id(), &list_id, &text, pos)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_item(
    state: State<AppState>,
    id: String,
    text: String,
    checked: bool,
) -> Result<(), String> {
    state.db.lock().unwrap().update_item(&id, &text, checked).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_item(state: State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_item(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_item(state: State<AppState>, id: String, pos: f64) -> Result<(), String> {
    state.db.lock().unwrap().reorder_item(&id, pos).map_err(|e| e.to_string())
}

// ── Config & Sync ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Option<Config> {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(
    state: State<AppState>,
    server_url: String,
    token: String,
) -> Result<(), String> {
    let cfg = Config { server_url, token };
    config::save(&cfg)?;
    *state.config.lock().unwrap() = Some(cfg);
    Ok(())
}

#[tauri::command]
pub async fn test_connection(server_url: String, token: String) -> Result<(), String> {
    let client = SyncClient::new(server_url, token);
    if client.health_check().await {
        Ok(())
    } else {
        Err("Could not reach server".to_string())
    }
}

#[tauri::command]
pub async fn trigger_sync(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    match cfg {
        None => Err("No configuration found".to_string()),
        Some(c) => {
            let db = Arc::clone(&state.db);
            let client = SyncClient::new(c.server_url, c.token);
            run_sync(db, &client).await
        }
    }
}
