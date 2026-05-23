use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub id: String,
    pub game_id: String,
    pub dll_family: String,
    pub dll_filename: String,
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub previous_version: Option<String>,
    pub previous_sha256: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub restored_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

pub struct BackupStore {
    db_path: PathBuf,
    pub root_dir: PathBuf,
}

impl BackupStore {
    pub fn open(db_path: PathBuf, root_dir: PathBuf) -> Result<Self, BackupError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(&root_dir)?;
        let store = Self { db_path, root_dir };
        store.ensure_schema()?;
        Ok(store)
    }

    fn conn(&self) -> Result<rusqlite::Connection, BackupError> {
        Ok(rusqlite::Connection::open(&self.db_path)?)
    }

    fn ensure_schema(&self) -> Result<(), BackupError> {
        let conn = self.conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS backups (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL,
                dll_family TEXT NOT NULL,
                dll_filename TEXT NOT NULL,
                original_path TEXT NOT NULL,
                backup_path TEXT NOT NULL,
                previous_version TEXT,
                previous_sha256 TEXT,
                created_at TEXT NOT NULL,
                restored_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_backups_game ON backups(game_id);
            CREATE INDEX IF NOT EXISTS idx_backups_created ON backups(created_at DESC);",
        )?;
        Ok(())
    }

    pub fn allocate_backup_path(
        &self,
        game_label: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
        filename: &str,
    ) -> Result<PathBuf, BackupError> {
        let game_dir = self.root_dir.join(sanitize_folder_name(game_label));
        let stamp = timestamp.format("%Y-%m-%d %H-%M-%S").to_string();
        let dir = game_dir.join(stamp);
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join(filename))
    }

    pub fn insert(&self, entry: &BackupEntry) -> Result<(), BackupError> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO backups
                (id, game_id, dll_family, dll_filename, original_path, backup_path,
                 previous_version, previous_sha256, created_at, restored_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                entry.id,
                entry.game_id,
                entry.dll_family,
                entry.dll_filename,
                path_str(&entry.original_path),
                path_str(&entry.backup_path),
                entry.previous_version,
                entry.previous_sha256,
                entry.created_at.to_rfc3339(),
                entry.restored_at.map(|d| d.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<BackupEntry, BackupError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, game_id, dll_family, dll_filename, original_path, backup_path,
                    previous_version, previous_sha256, created_at, restored_at
             FROM backups WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        let row = rows
            .next()?
            .ok_or_else(|| BackupError::NotFound(id.to_string()))?;
        let mut entry = row_to_entry(row)?;
        entry.size_bytes = std::fs::metadata(&entry.backup_path).ok().map(|m| m.len());
        Ok(entry)
    }

    pub fn mark_restored(
        &self,
        id: &str,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), BackupError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE backups SET restored_at = ?1 WHERE id = ?2",
            rusqlite::params![at.to_rfc3339(), id],
        )?;
        if affected == 0 {
            return Err(BackupError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<BackupEntry>, BackupError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, game_id, dll_family, dll_filename, original_path, backup_path,
                    previous_version, previous_sha256, created_at, restored_at
             FROM backups ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| Ok(row_to_entry(row)))?;
        let mut out = Vec::new();
        for r in rows {
            let mut entry = r??;
            entry.size_bytes = std::fs::metadata(&entry.backup_path).ok().map(|m| m.len());
            out.push(entry);
        }
        Ok(out)
    }

    pub fn delete(&self, id: &str, remove_file: bool) -> Result<DeleteOutcome, BackupError> {
        let entry = self.get(id)?;
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM backups WHERE id = ?1", rusqlite::params![id])?;
        if affected == 0 {
            return Err(BackupError::NotFound(id.to_string()));
        }
        let mut outcome = DeleteOutcome {
            removed_file: false,
            removed_empty_dirs: 0,
            file_error: None,
        };
        if remove_file && entry.backup_path.exists() {
            match std::fs::remove_file(&entry.backup_path) {
                Ok(()) => {
                    outcome.removed_file = true;
                    outcome.removed_empty_dirs =
                        prune_empty_parents(entry.backup_path.parent(), &self.root_dir);
                }
                Err(e) => {
                    outcome.file_error = Some(e.to_string());
                }
            }
        }
        Ok(outcome)
    }

    pub fn rewrite_path_prefix(
        db_path: &Path,
        old_prefix: &str,
        new_prefix: &str,
    ) -> Result<usize, BackupError> {
        if !db_path.exists() {
            return Ok(0);
        }
        let conn = rusqlite::Connection::open(db_path)?;
        let mut tables: Vec<String> = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='backups'")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                tables.push(row.get(0)?);
            }
        }
        if tables.is_empty() {
            return Ok(0);
        }
        let like_pattern = format!("{old_prefix}%");
        let updated = conn.execute(
            "UPDATE backups
             SET backup_path = ?1 || substr(backup_path, length(?2) + 1)
             WHERE backup_path LIKE ?3",
            rusqlite::params![new_prefix, old_prefix, like_pattern],
        )?;
        Ok(updated)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeleteOutcome {
    pub removed_file: bool,
    pub removed_empty_dirs: usize,
    pub file_error: Option<String>,
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> Result<BackupEntry, rusqlite::Error> {
    let created: String = row.get(8)?;
    let restored: Option<String> = row.get(9)?;
    Ok(BackupEntry {
        id: row.get(0)?,
        game_id: row.get(1)?,
        dll_family: row.get(2)?,
        dll_filename: row.get(3)?,
        original_path: PathBuf::from(row.get::<_, String>(4)?),
        backup_path: PathBuf::from(row.get::<_, String>(5)?),
        previous_version: row.get(6)?,
        previous_sha256: row.get(7)?,
        created_at: parse_iso(&created)?,
        restored_at: restored.as_deref().map(parse_iso).transpose()?,
        size_bytes: None,
    })
}

fn parse_iso(s: &str) -> Result<chrono::DateTime<chrono::Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .map_err(|_| rusqlite::Error::InvalidQuery)
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn sanitize_folder_name(raw: &str) -> String {
    const FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let trimmed = raw.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if (ch as u32) < 0x20 || FORBIDDEN.contains(&ch) {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    while let Some(last) = out.chars().last() {
        if last == '.' || last == ' ' {
            out.pop();
        } else {
            break;
        }
    }
    if out.is_empty() {
        out.push_str("unnamed");
    }
    let mut truncated = String::with_capacity(out.len().min(80));
    for (i, ch) in out.chars().enumerate() {
        if i >= 80 {
            break;
        }
        truncated.push(ch);
    }
    while let Some(last) = truncated.chars().last() {
        if last == '.' || last == ' ' {
            truncated.pop();
        } else {
            break;
        }
    }
    if truncated.is_empty() {
        truncated.push_str("unnamed");
    }
    truncated
}

fn prune_empty_parents(start: Option<&Path>, root: &Path) -> usize {
    let mut current = match start {
        Some(p) => p.to_path_buf(),
        None => return 0,
    };
    let mut count = 0usize;
    while current.starts_with(root) && current != root {
        let is_empty = std::fs::read_dir(&current)
            .map(|mut it| it.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            break;
        }
        if std::fs::remove_dir(&current).is_err() {
            break;
        }
        count += 1;
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => break,
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh_store(dir: &tempfile::TempDir) -> BackupStore {
        let db = dir.path().join("Backups").join("backups.db");
        let root = dir.path().join("Backups");
        BackupStore::open(db, root).unwrap()
    }

    fn fake_entry(store: &BackupStore, game_label: &str, filename: &str) -> BackupEntry {
        let stamp = chrono::Utc::now();
        let bp = store
            .allocate_backup_path(game_label, stamp, filename)
            .unwrap();
        std::fs::write(&bp, b"hello dll").unwrap();
        BackupEntry {
            id: uuid::Uuid::new_v4().to_string(),
            game_id: format!("steam-{game_label}"),
            dll_family: "dlss_sr".into(),
            dll_filename: filename.into(),
            original_path: PathBuf::from(format!("C:\\Games\\{game_label}\\{filename}")),
            backup_path: bp,
            previous_version: Some("v3.0-test".into()),
            previous_sha256: Some("deadbeef".into()),
            created_at: stamp,
            restored_at: None,
            size_bytes: None,
        }
    }

    #[test]
    fn sanitize_folder_name_keeps_intuitive_chars() {
        assert_eq!(
            sanitize_folder_name("Steam - Cyberpunk 2077"),
            "Steam - Cyberpunk 2077"
        );
        assert_eq!(
            sanitize_folder_name("Tony Hawk's Pro Skater 1+2"),
            "Tony Hawk's Pro Skater 1+2"
        );
        assert_eq!(sanitize_folder_name("Yakuza 0"), "Yakuza 0");
    }

    #[test]
    fn sanitize_folder_name_replaces_forbidden_windows_chars() {
        assert_eq!(sanitize_folder_name("foo:bar"), "foo_bar");
        assert_eq!(sanitize_folder_name("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_folder_name("<game>"), "_game_");
        assert_eq!(sanitize_folder_name("a\"b|c?d*"), "a_b_c_d_");
    }

    #[test]
    fn sanitize_folder_name_strips_trailing_dot_and_space() {
        assert_eq!(sanitize_folder_name("Game...   "), "Game");
        assert_eq!(sanitize_folder_name("  Game.  "), "Game");
    }

    #[test]
    fn sanitize_folder_name_truncates_to_80_chars() {
        let long = "a".repeat(120);
        let out = sanitize_folder_name(&long);
        assert_eq!(out.len(), 80);
    }

    #[test]
    fn sanitize_folder_name_empty_returns_unnamed() {
        assert_eq!(sanitize_folder_name(""), "unnamed");
        assert_eq!(sanitize_folder_name("   "), "unnamed");
        assert_eq!(sanitize_folder_name("..."), "unnamed");
    }

    #[test]
    fn open_creates_separate_db_and_root_dirs() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("Backups").join("backups.db");
        let root = dir.path().join("Backups");
        let store = BackupStore::open(db.clone(), root.clone()).unwrap();
        assert!(root.is_dir());
        assert_eq!(store.root_dir, root);
    }

    #[test]
    fn insert_and_list_returns_size_bytes() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let entry = fake_entry(&store, "Steam - Demo Game", "nvngx_dlss.dll");
        store.insert(&entry).unwrap();
        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].size_bytes, Some(b"hello dll".len() as u64));
    }

    #[test]
    fn delete_removes_row_only_when_remove_file_false() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let entry = fake_entry(&store, "Steam - X", "nvngx_dlss.dll");
        let bp = entry.backup_path.clone();
        store.insert(&entry).unwrap();
        let outcome = store.delete(&entry.id, false).unwrap();
        assert!(!outcome.removed_file);
        assert!(bp.exists());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn delete_with_remove_file_removes_file_and_empty_dirs() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let entry = fake_entry(&store, "Steam - Y", "nvngx_dlss.dll");
        let bp = entry.backup_path.clone();
        let stamp_dir = bp.parent().unwrap().to_path_buf();
        let game_dir = stamp_dir.parent().unwrap().to_path_buf();
        store.insert(&entry).unwrap();
        let outcome = store.delete(&entry.id, true).unwrap();
        assert!(outcome.removed_file);
        assert!(!bp.exists());
        assert!(!stamp_dir.exists());
        assert!(!game_dir.exists());
        assert!(store.root_dir.exists());
    }

    #[test]
    fn delete_unknown_id_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let res = store.delete("missing", true);
        assert!(matches!(res, Err(BackupError::NotFound(_))));
    }

    #[test]
    fn allocate_backup_path_uses_pretty_layout() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let stamp = chrono::DateTime::parse_from_rfc3339("2026-05-21T19:07:27Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let p = store
            .allocate_backup_path("Steam - Cyberpunk 2077", stamp, "libxess_fg.dll")
            .unwrap();
        let suffix = p
            .strip_prefix(&store.root_dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(
            suffix,
            "Steam - Cyberpunk 2077/2026-05-21 19-07-27/libxess_fg.dll"
        );
    }

    #[test]
    fn rewrite_path_prefix_swaps_only_matching_rows() {
        let dir = tempdir().unwrap();
        let store = fresh_store(&dir);
        let entry_a = fake_entry(&store, "Steam - A", "a.dll");
        store.insert(&entry_a).unwrap();
        let conn =
            rusqlite::Connection::open(dir.path().join("Backups").join("backups.db")).unwrap();
        conn.execute(
            "INSERT INTO backups VALUES ('legacy', 'steam-1', 'dlss_sr', 'b.dll',
              'C:\\Games\\Foo\\b.dll',
              'C:\\old\\backups\\steam-1\\stamp\\b.dll',
              '1.0', 'abc', '2026-01-01T00:00:00Z', NULL)",
            [],
        )
        .unwrap();
        drop(conn);
        let n = BackupStore::rewrite_path_prefix(
            &dir.path().join("Backups").join("backups.db"),
            "C:\\old\\backups",
            "D:\\new\\Backups",
        )
        .unwrap();
        assert_eq!(n, 1);
        let conn =
            rusqlite::Connection::open(dir.path().join("Backups").join("backups.db")).unwrap();
        let val: String = conn
            .query_row(
                "SELECT backup_path FROM backups WHERE id = 'legacy'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "D:\\new\\Backups\\steam-1\\stamp\\b.dll");
    }

    #[test]
    fn rewrite_path_prefix_returns_zero_for_missing_db() {
        let dir = tempdir().unwrap();
        let n =
            BackupStore::rewrite_path_prefix(&dir.path().join("ghost.db"), "C:\\old", "D:\\new")
                .unwrap();
        assert_eq!(n, 0);
    }
}
