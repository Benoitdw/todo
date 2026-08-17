use crate::models::{Island, Item, List, Note, Ref, SyncItem, SyncList};
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::path::Path;
use uuid::Uuid;

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

        if version < 2 {
            self.conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS notes (
                    id         TEXT PRIMARY KEY,
                    title      TEXT NOT NULL,
                    pos        REAL NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now')),
                    deleted_at TEXT
                );

                CREATE TABLE IF NOT EXISTS islands (
                    id         TEXT PRIMARY KEY,
                    note_id    TEXT NOT NULL REFERENCES notes(id),
                    kind       TEXT NOT NULL CHECK (kind IN ('text','photo','video','audio')),
                    text       TEXT NOT NULL DEFAULT '',
                    pos        REAL NOT NULL,
                    media_path TEXT,
                    media_mime TEXT,
                    media_size INTEGER,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now')),
                    deleted_at TEXT
                );

                CREATE TABLE IF NOT EXISTS links (
                    id            TEXT PRIMARY KEY,
                    src_kind      TEXT NOT NULL CHECK (src_kind IN ('island','item')),
                    src_id        TEXT NOT NULL,
                    dst_island_id TEXT NOT NULL REFERENCES islands(id),
                    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now')),
                    deleted_at    TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_islands_note ON islands(note_id);
                CREATE INDEX IF NOT EXISTS idx_links_src ON links(src_kind, src_id);
                CREATE INDEX IF NOT EXISTS idx_links_dst ON links(dst_island_id);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_links_unique
                    ON links(src_kind, src_id, dst_island_id) WHERE deleted_at IS NULL;

                PRAGMA user_version = 2;
                ",
            )?;
        }

        if version < 3 {
            // `links` is recreated rather than altered: no route has ever written
            // to it, so it is empty by construction. Its target is now any of the
            // four addressable kinds, and it holds no soft delete because it is an
            // index derived from the source text, not user data.
            self.conn.execute_batch(
                "
                DROP TABLE IF EXISTS links;

                CREATE TABLE links (
                    id         TEXT PRIMARY KEY,
                    src_kind   TEXT NOT NULL CHECK (src_kind IN ('island','item')),
                    src_id     TEXT NOT NULL,
                    dst_kind   TEXT NOT NULL CHECK (dst_kind IN ('note','island','list','item')),
                    dst_id     TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now'))
                );

                CREATE INDEX IF NOT EXISTS idx_links_src ON links(src_kind, src_id);
                CREATE INDEX IF NOT EXISTS idx_links_dst ON links(dst_kind, dst_id);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_links_unique
                    ON links(src_kind, src_id, dst_kind, dst_id);

                PRAGMA user_version = 3;
                ",
            )?;
        }

        if version < 4 {
            // Adding 'sketch' to islands.kind means widening a CHECK constraint,
            // which SQLite can only do by rebuilding the table. Every column and
            // every row is carried over verbatim; nothing else changes.
            self.conn.execute_batch(
                "
                PRAGMA foreign_keys = OFF;

                CREATE TABLE islands_v4 (
                    id         TEXT PRIMARY KEY,
                    note_id    TEXT NOT NULL REFERENCES notes(id),
                    kind       TEXT NOT NULL CHECK (kind IN ('text','photo','video','audio','sketch')),
                    text       TEXT NOT NULL DEFAULT '',
                    pos        REAL NOT NULL,
                    media_path TEXT,
                    media_mime TEXT,
                    media_size INTEGER,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S%.3fZ','now')),
                    deleted_at TEXT
                );

                INSERT INTO islands_v4
                    (id, note_id, kind, text, pos, media_path, media_mime, media_size,
                     updated_at, deleted_at)
                SELECT id, note_id, kind, text, pos, media_path, media_mime, media_size,
                       updated_at, deleted_at
                FROM islands;

                DROP TABLE islands;
                ALTER TABLE islands_v4 RENAME TO islands;

                CREATE INDEX IF NOT EXISTS idx_islands_note ON islands(note_id);

                PRAGMA foreign_keys = ON;
                PRAGMA user_version = 4;
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

    // CRUD methods

    pub fn get_lists(&self) -> Result<Vec<List>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, pos FROM lists WHERE deleted_at IS NULL ORDER BY pos",
        )?;
        let lists = stmt
            .query_map([], |row| {
                Ok(List {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    pos: row.get(2)?,
                })
            })?
            .collect::<Result<_>>()?;
        Ok(lists)
    }

    pub fn create_list(&self, id: &str, title: &str, pos: f64) -> Result<List> {
        let updated_at = now();
        self.conn.execute(
            "INSERT INTO lists (id, title, pos, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, pos, updated_at],
        )?;
        Ok(List {
            id: id.to_string(),
            title: title.to_string(),
            pos,
        })
    }

    pub fn update_list(&self, id: &str, title: Option<&str>, pos: Option<f64>) -> Result<()> {
        let updated_at = now();
        if let Some(title) = title {
            self.conn.execute(
                "UPDATE lists SET title = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![title, updated_at, id],
            )?;
        }
        if let Some(pos) = pos {
            self.conn.execute(
                "UPDATE lists SET pos = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![pos, updated_at, id],
            )?;
        }
        Ok(())
    }

    pub fn delete_list(&self, id: &str) -> Result<()> {
        let deleted_at = now();
        self.conn.execute(
            "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE list_id = ?2 AND deleted_at IS NULL",
            params![deleted_at, id],
        )?;
        self.conn.execute(
            "UPDATE lists SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![deleted_at, id],
        )?;
        Ok(())
    }

    pub fn get_items(&self, list_id: &str) -> Result<Vec<Item>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, list_id, text, checked, pos FROM items WHERE list_id = ?1 AND deleted_at IS NULL ORDER BY pos",
        )?;
        let items = stmt
            .query_map(params![list_id], |row| {
                Ok(Item {
                    id: row.get(0)?,
                    list_id: row.get(1)?,
                    text: row.get(2)?,
                    checked: row.get::<_, i32>(3)? != 0,
                    pos: row.get(4)?,
                })
            })?
            .collect::<Result<_>>()?;
        Ok(items)
    }

    pub fn create_item(&self, id: &str, list_id: &str, text: &str, pos: f64) -> Result<Item> {
        let updated_at = now();
        self.conn.execute(
            "INSERT INTO items (id, list_id, text, checked, pos, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?5)",
            params![id, list_id, text, pos, updated_at],
        )?;
        Ok(Item {
            id: id.to_string(),
            list_id: list_id.to_string(),
            text: text.to_string(),
            checked: false,
            pos,
        })
    }

    pub fn update_item(
        &self,
        id: &str,
        text: Option<&str>,
        checked: Option<bool>,
        pos: Option<f64>,
    ) -> Result<()> {
        let updated_at = now();
        if let Some(text) = text {
            self.conn.execute(
                "UPDATE items SET text = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![text, updated_at, id],
            )?;
        }
        if let Some(checked) = checked {
            self.conn.execute(
                "UPDATE items SET checked = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![checked as i32, updated_at, id],
            )?;
        }
        if let Some(pos) = pos {
            self.conn.execute(
                "UPDATE items SET pos = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![pos, updated_at, id],
            )?;
        }
        Ok(())
    }

    pub fn delete_item(&self, id: &str) -> Result<()> {
        let deleted_at = now();
        self.conn.execute(
            "UPDATE items SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![deleted_at, id],
        )?;
        Ok(())
    }

    // Notes

    pub fn get_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, pos FROM notes WHERE deleted_at IS NULL ORDER BY pos",
        )?;
        let notes = stmt
            .query_map([], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    pos: row.get(2)?,
                })
            })?
            .collect::<Result<_>>()?;
        Ok(notes)
    }

    pub fn create_note(&self, id: &str, title: &str, pos: f64) -> Result<Note> {
        let updated_at = now();
        self.conn.execute(
            "INSERT INTO notes (id, title, pos, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, pos, updated_at],
        )?;
        Ok(Note {
            id: id.to_string(),
            title: title.to_string(),
            pos,
        })
    }

    pub fn update_note(&self, id: &str, title: Option<&str>, pos: Option<f64>) -> Result<()> {
        let updated_at = now();
        if let Some(title) = title {
            self.conn.execute(
                "UPDATE notes SET title = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![title, updated_at, id],
            )?;
        }
        if let Some(pos) = pos {
            self.conn.execute(
                "UPDATE notes SET pos = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![pos, updated_at, id],
            )?;
        }
        Ok(())
    }

    /// Soft-deletes the note, its islands, and every link touching those islands.
    /// Returns the media paths of the deleted islands so the caller can unlink the files.
    pub fn delete_note(&self, id: &str) -> Result<Vec<String>> {
        let deleted_at = now();

        let mut stmt = self.conn.prepare(
            "SELECT media_path FROM islands
             WHERE note_id = ?1 AND deleted_at IS NULL AND media_path IS NOT NULL",
        )?;
        let paths: Vec<String> = stmt
            .query_map(params![id], |row| row.get(0))?
            .collect::<Result<_>>()?;
        drop(stmt);

        // Only the outgoing links go: they were derived from texts that no
        // longer exist. Incoming rows mirror text living elsewhere, so they stay
        // and `get_backlinks` filters them out by joining on live sources.
        self.conn.execute(
            "DELETE FROM links
             WHERE src_kind = 'island'
               AND src_id IN (SELECT id FROM islands WHERE note_id = ?1)",
            params![id],
        )?;
        self.conn.execute(
            "UPDATE islands SET deleted_at = ?1, updated_at = ?1 WHERE note_id = ?2 AND deleted_at IS NULL",
            params![deleted_at, id],
        )?;
        self.conn.execute(
            "UPDATE notes SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![deleted_at, id],
        )?;
        Ok(paths)
    }

    // Islands

    pub fn get_islands(&self, note_id: &str) -> Result<Vec<Island>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note_id, kind, text, pos, media_path, media_mime, media_size
             FROM islands WHERE note_id = ?1 AND deleted_at IS NULL ORDER BY pos",
        )?;
        let islands = stmt
            .query_map(params![note_id], |row| {
                Ok(Island {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    kind: row.get(2)?,
                    text: row.get(3)?,
                    pos: row.get(4)?,
                    media_path: row.get(5)?,
                    media_mime: row.get(6)?,
                    media_size: row.get(7)?,
                })
            })?
            .collect::<Result<_>>()?;
        Ok(islands)
    }

    pub fn get_island(&self, id: &str) -> Result<Option<Island>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note_id, kind, text, pos, media_path, media_mime, media_size
             FROM islands WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Island {
                id: row.get(0)?,
                note_id: row.get(1)?,
                kind: row.get(2)?,
                text: row.get(3)?,
                pos: row.get(4)?,
                media_path: row.get(5)?,
                media_mime: row.get(6)?,
                media_size: row.get(7)?,
            })
        })?;
        rows.next().transpose()
    }

    pub fn create_island(
        &self,
        id: &str,
        note_id: &str,
        kind: &str,
        text: &str,
        pos: f64,
    ) -> Result<Island> {
        let updated_at = now();
        self.conn.execute(
            "INSERT INTO islands (id, note_id, kind, text, pos, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, note_id, kind, text, pos, updated_at],
        )?;
        Ok(Island {
            id: id.to_string(),
            note_id: note_id.to_string(),
            kind: kind.to_string(),
            text: text.to_string(),
            pos,
            media_path: None,
            media_mime: None,
            media_size: None,
        })
    }

    pub fn update_island(&self, id: &str, text: Option<&str>, pos: Option<f64>) -> Result<()> {
        let updated_at = now();
        if let Some(text) = text {
            self.conn.execute(
                "UPDATE islands SET text = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![text, updated_at, id],
            )?;
        }
        if let Some(pos) = pos {
            self.conn.execute(
                "UPDATE islands SET pos = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                params![pos, updated_at, id],
            )?;
        }
        Ok(())
    }

    /// Soft-deletes the island and every link touching it.
    /// Returns its media path, if any, so the caller can unlink the file.
    pub fn delete_island(&self, id: &str) -> Result<Option<String>> {
        let deleted_at = now();
        let path = self.get_island(id)?.and_then(|i| i.media_path);

        // See delete_note: the island's own outgoing links are dropped, links
        // pointing at it are left to the backlink query's liveness filter.
        self.conn.execute(
            "DELETE FROM links WHERE src_kind = 'island' AND src_id = ?1",
            params![id],
        )?;
        self.conn.execute(
            "UPDATE islands SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![deleted_at, id],
        )?;
        Ok(path)
    }

    pub fn set_island_media(
        &self,
        id: &str,
        media_path: &str,
        media_mime: &str,
        media_size: i64,
    ) -> Result<()> {
        let updated_at = now();
        self.conn.execute(
            "UPDATE islands
             SET media_path = ?1, media_mime = ?2, media_size = ?3, updated_at = ?4
             WHERE id = ?5 AND deleted_at IS NULL",
            params![media_path, media_mime, media_size, updated_at, id],
        )?;
        Ok(())
    }

    // Links — a derived index, rewritten wholesale from the source text.

    /// Replaces every outgoing link of one source. `targets` is the deduplicated
    /// list of (kind, id) parsed out of the source's text.
    pub fn reindex_links(
        &self,
        src_kind: &str,
        src_id: &str,
        targets: &[(String, String)],
    ) -> Result<()> {
        self.conn.execute(
            "DELETE FROM links WHERE src_kind = ?1 AND src_id = ?2",
            params![src_kind, src_id],
        )?;
        let updated_at = now();
        for (dst_kind, dst_id) in targets {
            self.conn.execute(
                "INSERT OR IGNORE INTO links (id, src_kind, src_id, dst_kind, dst_id, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    Uuid::new_v4().to_string(),
                    src_kind,
                    src_id,
                    dst_kind,
                    dst_id,
                    updated_at
                ],
            )?;
        }
        Ok(())
    }

    /// Every target a link can point at, with the label used both by the `[[`
    /// palette and by token resolution at render time.
    pub fn get_refs(&self) -> Result<Vec<Ref>> {
        let mut stmt = self.conn.prepare(
            "SELECT 'note', n.id, n.title, NULL, NULL
             FROM notes n WHERE n.deleted_at IS NULL

             UNION ALL
             SELECT 'island', i.id,
                    CASE WHEN trim(i.text) = '' THEN '(' || i.kind || ')'
                         ELSE replace(i.text, char(10), ' ') END,
                    i.note_id, n.title
             FROM islands i JOIN notes n ON n.id = i.note_id
             WHERE i.deleted_at IS NULL AND n.deleted_at IS NULL

             UNION ALL
             SELECT 'list', l.id, l.title, NULL, NULL
             FROM lists l WHERE l.deleted_at IS NULL

             UNION ALL
             SELECT 'item', it.id, it.text, it.list_id, l.title
             FROM items it JOIN lists l ON l.id = it.list_id
             WHERE it.deleted_at IS NULL AND l.deleted_at IS NULL",
        )?;
        let refs = stmt
            .query_map([], |row| {
                Ok(Ref {
                    kind: row.get(0)?,
                    id: row.get(1)?,
                    label: row.get(2)?,
                    parent_id: row.get(3)?,
                    parent_label: row.get(4)?,
                })
            })?
            .collect::<Result<_>>()?;
        Ok(refs)
    }

    /// The sources pointing at one target, described the same way as a ref.
    /// Rows whose source has since been deleted are filtered out here rather
    /// than pruned on delete, so the index stays a pure function of the texts.
    pub fn get_backlinks(&self, dst_kind: &str, dst_id: &str) -> Result<Vec<Ref>> {
        let mut stmt = self.conn.prepare(
            "SELECT 'island', i.id,
                    CASE WHEN trim(i.text) = '' THEN '(' || i.kind || ')'
                         ELSE replace(i.text, char(10), ' ') END,
                    i.note_id, n.title
             FROM links lk
             JOIN islands i ON i.id = lk.src_id AND i.deleted_at IS NULL
             JOIN notes n ON n.id = i.note_id AND n.deleted_at IS NULL
             WHERE lk.src_kind = 'island' AND lk.dst_kind = ?1 AND lk.dst_id = ?2

             UNION ALL
             SELECT 'item', it.id, it.text, it.list_id, l.title
             FROM links lk
             JOIN items it ON it.id = lk.src_id AND it.deleted_at IS NULL
             JOIN lists l ON l.id = it.list_id AND l.deleted_at IS NULL
             WHERE lk.src_kind = 'item' AND lk.dst_kind = ?1 AND lk.dst_id = ?2",
        )?;
        let refs = stmt
            .query_map(params![dst_kind, dst_id], |row| {
                Ok(Ref {
                    kind: row.get(0)?,
                    id: row.get(1)?,
                    label: row.get(2)?,
                    parent_id: row.get(3)?,
                    parent_label: row.get(4)?,
                })
            })?
            .collect::<Result<_>>()?;
        Ok(refs)
    }
}
