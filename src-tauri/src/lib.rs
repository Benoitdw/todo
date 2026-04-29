mod commands;
mod config;
mod db;
mod models;
mod sync;

use config::Config;
use db::Database;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub config: Mutex<Option<Config>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir")
                .join("todo.db");
            std::fs::create_dir_all(db_path.parent().unwrap()).ok();
            let db = Database::new(&db_path).expect("failed to open database");
            let db = Arc::new(Mutex::new(db));

            let cfg = config::load();

            if let Some(ref c) = cfg {
                let db_clone = Arc::clone(&db);
                let base_url = c.server_url.clone();
                let token = c.token.clone();

                tauri::async_runtime::spawn(async move {
                    let client = sync::SyncClient::new(base_url, token);

                    // Initial sync attempt
                    if client.health_check().await {
                        if let Err(e) = sync::run_sync(Arc::clone(&db_clone), &client).await {
                            eprintln!("[sync] initial sync error: {e}");
                        }
                    }

                    // Periodic sync every 30 seconds
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_secs(30));
                    loop {
                        interval.tick().await;
                        if client.health_check().await {
                            if let Err(e) =
                                sync::run_sync(Arc::clone(&db_clone), &client).await
                            {
                                eprintln!("[sync] periodic sync error: {e}");
                            }
                        }
                    }
                });
            }

            app.manage(AppState { db, config: Mutex::new(cfg) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_lists,
            commands::create_list,
            commands::update_list,
            commands::delete_list,
            commands::reorder_list,
            commands::get_items,
            commands::create_item,
            commands::update_item,
            commands::delete_item,
            commands::reorder_item,
            commands::get_config,
            commands::save_config,
            commands::test_connection,
            commands::trigger_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
