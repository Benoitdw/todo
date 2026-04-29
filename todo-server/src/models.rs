use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncList {
    pub id: String,
    pub title: String,
    pub pos: f64,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncItem {
    pub id: String,
    pub list_id: String,
    pub text: String,
    pub checked: bool,
    pub pos: f64,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPayload {
    pub lists: Vec<SyncList>,
    pub items: Vec<SyncItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncRequest {
    pub since: String,
    pub lists: Vec<SyncList>,
    pub items: Vec<SyncItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncResponse {
    pub lists: Vec<SyncList>,
    pub items: Vec<SyncItem>,
    pub server_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub title: String,
    pub pos: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub text: String,
    pub checked: bool,
    pub pos: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateList {
    pub title: String,
    pub pos: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateList {
    pub title: Option<String>,
    pub pos: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateItem {
    pub list_id: String,
    pub text: String,
    pub pos: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItem {
    pub text: Option<String>,
    pub checked: Option<bool>,
    pub pos: Option<f64>,
}
