use crate::db::Database;
use crate::models::{SyncRequest, SyncResponse};
use std::sync::{Arc, Mutex};

pub struct SyncClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl SyncClient {
    pub fn new(base_url: String, token: String) -> Self {
        SyncClient {
            client: reqwest::Client::new(),
            base_url,
            token,
        }
    }

    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        match self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn sync(&self, req: SyncRequest) -> Result<SyncResponse, String> {
        let url = format!("{}/sync", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(&req)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("sync failed with status {}", resp.status()));
        }

        resp.json::<SyncResponse>().await.map_err(|e| e.to_string())
    }
}

pub async fn run_sync(
    db: Arc<Mutex<Database>>,
    client: &SyncClient,
) -> Result<(), String> {
    // Step 1: gather local state (brief lock, no await held)
    let (since, local_lists, local_items) = {
        let guard = db.lock().map_err(|e| e.to_string())?;
        let since = guard.get_last_sync_at().map_err(|e| e.to_string())?;
        let (lists, items) =
            guard.get_changes_since(since.as_deref()).map_err(|e| e.to_string())?;
        (since, lists, items)
    };

    let req = SyncRequest {
        since: since.unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string()),
        lists: local_lists,
        items: local_items,
    };

    // Step 2: perform network request (no lock held)
    let response = client.sync(req).await?;

    // Step 3: apply server changes (brief lock, no await held)
    {
        let guard = db.lock().map_err(|e| e.to_string())?;
        guard
            .apply_sync_changes(&response.lists, &response.items)
            .map_err(|e| e.to_string())?;
        guard
            .set_last_sync_at(&response.server_time)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
