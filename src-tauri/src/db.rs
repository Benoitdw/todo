use crate::models::{Item, List, SyncItem, SyncList};
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::path::Path;

pub fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let version: i32 =
            self.conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if version < 1 {
            self.conn.execute_batch(
                "
                PRAGMA foreign_keys = ON;

                CREATE TABLE IF NOT EXISTS lists (
                    id         TEXT PRIMARY KEY,
                    title      TEXT NOT NULL,
                    pos        REAL NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now')),
                    deleted_at TEXT
                );

                CREATE TABLE IF NOT EXISTS items (
                    id         TEXT PRIMARY KEY,
                    list_id    TEXT NOT NULL REFERENCES lists(id),
                    text       TEXT NOT NULL,
                    checked    INTEGER NOT NULL DEFAULT 0,
                    pos        REAL NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now')),
                    deleted_at TEXT
                );

                CREATE TABLE IF NOT EXISTS sync_meta (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                PRAGMA user_version = 1;
                ",
            )?;

            // If tables already existed without new columns, add them (errors ignored)
            self.conn
                .execute_batch(
                    "ALTER TABLE lists ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now'));",
                )
                .ok();
            self.conn
                .execute_batch("ALTER TABLE lists ADD COLUMN deleted_at TEXT;")
                .ok();
            self.conn
                .execute_batch(
                    "ALTER TABLE items ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now'));",
                )
                .ok();
            self.conn
                .execute_batch("ALTER TABLE items ADD COLUMN deleted_at TEXT;")
                .ok();
        }

        Ok(())
    }

    // ── Lists ──────────────────────────────────────────────────────────────────

    pub fn get_lists(&self) -> Result<Vec<List>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, pos FROM lists WHERE deleted_at IS NULL ORDER BY pos",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(List { id: row.get(0)?, title: row.get(1)?, pos: row.get(2)? })
        })?;
        rows.collect()
    }

    pub fn create_list(&self, id: &str, title: &str, pos: f64) -> Result<List> {
        let ts = now();
        self.conn.execute(
            "INSERT INTO lists (id, title, pos, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, pos, ts],
        )?;
        Ok(List { id: id.to_string(), title: title.to_string(), pos })
    }

    pub fn update_list(&self, id: &str, title: &str) -> Result<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE lists SET title = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            params![title, ts, id],
        )?;
        Ok(())
    }

    pub fn delete_list(&self, id: &str) -> Result<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE list_id = ?2 AND deleted_at IS NULL",
            params![ts, id],
        )?;
        self.conn.execute(
            "UPDATE lists SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![ts, id],
        )?;
        Ok(())
    }

    pub fn reorder_list(&self, id: &str, pos: f64) -> Result<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE lists SET pos = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            params![pos, ts, id],
        )?;
        Ok(())
    }

    // ── Items ──────────────────────────────────────────────────────────────────

    pub fn get_items(&self, list_id: &str) -> Result<Vec<Item>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, list_id, text, checked, pos FROM items WHERE list_id = ?1 AND deleted_at IS NULL ORDER BY pos",
        )?;
        let rows = stmt.query_map(params![list_id], |row| {
            Ok(Item {
                id: row.get(0)?,
                list_id: row.get(1)?,
                text: row.get(2)?,
                checked: row.get::<_, i32>(3)? != 0,
                pos: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn create_item(&self, id: &str, list_id: &str, text: &str, pos: f64) -> Result<Item> {
        let ts = now();
        self.conn.execute(
            "INSERT INTO items (id, list_id, text, checked, pos, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?5)",
            params![id, list_id, text, pos, ts],
        )?;
        Ok(Item {
            id: id.to_string(),
            list_id: list_id.to_string(),
            text: text.to_string(),
            checked: false,
            pos,
        })
    }

    pub fn update_item(&self, id: &str, text: &str, checked: bool) -> Result<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE items SET text = ?1, checked = ?2, updated_at = ?3 WHERE id = ?4 AND deleted_at IS NULL",
            params![text, checked as i32, ts, id],
        )?;
        Ok(())
    }

    pub fn delete_item(&self, id: &str) -> Result<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![ts, id],
        )?;
        Ok(())
    }

    pub fn reorder_item(&self, id: &str, pos: f64) -> Result<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE items SET pos = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            params![pos, ts, id],
        )?;
        Ok(())
    }

    // ── Sync meta ──────────────────────────────────────────────────────────────

    pub fn get_last_sync_at(&self) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM sync_meta WHERE key = 'last_sync_at'")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_last_sync_at(&self, ts: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_meta (key, value) VALUES ('last_sync_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![ts],
        )?;
        Ok(())
    }

    pub fn get_changes_since(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<SyncList>, Vec<SyncItem>)> {
        let since_ts = since.unwrap_or("1970-01-01T00:00:00.000Z");

        let mut stmt = self.conn.prepare(
            "SELECT id, title, pos, updated_at, deleted_at FROM lists WHERE updated_at > ?1",
        )?;
        let lists: Vec<SyncList> = stmt
            .query_map(params![since_ts], |row| {
                Ok(SyncList {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    pos: row.get(2)?,
                    updated_at: row.get(3)?,
                    deleted_at: row.get(4)?,
                })
            })?
            .collect::<Result<_>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, list_id, text, checked, pos, updated_at, deleted_at FROM items WHERE updated_at > ?1",
        )?;
        let items: Vec<SyncItem> = stmt
            .query_map(params![since_ts], |row| {
                Ok(SyncItem {
                    id: row.get(0)?,
                    list_id: row.get(1)?,
                    text: row.get(2)?,
                    checked: row.get::<_, i32>(3)? != 0,
                    pos: row.get(4)?,
                    updated_at: row.get(5)?,
                    deleted_at: row.get(6)?,
                })
            })?
            .collect::<Result<_>>()?;

        Ok((lists, items))
    }

    pub fn apply_sync_changes(
        &self,
        lists: &[SyncList],
        items: &[SyncItem],
    ) -> Result<()> {
        self.conn.execute_batch("PRAGMA foreign_keys = OFF")?;

        for list in lists {
            self.conn.execute(
                "INSERT INTO lists (id, title, pos, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                     title      = excluded.title,
                     pos        = excluded.pos,
                     updated_at = excluded.updated_at,
                     deleted_at = excluded.deleted_at
                 WHERE excluded.updated_at > lists.updated_at",
                params![list.id, list.title, list.pos, list.updated_at, list.deleted_at],
            )?;
        }

        for item in items {
            self.conn.execute(
                "INSERT INTO items (id, list_id, text, checked, pos, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                     list_id    = excluded.list_id,
                     text       = excluded.text,
                     checked    = excluded.checked,
                     pos        = excluded.pos,
                     updated_at = excluded.updated_at,
                     deleted_at = excluded.deleted_at
                 WHERE excluded.updated_at > items.updated_at",
                params![
                    item.id,
                    item.list_id,
                    item.text,
                    item.checked as i32,
                    item.pos,
                    item.updated_at,
                    item.deleted_at
                ],
            )?;
        }

        self.conn.execute_batch("PRAGMA foreign_keys = ON")?;
        Ok(())
    }
}
