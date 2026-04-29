mod auth;
mod db;
mod models;
mod routes;

use auth::require_auth;
use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use db::Database;
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

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

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "/static".to_string());

    let api_routes = Router::new()
        .route("/lists", get(routes::get_lists).post(routes::create_list))
        .route(
            "/lists/:id",
            put(routes::update_list).delete(routes::delete_list),
        )
        .route("/lists/:id/items", get(routes::get_items))
        .route("/items", post(routes::create_item))
        .route(
            "/items/:id",
            put(routes::update_item).delete(routes::delete_item),
        );

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/sync", post(routes::sync_handler))
        .nest("/api", api_routes)
        .layer(middleware::from_fn(require_auth))
        .layer(cors)
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
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
