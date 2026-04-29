mod auth;
mod db;
mod models;
mod routes;

use auth::require_auth;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use db::Database;
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

#[tokio::main]
async fn main() {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string());
    let db_path = PathBuf::from(&data_dir).join("todo.db");

    std::fs::create_dir_all(&data_dir).expect("failed to create data directory");

    let db = Database::new(&db_path).expect("failed to open database");
    let state = AppState { db: Arc::new(Mutex::new(db)) };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/sync", post(routes::sync_handler))
        .layer(middleware::from_fn(require_auth))
        .layer(cors)
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("todo-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
