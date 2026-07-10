use dlssync_contracts::{JournalFilter, OperationRecord};
use rusqlite::params;
use std::collections::BTreeMap;
use std::path::PathBuf;

const DEFAULT_LIMIT: u32 = 200;
const MAX_LIMIT: u32 = 1_000;

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid journal row: {0}")]
    InvalidRow(String),
}

#[derive(Debug, Clone)]
pub struct JournalStore {
    db_path: PathBuf,
}

impl JournalStore {
    pub fn open(db_path: PathBuf) -> Result<Self, JournalError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self { db_path };
        store.ensure_schema()?;
        Ok(store)
    }

    fn connection(&self) -> Result<rusqlite::Connection, JournalError> {
        Ok(rusqlite::Connection::open(&self.db_path)?)
    }

    fn ensure_schema(&self) -> Result<(), JournalError> {
        self.connection()?.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS operations (
                 id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL,
                 actor TEXT NOT NULL,
                 kind TEXT NOT NULL,
                 status TEXT NOT NULL,
                 target TEXT,
                 summary TEXT NOT NULL,
                 details_json TEXT NOT NULL,
                 duration_ms INTEGER,
                 backup_id TEXT,
                 error TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_operations_created
                 ON operations(created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_operations_target
                 ON operations(target, created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_operations_kind_status
                 ON operations(kind, status, created_at DESC);",
        )?;
        Ok(())
    }

    pub fn append(&self, record: &OperationRecord) -> Result<(), JournalError> {
        self.connection()?.execute(
            "INSERT INTO operations
                (id, created_at, actor, kind, status, target, summary, details_json,
                 duration_ms, backup_id, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.created_at,
                record.actor.as_str(),
                record.kind.as_str(),
                record.status.as_str(),
                record.target,
                record.summary,
                serde_json::to_string(&record.details)?,
                record.duration_ms,
                record.backup_id,
                record.error,
            ],
        )?;
        Ok(())
    }

    pub fn list(&self, filter: &JournalFilter) -> Result<Vec<OperationRecord>, JournalError> {
        let limit = filter.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, actor, kind, status, target, summary, details_json,
                    duration_ms, backup_id, error
             FROM operations ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([MAX_LIMIT], row_to_record)?;
        let mut records = Vec::new();
        for row in rows {
            let record = row?;
            if filter
                .target
                .as_ref()
                .is_some_and(|target| record.target.as_deref() != Some(target.as_str()))
            {
                continue;
            }
            if filter.kind.is_some_and(|kind| record.kind != kind) {
                continue;
            }
            if filter.status.is_some_and(|status| record.status != status) {
                continue;
            }
            records.push(record);
            if records.len() == limit {
                break;
            }
        }
        Ok(records)
    }

    pub fn export_redacted_json(&self, filter: &JournalFilter) -> Result<String, JournalError> {
        let mut records = self.list(filter)?;
        for record in &mut records {
            record.target = record
                .target
                .as_ref()
                .map(|_| "[redacted-target]".to_string());
            for (key, value) in &mut record.details {
                if is_sensitive_key(key) || looks_like_local_path(value) {
                    *value = "[redacted]".to_string();
                }
            }
            if looks_like_local_path(&record.summary) {
                record.summary = "[redacted-summary-with-local-path]".to_string();
            }
            if record
                .error
                .as_ref()
                .is_some_and(|value| looks_like_local_path(value))
            {
                record.error = Some("[redacted-error-with-local-path]".into());
            }
        }
        Ok(serde_json::to_string_pretty(&records)?)
    }

    pub fn prune_unlinked(&self, keep: u32) -> Result<usize, JournalError> {
        let changed = self.connection()?.execute(
            "DELETE FROM operations
             WHERE backup_id IS NULL
               AND id IN (
                 SELECT id FROM operations
                 WHERE backup_id IS NULL
                 ORDER BY created_at DESC
                 LIMIT -1 OFFSET ?1
               )",
            [keep],
        )?;
        Ok(changed)
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRecord> {
    let actor: String = row.get(2)?;
    let kind: String = row.get(3)?;
    let status: String = row.get(4)?;
    let details_json: String = row.get(7)?;
    Ok(OperationRecord {
        id: row.get(0)?,
        created_at: row.get(1)?,
        actor: parse_enum(&actor).map_err(to_sql_error)?,
        kind: parse_enum(&kind).map_err(to_sql_error)?,
        status: parse_enum(&status).map_err(to_sql_error)?,
        target: row.get(5)?,
        summary: row.get(6)?,
        details: serde_json::from_str::<BTreeMap<String, String>>(&details_json)
            .map_err(to_sql_error)?,
        duration_ms: row.get(8)?,
        backup_id: row.get(9)?,
        error: row.get(10)?,
    })
}

fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(&serde_json::to_string(value)?)
}

fn to_sql_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("path") || key.contains("token") || key.contains("secret") || key.contains("key")
}

/// Value-based detector for local filesystem paths. Conservative by design: it aims
/// to match real local paths (Windows drive/UNC/env-var, home-tilde, and POSIX
/// absolute user paths) while leaving ordinary prose and web URLs intact.
fn looks_like_local_path(value: &str) -> bool {
    // Never treat web URLs as local paths.
    let trimmed = value.trim_start();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return false;
    }

    let bytes = value.as_bytes();

    // UNC paths, e.g. \\server\share
    if value.contains("\\\\") {
        return true;
    }

    // Windows drive paths anywhere in the value, e.g. C:\ or D:/ — but never a URL
    // scheme like `x://`. The letter must stand alone as a drive letter.
    for i in 1..bytes.len() {
        if bytes[i] != b':' {
            continue;
        }
        if !bytes[i - 1].is_ascii_alphabetic() {
            continue;
        }
        let boundary = i == 1 || !bytes[i - 2].is_ascii_alphanumeric();
        if !boundary {
            continue;
        }
        match bytes.get(i + 1) {
            Some(b'\\') => return true,
            // `C:/path` is a drive path, but `://` (a URL scheme) is not.
            Some(b'/') if bytes.get(i + 2) != Some(&b'/') => return true,
            _ => {}
        }
    }

    // Windows environment-variable directories.
    if value.contains("%APPDATA%")
        || value.contains("%LOCALAPPDATA%")
        || value.contains("%USERPROFILE%")
        || contains_env_var_dir(value)
    {
        return true;
    }

    // Home-relative path, e.g. ~/Library/...
    if value.contains("~/") {
        return true;
    }

    // Well-known POSIX user/system directories.
    if value.contains("/home/")
        || value.contains("/Users/")
        || value.contains("/mnt/")
        || value.contains("/root/")
    {
        return true;
    }

    // A leading absolute POSIX path with at least two segments, e.g. /etc/hosts.
    if let Some(rest) = value.strip_prefix('/') {
        if let Some(idx) = rest.find('/') {
            if idx > 0 {
                return true;
            }
        }
    }

    false
}

