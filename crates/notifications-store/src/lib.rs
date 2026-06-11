use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_NOTIFICATIONS: usize = 200;
pub const DEFAULT_LIST_LIMIT: u32 = 100;
pub const NOTIFICATION_PUSHED_EVENT: &str = "notification:pushed";

const DEDUP_INDEX_DDL: &str = "CREATE UNIQUE INDEX IF NOT EXISTS idx_notif_dedup
     ON notifications(kind, title)
     WHERE dismissed_at IS NULL
       AND kind NOT IN ('apply_success', 'apply_failure', 'apply_cancelled');";

const DEDUP_PRUNE_DDL: &str = "DELETE FROM notifications
     WHERE dismissed_at IS NULL
       AND kind NOT IN ('apply_success', 'apply_failure', 'apply_cancelled')
       AND rowid NOT IN (
         SELECT MAX(rowid) FROM notifications
         WHERE dismissed_at IS NULL
           AND kind NOT IN ('apply_success', 'apply_failure', 'apply_cancelled')
         GROUP BY kind, title
       );";

#[derive(Debug, thiserror::Error)]
pub enum NotificationsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    ApplySuccess,
    ApplyFailure,
    ApplyCancelled,
    AppUpdateAvailable,
    CatalogUpdateAvailable,
    DriverUpdateAvailable,
    SystemDriverUpdateAvailable,
    DllUpdatesAvailable,
    BackupRestored,
    ScanFailed,
    CatalogRefreshFailed,
}

