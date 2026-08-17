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

#[derive(Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub pos: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Island {
    pub id: String,
    pub note_id: String,
    pub kind: String,
    pub text: String,
    pub pos: f64,
    pub media_path: Option<String>,
    pub media_mime: Option<String>,
    pub media_size: Option<i64>,
}

/// One addressable link target — a note, an island, a list or an item.
/// Doubles as the description of a backlink's source.
#[derive(Debug, Serialize, Deserialize)]
pub struct Ref {
    pub kind: String,
    pub id: String,
    pub label: String,
    pub parent_id: Option<String>,
    pub parent_label: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct BacklinkQuery {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNote {
    pub title: String,
    pub pos: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNote {
    pub title: Option<String>,
    pub pos: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIsland {
    pub note_id: String,
    pub kind: String,
    pub text: Option<String>,
    pub pos: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIsland {
    pub text: Option<String>,
    pub pos: Option<f64>,
}
