use crate::{
    db::{now, Database},
    models::{
        BacklinkQuery, CreateIsland, CreateItem, CreateList, CreateNote, Island, Item, List, Note,
        Ref, SyncRequest, SyncResponse, UpdateIsland, UpdateItem, UpdateList, UpdateNote,
    },
    AppState,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, Request, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures_util::StreamExt as _;
use std::{convert::Infallible, sync::{Arc, Mutex}};
use tokio::{fs, io::AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;
use tower::ServiceExt as _;
use tower_http::services::ServeFile;
use uuid::Uuid;

pub async fn health() -> StatusCode {
    StatusCode::OK
}

const LINK_KINDS: [&str; 4] = ["note", "island", "list", "item"];

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// Pulls `[[kind:uuid]]` tokens out of a source text, deduplicated and in order.
/// Anything malformed is skipped: the text is the user's, the index is ours.
fn parse_link_targets(text: &str) -> Vec<(String, String)> {
    let bytes = text.as_bytes();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    // `[` is ASCII, so any index matching it is a char boundary and the slices
    // below can never split a multi-byte character.
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(end) = text[i + 2..].find("]]") {
                let inner = &text[i + 2..i + 2 + end];
                if let Some((kind, id)) = inner.split_once(':') {
                    if LINK_KINDS.contains(&kind) && is_uuid(id) {
                        let pair = (kind.to_string(), id.to_string());
                        if !out.contains(&pair) {
                            out.push(pair);
                        }
                    }
                }
                i += 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Reindexes one source's outgoing links. Never propagates a failure: the text
/// has already been written and is the source of truth — a stale index is a
/// lesser evil than a rejected save.
fn reindex(guard: &Database, src_kind: &str, src_id: &str, text: &str) {
    if let Err(e) = guard.reindex_links(src_kind, src_id, &parse_link_targets(text)) {
        tracing::warn!("link reindex failed for {src_kind} {src_id}: {e}");
    }
}

/// Turns a source text into something readable in the palette: link tokens
/// become an arrow rather than a raw uuid, then the result is truncated.
fn clean_label(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(end) = text[i + 2..].find("]]") {
                let inner = &text[i + 2..i + 2 + end];
                if let Some((kind, id)) = inner.split_once(':') {
                    if LINK_KINDS.contains(&kind) && is_uuid(id) {
                        out.push('↗');
                        i += 2 + end + 2;
                        continue;
                    }
                }
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    let trimmed = out.trim();
    if trimmed.chars().count() > 80 {
        trimmed.chars().take(79).collect::<String>() + "…"
    } else {
        trimmed.to_string()
    }
}

fn clean_labels(mut refs: Vec<Ref>) -> Vec<Ref> {
    for r in &mut refs {
        r.label = clean_label(&r.label);
    }
    refs
}

// --- media ------------------------------------------------------------------

const DEFAULT_PHOTO_MB: usize = 15;
const DEFAULT_AUDIO_MB: usize = 25;
const DEFAULT_VIDEO_MB: usize = 200;
/// A sketch is vector markup, not pixels — it has no business being large.
const DEFAULT_SKETCH_MB: usize = 5;

fn env_mb(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
        * 1024
        * 1024
}

/// Route-level ceiling, read once at startup; the per-kind limit is enforced
/// while streaming. Overridable so the sizes can change without rebuilding the
/// image — the value must stay in step with the proxy's client_max_body_size.
pub fn max_media_bytes() -> usize {
    env_mb("MAX_PHOTO_MB", DEFAULT_PHOTO_MB)
        .max(env_mb("MAX_AUDIO_MB", DEFAULT_AUDIO_MB))
        .max(env_mb("MAX_VIDEO_MB", DEFAULT_VIDEO_MB))
}

fn max_bytes_for(kind: &str) -> Option<usize> {
    match kind {
        "photo" => Some(env_mb("MAX_PHOTO_MB", DEFAULT_PHOTO_MB)),
        "audio" => Some(env_mb("MAX_AUDIO_MB", DEFAULT_AUDIO_MB)),
        "video" => Some(env_mb("MAX_VIDEO_MB", DEFAULT_VIDEO_MB)),
        "sketch" => Some(env_mb("MAX_SKETCH_MB", DEFAULT_SKETCH_MB)),
        _ => None, // a text island carries no binary
    }
}

fn mime_matches_kind(mime: &str, kind: &str) -> bool {
    let base = mime.split(';').next().unwrap_or("").trim();
    match kind {
        "photo" => base.starts_with("image/"),
        "audio" => base.starts_with("audio/"),
        "video" => base.starts_with("video/"),
        // A sketch is stored as the SVG the editor produced: an image the
        // browser can display, and markup the editor can load back for editing.
        "sketch" => base == "image/svg+xml",
        _ => false,
    }
}

/// Cosmetic only — `media_mime` stays the source of truth for playback.
fn ext_for_mime(mime: &str) -> &'static str {
    match mime.split(';').next().unwrap_or("").trim() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/heic" | "image/heif" => "heic",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "video/mp4" => "mp4",
        "video/quicktime" => "mov",
        "video/webm" => "webm",
        "audio/webm" => "webm",
        "audio/mp4" | "audio/aac" | "audio/x-m4a" => "m4a",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "audio/wav" | "audio/x-wav" => "wav",
        _ => "bin",
    }
}

pub async fn put_island_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<Island>, StatusCode> {
    // The lock is taken to read, then released: the upload itself must never
    // hold the global database mutex, or a 200 MB video freezes the whole app.
    let island = {
        let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.get_island(&id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    }
    .ok_or(StatusCode::NOT_FOUND)?;

    // A captured photo, video or audio is a one-shot: one island, one binary,
    // replacing means delete and recreate. A sketch is the exception — it is
    // meant to be reopened and saved again.
    let previous = island.media_path.clone();
    if previous.is_some() && island.kind != "sketch" {
        return Err(StatusCode::CONFLICT);
    }
    let max = max_bytes_for(&island.kind).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

    let mime = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?
        .to_string();
    if !mime_matches_kind(&mime, &island.kind) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    let dir = state.data_dir.join("media").join(&island.note_id);
    fs::create_dir_all(&dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let part = dir.join(format!("{id}.part"));
    let final_name = format!("{id}.{}", ext_for_mime(&mime));
    let final_path = dir.join(&final_name);

    let mut file = fs::File::create(&part)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stream = body.into_data_stream();
    let mut written: usize = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(_) => {
                let _ = fs::remove_file(&part).await;
                return Err(StatusCode::BAD_REQUEST);
            }
        };
        written += chunk.len();
        if written > max {
            let _ = fs::remove_file(&part).await;
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        if file.write_all(&chunk).await.is_err() {
            let _ = fs::remove_file(&part).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    if file.flush().await.is_err() {
        let _ = fs::remove_file(&part).await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    drop(file);

    // Renamed only once complete, so a `.part` on disk always means a failed upload.
    fs::rename(&part, &final_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rel = format!("media/{}/{}", island.note_id, final_name);
    // On a sketch resave the extension is identical, so the rename above already
    // overwrote the file; this only matters if a previous save used another mime.
    if let Some(old) = previous.filter(|p| *p != rel) {
        let _ = fs::remove_file(state.data_dir.join(old)).await;
    }
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .set_island_media(&id, &rel, &mime, written as i64)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let updated = guard
        .get_island(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let _ = state.broadcast.send(());
    Ok(Json(updated))
}

pub async fn get_island_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    let island = {
        let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.get_island(&id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    }
    .ok_or(StatusCode::NOT_FOUND)?;

    let (Some(rel), Some(mime)) = (island.media_path, island.media_mime) else {
        // An island with no binary yet is "nothing", not "ok" — an <img> has to
        // be able to tell the difference.
        return Err(StatusCode::NOT_FOUND);
    };

    // ServeFile brings Range, 206 and If-Range for free; reimplementing byte
    // ranges by hand is a classic source of bugs.
    let mut resp = ServeFile::new(state.data_dir.join(rel))
        .oneshot(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_response();

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(StatusCode::NOT_FOUND);
    }
    // The stored mime wins over anything guessed from the extension.
    if let Ok(value) = HeaderValue::from_str(&mime) {
        resp.headers_mut().insert(axum::http::header::CONTENT_TYPE, value);
    }
    Ok(resp)
}

pub async fn get_refs(State(state): State<AppState>) -> Result<Json<Vec<Ref>>, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let refs = guard.get_refs().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(clean_labels(refs)))
}

pub async fn get_backlinks(
    State(state): State<AppState>,
    Query(q): Query<BacklinkQuery>,
) -> Result<Json<Vec<Ref>>, StatusCode> {
    if !LINK_KINDS.contains(&q.kind.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let refs = guard
        .get_backlinks(&q.kind, &q.id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(clean_labels(refs)))
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
    reindex(&guard, "item", &id, &body.text);
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
    if let Some(text) = body.text.as_deref() {
        reindex(&guard, "item", &id, text);
    }
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

pub async fn get_notes(
    State(state): State<AppState>,
) -> Result<Json<Vec<Note>>, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let notes = guard.get_notes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(notes))
}

pub async fn create_note(
    State(state): State<AppState>,
    Json(body): Json<CreateNote>,
) -> Result<Json<Note>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let note = guard
        .create_note(&id, &body.title, body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = state.broadcast.send(());
    Ok(Json(note))
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNote>,
) -> Result<StatusCode, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .update_note(&id, body.title.as_deref(), body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = state.broadcast.send(());
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let paths = {
        let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.delete_note(&id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    // A file that refuses to go is litter, not an error: the row is what the app
    // reads, so the delete has already succeeded as far as the user is concerned.
    for rel in paths {
        let _ = fs::remove_file(state.data_dir.join(rel)).await;
    }
    let _ = fs::remove_dir_all(state.data_dir.join("media").join(&id)).await;
    let _ = state.broadcast.send(());
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_islands(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<Json<Vec<Island>>, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let islands = guard.get_islands(&note_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(islands))
}

pub async fn create_island(
    State(state): State<AppState>,
    Json(body): Json<CreateIsland>,
) -> Result<Json<Island>, StatusCode> {
    // Rejected here rather than by the CHECK constraint, which would surface as a 500.
    if !matches!(body.kind.as_str(), "text" | "photo" | "video" | "audio" | "sketch") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let id = Uuid::new_v4().to_string();
    let text = body.text.unwrap_or_default();
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let island = guard
        .create_island(&id, &body.note_id, &body.kind, &text, body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    reindex(&guard, "island", &id, &text);
    let _ = state.broadcast.send(());
    Ok(Json(island))
}

pub async fn update_island(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateIsland>,
) -> Result<StatusCode, StatusCode> {
    let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    guard
        .update_island(&id, body.text.as_deref(), body.pos)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(text) = body.text.as_deref() {
        reindex(&guard, "island", &id, text);
    }
    let _ = state.broadcast.send(());
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_island(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let path = {
        let guard = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        guard.delete_island(&id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    if let Some(rel) = path {
        let _ = fs::remove_file(state.data_dir.join(rel)).await;
    }
    let _ = state.broadcast.send(());
    Ok(StatusCode::NO_CONTENT)
}