impl NotificationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::ApplySuccess => "apply_success",
            Self::ApplyFailure => "apply_failure",
            Self::ApplyCancelled => "apply_cancelled",
            Self::AppUpdateAvailable => "app_update_available",
            Self::CatalogUpdateAvailable => "catalog_update_available",
            Self::DriverUpdateAvailable => "driver_update_available",
            Self::SystemDriverUpdateAvailable => "system_driver_update_available",
            Self::DllUpdatesAvailable => "dll_updates_available",
            Self::BackupRestored => "backup_restored",
            Self::ScanFailed => "scan_failed",
            Self::CatalogRefreshFailed => "catalog_refresh_failed",
        }
    }

    pub fn is_dedup_exempt(self) -> bool {
        matches!(
            self,
            Self::ApplySuccess | Self::ApplyFailure | Self::ApplyCancelled
        )
    }

    fn parse(value: &str) -> Result<Self, rusqlite::Error> {
        match value {
            "apply_success" => Ok(Self::ApplySuccess),
            "apply_failure" => Ok(Self::ApplyFailure),
            "apply_cancelled" => Ok(Self::ApplyCancelled),
            "app_update_available" => Ok(Self::AppUpdateAvailable),
            "catalog_update_available" => Ok(Self::CatalogUpdateAvailable),
            "driver_update_available" => Ok(Self::DriverUpdateAvailable),
            "system_driver_update_available" => Ok(Self::SystemDriverUpdateAvailable),
            "dll_updates_available" => Ok(Self::DllUpdatesAvailable),
            "backup_restored" => Ok(Self::BackupRestored),
            "scan_failed" => Ok(Self::ScanFailed),
            "catalog_refresh_failed" => Ok(Self::CatalogRefreshFailed),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: String,
    pub kind: NotificationKind,
    pub title: String,
    pub body: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub dismissed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub apply_id: Option<String>,
    pub game_id: Option<String>,
    pub error_class: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    /// Locale-independent identity for dedup (e.g. `driver:RTX 4070:610.47`).
    /// Localized titles change with the UI language; this key does not.
    #[serde(default)]
    pub dedup_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListFilter {
    pub include_dismissed: Option<bool>,
    pub limit: Option<u32>,
}

pub struct NotificationsStore {
    db_path: PathBuf,
}

impl NotificationsStore {
    pub fn open(db_path: PathBuf) -> Result<Self, NotificationsError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self { db_path };
        store.ensure_schema()?;
        Ok(store)
    }

    fn conn(&self) -> Result<rusqlite::Connection, NotificationsError> {
        Ok(rusqlite::Connection::open(&self.db_path)?)
    }

    fn ensure_schema(&self) -> Result<(), NotificationsError> {
        let conn = self.conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notifications (
                id            TEXT PRIMARY KEY,
                kind          TEXT NOT NULL,
                title         TEXT NOT NULL,
                body          TEXT,
                created_at    TEXT NOT NULL,
                read_at       TEXT,
                dismissed_at  TEXT,
                apply_id      TEXT,
                game_id       TEXT,
                error_class   TEXT,
                link          TEXT,
                vendor        TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_notif_created ON notifications(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notif_unread  ON notifications(read_at) WHERE read_at IS NULL;",
        )?;
        ensure_column(&conn, "link")?;
        ensure_column(&conn, "vendor")?;
        ensure_column(&conn, "dedup_key")?;
        conn.execute_batch(DEDUP_PRUNE_DDL)?;
        conn.execute_batch(DEDUP_INDEX_DDL)?;
        Ok(())
    }

    pub fn insert(&self, entry: &NotificationEntry) -> Result<bool, NotificationsError> {
        let conn = self.conn()?;
        if let Some(key) = &entry.dedup_key {
            let active_dupes: i64 = conn.query_row(
                "SELECT COUNT(*) FROM notifications
                 WHERE kind = ?1 AND dedup_key = ?2 AND dismissed_at IS NULL",
                rusqlite::params![entry.kind.as_str(), key],
                |r| r.get(0),
            )?;
            if active_dupes > 0 {
                return Ok(false);
            }
        }
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO notifications
                (id, kind, title, body, created_at, read_at, dismissed_at,
                 apply_id, game_id, error_class, link, vendor, dedup_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                entry.id,
                entry.kind.as_str(),
                entry.title,
                entry.body,
                entry.created_at.to_rfc3339(),
                entry.read_at.map(|d| d.to_rfc3339()),
                entry.dismissed_at.map(|d| d.to_rfc3339()),
                entry.apply_id,
                entry.game_id,
                entry.error_class,
                entry.link,
                entry.vendor,
                entry.dedup_key,
            ],
        )?;
        if inserted == 0 {
            return Ok(false);
        }
        self.evict_to_cap_with(&conn)?;
        Ok(true)
    }

    fn evict_to_cap_with(&self, conn: &rusqlite::Connection) -> Result<usize, NotificationsError> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM notifications", [], |r| r.get(0))?;
        if (count as usize) <= MAX_NOTIFICATIONS {
            return Ok(0);
        }
        let to_remove = count as usize - MAX_NOTIFICATIONS;
        let removed = conn.execute(
            "DELETE FROM notifications WHERE id IN (
                 SELECT id FROM notifications ORDER BY created_at ASC LIMIT ?1
             )",
            rusqlite::params![to_remove as i64],
        )?;
        if removed > 0 {
            tracing::warn!(
                removed,
                cap = MAX_NOTIFICATIONS,
                "notifications-store evicted oldest entries"
            );
        }
        Ok(removed)
    }

    pub fn list(&self, filter: &ListFilter) -> Result<Vec<NotificationEntry>, NotificationsError> {
        let conn = self.conn()?;
        let include_dismissed = filter.include_dismissed.unwrap_or(false);
        let limit = filter.limit.unwrap_or(DEFAULT_LIST_LIMIT) as i64;
        let sql = if include_dismissed {
            "SELECT id, kind, title, body, created_at, read_at, dismissed_at,
                    apply_id, game_id, error_class, link, vendor, dedup_key
             FROM notifications
             ORDER BY created_at DESC
             LIMIT ?1"
        } else {
            "SELECT id, kind, title, body, created_at, read_at, dismissed_at,
                    apply_id, game_id, error_class, link, vendor, dedup_key
             FROM notifications
             WHERE dismissed_at IS NULL
             ORDER BY created_at DESC
             LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| Ok(row_to_entry(row)))?;
        let mut out = Vec::new();
        for r in rows {
            let inner = match r {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, "notifications-store row read error");
                    continue;
                }
            };
            match inner {
                Ok(entry) => out.push(entry),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "notifications-store skipping unparseable row"
                    );
                }
            }
        }
        Ok(out)
    }

    pub fn mark_read(&self, id: &str) -> Result<(), NotificationsError> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE notifications SET read_at = COALESCE(read_at, ?1) WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        if affected == 0 {
            return Err(NotificationsError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn mark_all_read(&self) -> Result<u32, NotificationsError> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE notifications SET read_at = ?1 WHERE read_at IS NULL",
            rusqlite::params![now],
        )?;
        Ok(affected as u32)
    }

    pub fn dismiss(&self, id: &str) -> Result<(), NotificationsError> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE notifications SET dismissed_at = COALESCE(dismissed_at, ?1) WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        if affected == 0 {
            return Err(NotificationsError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn unread_count(&self) -> Result<u32, NotificationsError> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM notifications
             WHERE read_at IS NULL AND dismissed_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(count as u32)
    }

    pub fn total_count(&self) -> Result<u32, NotificationsError> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM notifications", [], |r| r.get(0))?;
        Ok(count as u32)
    }
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> Result<NotificationEntry, rusqlite::Error> {
    let kind_str: String = row.get(1)?;
    let created: String = row.get(4)?;
    let read: Option<String> = row.get(5)?;
    let dismissed: Option<String> = row.get(6)?;
    Ok(NotificationEntry {
        id: row.get(0)?,
        kind: NotificationKind::parse(&kind_str)?,
        title: row.get(2)?,
        body: row.get(3)?,
        created_at: parse_iso(&created)?,
        read_at: read.as_deref().map(parse_iso).transpose()?,
        dismissed_at: dismissed.as_deref().map(parse_iso).transpose()?,
        apply_id: row.get(7)?,
        game_id: row.get(8)?,
        error_class: row.get(9)?,
        link: row.get(10)?,
        vendor: row.get(11)?,
        dedup_key: row.get(12)?,
    })
}