/// Matches a generic Windows env-var expansion followed by a path separator, e.g. `%VAR%\`.
fn contains_env_var_dir(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut open: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'%' {
            continue;
        }
        match open {
            // A closing `%` with a non-empty name in between, followed by `\`.
            Some(start) if i > start + 1 => {
                if bytes.get(i + 1) == Some(&b'\\') {
                    return true;
                }
                open = Some(i);
            }
            _ => open = Some(i),
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlssync_contracts::{OperationActor, OperationKind, OperationStatus};
    use tempfile::tempdir;

    fn record(id: &str, backup_id: Option<&str>) -> OperationRecord {
        OperationRecord {
            id: id.into(),
            created_at: format!("2026-07-10T00:00:0{id}Z"),
            actor: OperationActor::Gui,
            kind: OperationKind::DllApply,
            status: OperationStatus::Succeeded,
            target: Some("C:\\Games\\Private\\game.dll".into()),
            summary: "Updated DLSS".into(),
            details: BTreeMap::from([
                ("source_url".into(), "https://vendor.example/file".into()),
                ("backup_path".into(), "C:\\Users\\Tony\\Backup".into()),
            ]),
            duration_ms: Some(42),
            backup_id: backup_id.map(str::to_string),
            error: None,
        }
    }

    #[test]
    fn append_list_filter_and_redacted_export_round_trip() {
        let dir = tempdir().unwrap();
        let store = JournalStore::open(dir.path().join("journal.db")).unwrap();
        store.append(&record("1", Some("backup-1"))).unwrap();
        let rows = store.list(&JournalFilter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].backup_id.as_deref(), Some("backup-1"));
        let export = store
            .export_redacted_json(&JournalFilter::default())
            .unwrap();
        assert!(export.contains("[redacted-target]"));
        assert!(export.contains("[redacted]"));
        assert!(!export.contains("Tony"));
    }

    fn record_with(
        id: &str,
        summary: &str,
        error: Option<&str>,
        details: BTreeMap<String, String>,
    ) -> OperationRecord {
        OperationRecord {
            id: id.into(),
            created_at: format!("2026-07-10T00:00:0{id}Z"),
            actor: OperationActor::Gui,
            kind: OperationKind::DllApply,
            status: OperationStatus::Succeeded,
            target: None,
            summary: summary.into(),
            details,
            duration_ms: Some(1),
            backup_id: None,
            error: error.map(str::to_string),
        }
    }

    fn export_one(record: OperationRecord) -> String {
        let dir = tempdir().unwrap();
        let store = JournalStore::open(dir.path().join("journal.db")).unwrap();
        store.append(&record).unwrap();
        store
            .export_redacted_json(&JournalFilter::default())
            .unwrap()
    }

    #[test]
    fn redacts_windows_path_detail_under_non_sensitive_key() {
        let details =
            BTreeMap::from([("note".into(), "C:\\Users\\bob\\game\\nvngx_dlss.dll".into())]);
        let export = export_one(record_with("1", "ok", None, details));
        assert!(export.contains("[redacted]"));
        assert!(!export.contains("nvngx_dlss.dll"));
        assert!(!export.contains("bob"));
        // The key name itself is preserved — only the value is redacted.
        assert!(export.contains("\"note\""));
    }

    #[test]
    fn redacts_posix_path_detail_under_non_sensitive_key() {
        let details = BTreeMap::from([("game".into(), "/home/bob/games/x/libxess.dll".into())]);
        let export = export_one(record_with("1", "ok", None, details));
        assert!(export.contains("[redacted]"));
        assert!(!export.contains("libxess.dll"));
        assert!(!export.contains("/home/bob"));
    }

    #[test]
    fn redacts_error_containing_posix_path() {
        let export = export_one(record_with(
            "1",
            "ok",
            Some("failed to write /Users/bob/Game/dll"),
            BTreeMap::new(),
        ));
        assert!(export.contains("[redacted-error-with-local-path]"));
        assert!(!export.contains("/Users/bob"));
    }

    #[test]
    fn redacts_summary_containing_local_path() {
        let export = export_one(record_with(
            "1",
            "Copied to C:\\Users\\bob\\game",
            None,
            BTreeMap::new(),
        ));
        assert!(export.contains("[redacted-summary-with-local-path]"));
        assert!(!export.contains("Copied to C"));
        assert!(!export.contains("bob"));
    }

    #[test]
    fn preserves_normal_summary_and_detail_values() {
        let details = BTreeMap::from([("count".into(), "4".into())]);
        let export = export_one(record_with("1", "Library scan completed", None, details));
        assert!(export.contains("Library scan completed"));
        assert!(!export.contains("[redacted-summary-with-local-path]"));
        assert!(export.contains("\"4\""));
        assert!(!export.contains("[redacted]"));
    }

    #[test]
    fn still_redacts_sensitive_key_values() {
        let details = BTreeMap::from([("api_key".into(), "secret-value-123".into())]);
        let export = export_one(record_with("1", "ok", None, details));
        assert!(export.contains("[redacted]"));
        assert!(!export.contains("secret-value-123"));
    }

    #[test]
    fn prune_keeps_backup_linked_operations() {
        let dir = tempdir().unwrap();
        let store = JournalStore::open(dir.path().join("journal.db")).unwrap();
        store.append(&record("1", None)).unwrap();
        store.append(&record("2", None)).unwrap();
        store.append(&record("3", Some("backup-3"))).unwrap();
        assert_eq!(store.prune_unlinked(1).unwrap(), 1);
        let rows = store
            .list(&JournalFilter {
                limit: Some(10),
                ..JournalFilter::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows
            .iter()
            .any(|row| row.backup_id.as_deref() == Some("backup-3")));
    }
}
