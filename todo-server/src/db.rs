use crate::models::{SyncItem, SyncList};
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
        }

        Ok(())
    }

    pub fn get_changes_since(&self, since: &str) -> Result<(Vec<SyncList>, Vec<SyncItem>)> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, pos, updated_at, deleted_at FROM lists WHERE updated_at > ?1",
        )?;
        let lists: Vec<SyncList> = stmt
            .query_map(params![since], |row| {
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
            .query_map(params![since], |row| {
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