fn parse_iso(value: &str) -> Result<chrono::DateTime<chrono::Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&chrono::Utc))
        .map_err(|_| rusqlite::Error::InvalidQuery)
}

fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn ensure_column(conn: &rusqlite::Connection, column: &str) -> Result<(), NotificationsError> {
    if !is_safe_identifier(column) {
        return Err(NotificationsError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let mut stmt = conn.prepare("PRAGMA table_info(notifications)")?;
    let existing: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !existing.iter().any(|name| name == column) {
        conn.execute_batch(&format!(
            "ALTER TABLE notifications ADD COLUMN {column} TEXT"
        ))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use tempfile::tempdir;

    fn fresh_store(dir: &tempfile::TempDir) -> NotificationsStore {
        NotificationsStore::open(dir.path().join("Cache").join("notifications.db")).unwrap()
    }

    fn make_entry(kind: NotificationKind, title: &str) -> NotificationEntry {
        NotificationEntry {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            title: title.into(),
            body: Some(format!("body for {title}")),
            created_at: chrono::Utc::now(),
            read_at: None,
            dismissed_at: None,
            apply_id: Some(format!("apply-{title}")),
            game_id: Some(format!("steam-{title}")),
            error_class: None,
            link: None,
            vendor: None,
            dedup_key: None,
        }
    }

    fn insert_with_offset(store: &NotificationsStore, title: &str, secs_ago: i64) {
        let mut entry = make_entry(NotificationKind::ApplySuccess, title);
        entry.created_at = chrono::Utc::now() - chrono::Duration::seconds(secs_ago);
        store.insert(&entry).unwrap();
    }

    #[test]
    fn dedup_key_blocks_duplicate_even_when_localized_title_differs() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let mut english = make_entry(NotificationKind::DriverUpdateAvailable, "GPU driver 610.47");
        english.dedup_key = Some("driver:RTX 4070 Ti SUPER:610.47".into());
        assert!(store.insert(&english).unwrap());

        let mut spanish = make_entry(
            NotificationKind::DriverUpdateAvailable,
            "Controlador 610.47",
        );
        spanish.dedup_key = Some("driver:RTX 4070 Ti SUPER:610.47".into());
        assert!(
            !store.insert(&spanish).unwrap(),
            "same structural identity must dedup across locales"
        );

        store.dismiss(&english.id).unwrap();
        let mut after_dismiss = make_entry(
            NotificationKind::DriverUpdateAvailable,
            "Controlador 610.47",
        );
        after_dismiss.dedup_key = Some("driver:RTX 4070 Ti SUPER:610.47".into());
        assert!(
            store.insert(&after_dismiss).unwrap(),
            "a dismissed entry no longer blocks a re-emit"
        );
    }

    #[test]
    fn open_creates_db_and_schema_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Cache").join("notifications.db");
        let _first = NotificationsStore::open(path.clone()).unwrap();
        let second = NotificationsStore::open(path.clone()).unwrap();
        assert_eq!(second.total_count().unwrap(), 0);
        assert!(path.exists());
    }

    #[test]
    fn insert_and_list_roundtrips_all_kinds() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let kinds = [
            NotificationKind::ApplySuccess,
            NotificationKind::ApplyFailure,
            NotificationKind::ApplyCancelled,
        ];
        for (i, k) in kinds.iter().enumerate() {
            let mut e = make_entry(*k, &format!("entry-{i}"));
            e.created_at = chrono::Utc::now() - chrono::Duration::seconds((kinds.len() - i) as i64);
            store.insert(&e).unwrap();
        }
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 3);
        let mut returned: Vec<NotificationKind> = listed.iter().map(|e| e.kind).collect();
        returned.sort_by_key(|k| k.as_str());
        let mut expected = kinds.to_vec();
        expected.sort_by_key(|k| k.as_str());
        assert_eq!(returned, expected);
    }

    #[test]
    fn kind_as_str_parse_symmetry_for_every_variant() {
        let all = [
            NotificationKind::ApplySuccess,
            NotificationKind::ApplyFailure,
            NotificationKind::ApplyCancelled,
            NotificationKind::AppUpdateAvailable,
            NotificationKind::CatalogUpdateAvailable,
            NotificationKind::DriverUpdateAvailable,
            NotificationKind::SystemDriverUpdateAvailable,
            NotificationKind::DllUpdatesAvailable,
            NotificationKind::BackupRestored,
            NotificationKind::ScanFailed,
            NotificationKind::CatalogRefreshFailed,
        ];
        for k in all {
            let s = k.as_str();
            let parsed = NotificationKind::parse(s).unwrap();
            assert_eq!(parsed, k, "round-trip mismatch for {s}");
        }
    }

    #[test]
    fn insert_and_list_roundtrips_signal_kinds() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let signals = [
            NotificationKind::AppUpdateAvailable,
            NotificationKind::CatalogUpdateAvailable,
            NotificationKind::ScanFailed,
            NotificationKind::CatalogRefreshFailed,
        ];
        for (i, k) in signals.iter().enumerate() {
            let mut e = make_entry(*k, &format!("signal-{i}"));
            e.created_at =
                chrono::Utc::now() - chrono::Duration::seconds((signals.len() - i) as i64);
            store.insert(&e).unwrap();
        }
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 4);
        let mut returned: Vec<NotificationKind> = listed.iter().map(|e| e.kind).collect();
        returned.sort_by_key(|k| k.as_str());
        let mut expected = signals.to_vec();
        expected.sort_by_key(|k| k.as_str());
        assert_eq!(returned, expected);
    }

    #[test]
    fn insert_and_list_preserves_link() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let mut e = make_entry(NotificationKind::AppUpdateAvailable, "release");
        e.link = Some("https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.5.0".into());
        store.insert(&e).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].link.as_deref(),
            Some("https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.5.0")
        );
    }

    #[test]
    fn insert_and_list_preserves_vendor() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let mut e = make_entry(NotificationKind::ApplySuccess, "cyberpunk");
        e.vendor = Some("nvidia".into());
        store.insert(&e).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].vendor.as_deref(), Some("nvidia"));
    }

    #[test]
    fn legacy_db_with_duplicate_rows_prunes_then_builds_dedup_index() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("Cache").join("notifications.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let legacy = rusqlite::Connection::open(&db_path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE notifications (
                    id TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT,
                    created_at TEXT NOT NULL, read_at TEXT, dismissed_at TEXT,
                    apply_id TEXT, game_id TEXT, error_class TEXT, link TEXT, vendor TEXT
                );",
            )
            .unwrap();
        let now = chrono::Utc::now();
        for i in 0..3 {
            legacy
                .execute(
                    "INSERT INTO notifications (id, kind, title, created_at)
                     VALUES (?1, 'driver_update_available', 'GPU driver 999', ?2)",
                    rusqlite::params![
                        format!("dup-{i}"),
                        (now - chrono::Duration::seconds((3 - i) as i64)).to_rfc3339()
                    ],
                )
                .unwrap();
        }
        drop(legacy);

        let store = NotificationsStore::open(db_path).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        let dups: Vec<_> = listed
            .iter()
            .filter(|e| e.title == "GPU driver 999")
            .collect();
        assert_eq!(dups.len(), 1, "pre-existing duplicates pruned to one");

        let again = make_entry(NotificationKind::DriverUpdateAvailable, "GPU driver 999");
        assert!(
            !store.insert(&again).unwrap(),
            "the dedup index must be active after prune (duplicate insert ignored)"
        );
    }

    #[test]
    fn legacy_db_without_vendor_column_migrates_and_reads_none() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("Cache").join("notifications.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let legacy = rusqlite::Connection::open(&db_path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE notifications (
                    id TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT,
                    created_at TEXT NOT NULL, read_at TEXT, dismissed_at TEXT,
                    apply_id TEXT, game_id TEXT, error_class TEXT, link TEXT
                );",
            )
            .unwrap();
        legacy
            .execute(
                "INSERT INTO notifications (id, kind, title, created_at)
                 VALUES ('legacy-1', 'apply_success', 'old', ?1)",
                rusqlite::params![chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();
        drop(legacy);

        let store = NotificationsStore::open(db_path).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "legacy-1");
        assert_eq!(listed[0].vendor, None);

        let mut fresh = make_entry(NotificationKind::DriverUpdateAvailable, "post-migration");
        fresh.vendor = Some("amd".into());
        store.insert(&fresh).unwrap();
        let after = store.list(&ListFilter::default()).unwrap();
        let migrated = after.iter().find(|e| e.id == fresh.id).unwrap();
        assert_eq!(migrated.vendor.as_deref(), Some("amd"));
    }

    #[test]
    fn legacy_db_without_link_column_migrates_and_reads_none() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("Cache").join("notifications.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let legacy = rusqlite::Connection::open(&db_path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE notifications (
                    id TEXT PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT,
                    created_at TEXT NOT NULL, read_at TEXT, dismissed_at TEXT,
                    apply_id TEXT, game_id TEXT, error_class TEXT
                );",
            )
            .unwrap();
        legacy
            .execute(
                "INSERT INTO notifications (id, kind, title, created_at)
                 VALUES ('legacy-1', 'apply_success', 'old', ?1)",
                rusqlite::params![chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();
        drop(legacy);

        let store = NotificationsStore::open(db_path).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "legacy-1");
        assert_eq!(listed[0].link, None);

        let mut fresh = make_entry(NotificationKind::DriverUpdateAvailable, "post-migration");
        fresh.link = Some("https://www.nexusmods.com/site/mods/1922".into());
        store.insert(&fresh).unwrap();
        let after = store.list(&ListFilter::default()).unwrap();
        let migrated = after.iter().find(|e| e.id == fresh.id).unwrap();
        assert_eq!(
            migrated.link.as_deref(),
            Some("https://www.nexusmods.com/site/mods/1922")
        );
    }

    #[test]
    fn list_excludes_dismissed_by_default() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let kept = make_entry(NotificationKind::ApplySuccess, "kept");
        let dismissed = make_entry(NotificationKind::ApplyFailure, "dismissed");
        store.insert(&kept).unwrap();
        store.insert(&dismissed).unwrap();
        store.dismiss(&dismissed.id).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, kept.id);
    }

    #[test]
    fn list_includes_dismissed_when_filter_set() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let kept = make_entry(NotificationKind::ApplySuccess, "kept");
        let dismissed = make_entry(NotificationKind::ApplyFailure, "dismissed");
        store.insert(&kept).unwrap();
        store.insert(&dismissed).unwrap();
        store.dismiss(&dismissed.id).unwrap();
        let listed = store
            .list(&ListFilter {
                include_dismissed: Some(true),
                limit: None,
            })
            .unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn list_respects_limit() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        for i in 0..5 {
            insert_with_offset(&store, &format!("e-{i}"), (10 - i) as i64);
        }
        let listed = store
            .list(&ListFilter {
                include_dismissed: None,
                limit: Some(2),
            })
            .unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn list_with_limit_zero_returns_empty() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        insert_with_offset(&store, "only", 1);
        let listed = store
            .list(&ListFilter {
                include_dismissed: None,
                limit: Some(0),
            })
            .unwrap();
        assert!(listed.is_empty());
    }

    #[test]
    fn mark_read_sets_read_at_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let entry = make_entry(NotificationKind::ApplySuccess, "to-read");
        store.insert(&entry).unwrap();
        store.mark_read(&entry.id).unwrap();
        let first = store.list(&ListFilter::default()).unwrap();
        let first_read_at = first[0].read_at.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        store.mark_read(&entry.id).unwrap();
        let second = store.list(&ListFilter::default()).unwrap();
        assert_eq!(second[0].read_at.unwrap(), first_read_at);
    }

    #[test]
    fn mark_read_unknown_id_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let res = store.mark_read("ghost");
        assert!(matches!(res, Err(NotificationsError::NotFound(_))));
    }

    #[test]
    fn mark_all_read_returns_count_and_clears_unread() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        for i in 0..4 {
            insert_with_offset(&store, &format!("u-{i}"), (10 - i) as i64);
        }
        assert_eq!(store.unread_count().unwrap(), 4);
        let marked = store.mark_all_read().unwrap();
        assert_eq!(marked, 4);
        assert_eq!(store.unread_count().unwrap(), 0);
        let marked_again = store.mark_all_read().unwrap();
        assert_eq!(marked_again, 0);
    }

    #[test]
    fn mark_all_read_on_empty_db_returns_zero() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        assert_eq!(store.mark_all_read().unwrap(), 0);
    }

    #[test]
    fn dismiss_excludes_from_default_list_but_keeps_row() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let entry = make_entry(NotificationKind::ApplyFailure, "to-dismiss");
        store.insert(&entry).unwrap();
        store.dismiss(&entry.id).unwrap();
        assert!(store.list(&ListFilter::default()).unwrap().is_empty());
        assert_eq!(store.total_count().unwrap(), 1);
    }

    #[test]
    fn dismiss_unknown_id_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let res = store.dismiss("ghost");
        assert!(matches!(res, Err(NotificationsError::NotFound(_))));
    }

    #[test]
    fn unread_count_excludes_read_and_dismissed() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let a = make_entry(NotificationKind::ApplySuccess, "a");
        let b = make_entry(NotificationKind::ApplyFailure, "b");
        let c = make_entry(NotificationKind::ApplyCancelled, "c");
        store.insert(&a).unwrap();
        store.insert(&b).unwrap();
        store.insert(&c).unwrap();
        store.mark_read(&a.id).unwrap();
        store.dismiss(&b.id).unwrap();
        assert_eq!(store.unread_count().unwrap(), 1);
    }

    #[test]
    fn fifo_eviction_caps_at_max_when_over_by_many() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let overflow = 50;
        let total = MAX_NOTIFICATIONS + overflow;
        for i in 0..total {
            insert_with_offset(&store, &format!("e-{i}"), (total - i) as i64);
        }
        assert_eq!(store.total_count().unwrap() as usize, MAX_NOTIFICATIONS);
        let listed = store
            .list(&ListFilter {
                include_dismissed: None,
                limit: Some(MAX_NOTIFICATIONS as u32),
            })
            .unwrap();
        assert_eq!(listed.len(), MAX_NOTIFICATIONS);
        let newest_title = &listed[0].title;
        assert_eq!(newest_title, &format!("e-{}", total - 1));
    }

    #[test]
    fn list_skips_unparseable_kind_gracefully() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let good = make_entry(NotificationKind::ApplySuccess, "good");
        store.insert(&good).unwrap();
        let conn =
            rusqlite::Connection::open(dir.path().join("Cache").join("notifications.db")).unwrap();
        conn.execute(
            "INSERT INTO notifications (id, kind, title, created_at)
             VALUES ('broken-kind', 'unknown_future_kind', 't', ?1)",
            rusqlite::params![chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();
        drop(conn);
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, good.id);
    }

    #[test]
    fn list_skips_unparseable_timestamp_gracefully() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let good = make_entry(NotificationKind::ApplySuccess, "good");
        store.insert(&good).unwrap();
        let conn =
            rusqlite::Connection::open(dir.path().join("Cache").join("notifications.db")).unwrap();
        conn.execute(
            "INSERT INTO notifications (id, kind, title, created_at)
             VALUES ('broken-ts', 'apply_success', 't', 'not-a-timestamp')",
            [],
        )
        .unwrap();
        drop(conn);
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, good.id);
    }

    #[test]
    fn future_dated_entry_preserves_desc_ordering() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let mut present = make_entry(NotificationKind::ApplySuccess, "present");
        present.created_at = chrono::Utc::now();
        let mut future = make_entry(NotificationKind::ApplySuccess, "future");
        future.created_at = chrono::Utc::now() + chrono::Duration::days(365);
        store.insert(&present).unwrap();
        store.insert(&future).unwrap();
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed[0].id, future.id);
        assert_eq!(listed[1].id, present.id);
    }

    #[test]
    fn duplicate_non_apply_kind_same_title_is_ignored() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let first = make_entry(NotificationKind::DriverUpdateAvailable, "NVIDIA 561.09");
        let second = make_entry(NotificationKind::DriverUpdateAvailable, "NVIDIA 561.09");
        assert!(store.insert(&first).unwrap());
        assert!(!store.insert(&second).unwrap());
        let listed = store.list(&ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, first.id);
    }

    #[test]
    fn duplicate_dll_digest_same_title_is_ignored() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let a = make_entry(NotificationKind::DllUpdatesAvailable, "3 updates ready");
        let b = make_entry(NotificationKind::DllUpdatesAvailable, "3 updates ready");
        assert!(store.insert(&a).unwrap());
        assert!(!store.insert(&b).unwrap());
        assert_eq!(store.total_count().unwrap(), 1);
    }

    #[test]
    fn apply_kinds_bypass_dedup_and_always_insert() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        for kind in [
            NotificationKind::ApplySuccess,
            NotificationKind::ApplyFailure,
            NotificationKind::ApplyCancelled,
        ] {
            let first = make_entry(kind, "Cyberpunk 2077");
            let second = make_entry(kind, "Cyberpunk 2077");
            assert!(store.insert(&first).unwrap());
            assert!(
                store.insert(&second).unwrap(),
                "apply kind {kind:?} must not be deduped"
            );
        }
        assert_eq!(store.total_count().unwrap(), 6);
    }

    #[test]
    fn dedup_reinserts_after_dismiss_clears_partial_index() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let first = make_entry(NotificationKind::DriverUpdateAvailable, "NVIDIA 561.09");
        assert!(store.insert(&first).unwrap());
        store.dismiss(&first.id).unwrap();
        let second = make_entry(NotificationKind::DriverUpdateAvailable, "NVIDIA 561.09");
        assert!(
            store.insert(&second).unwrap(),
            "after dismiss the partial unique index no longer covers the row"
        );
        let with_dismissed = store
            .list(&ListFilter {
                include_dismissed: Some(true),
                limit: None,
            })
            .unwrap();
        assert_eq!(with_dismissed.len(), 2);
    }

    #[test]
    fn dedup_distinguishes_by_kind_and_title() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        assert!(store
            .insert(&make_entry(NotificationKind::DriverUpdateAvailable, "A"))
            .unwrap());
        assert!(store
            .insert(&make_entry(NotificationKind::DriverUpdateAvailable, "B"))
            .unwrap());
        assert!(store
            .insert(&make_entry(NotificationKind::CatalogUpdateAvailable, "A"))
            .unwrap());
        assert_eq!(store.total_count().unwrap(), 3);
    }

    #[test]
    fn ensure_column_rejects_unsafe_identifier() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("Cache").join("notifications.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE notifications (id TEXT PRIMARY KEY);")
            .unwrap();
        let res = ensure_column(&conn, "evil TEXT; DROP TABLE notifications; --");
        assert!(matches!(
            res,
            Err(NotificationsError::Sqlite(rusqlite::Error::InvalidQuery))
        ));
        let still_there: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'notifications'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(still_there, 1);
    }

    #[test]
    fn ensure_column_accepts_safe_identifier() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let conn = store.conn().unwrap();
        assert!(ensure_column(&conn, "extra_meta").is_ok());
        assert!(ensure_column(&conn, "extra_meta").is_ok());
    }

    #[test]
    fn concurrent_inserts_serialize_without_corruption() {
        let dir = tempdir().unwrap();
        let store = Arc::new(fresh_store(&dir));
        let workers = 4;
        let per_worker = 5;
        let mut handles = Vec::new();
        for w in 0..workers {
            let store = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                for i in 0..per_worker {
                    let mut entry =
                        make_entry(NotificationKind::ApplySuccess, &format!("w{w}-i{i}"));
                    entry.created_at =
                        chrono::Utc::now() - chrono::Duration::milliseconds((w * 100 + i) as i64);
                    let mut attempts = 0;
                    loop {
                        match store.insert(&entry) {
                            Ok(_) => break,
                            Err(NotificationsError::Sqlite(rusqlite::Error::SqliteFailure(
                                err,
                                _,
                            ))) if err.code == rusqlite::ErrorCode::DatabaseBusy
                                && attempts < 5 =>
                            {
                                attempts += 1;
                                thread::sleep(std::time::Duration::from_millis(10));
                            }
                            Err(e) => panic!("insert failed: {e:?}"),
                        }
                    }
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(
            store.total_count().unwrap() as usize,
            (workers * per_worker) as usize
        );
    }
}
