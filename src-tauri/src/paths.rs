use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub backups_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub settings_dir: PathBuf,
    pub backups_db: PathBuf,
    pub notifications_db: PathBuf,
    pub catalog_cache: PathBuf,
    pub settings_file: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    pub legacy_root: Option<PathBuf>,
    pub moved_files: usize,
    pub rewrote_db_rows: usize,
    pub copied_settings: bool,
    pub copied_catalog: bool,
    pub errors: Vec<String>,
}

/// Debug-only data-root override so the e2e harness can point the app at a
/// throwaway directory instead of mutating the host's real `~/DLSSync` state.
/// Release builds ignore it entirely.
#[cfg(debug_assertions)]
fn data_dir_override() -> Option<PathBuf> {
    std::env::var_os("DLSSYNC_DATA_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

impl AppPaths {
    pub fn resolve(handle: &AppHandle) -> Result<Self, String> {
        #[cfg(debug_assertions)]
        if let Some(root) = data_dir_override() {
            return Ok(Self::from_root(root));
        }
        let home = handle
            .path()
            .home_dir()
            .map_err(|e| format!("home_dir: {e}"))?;
        Ok(Self::from_root(home.join(root_subdir())))
    }

    pub fn from_root(root: PathBuf) -> Self {
        let backups_dir = root.join("Backups");
        let cache_dir = root.join("Cache");
        let logs_dir = root.join("Logs");
        let settings_dir = root.join("Settings");
        Self {
            backups_db: backups_dir.join("backups.db"),
            notifications_db: cache_dir.join("notifications.db"),
            catalog_cache: cache_dir.join("catalog.json"),
            settings_file: settings_dir.join("settings.json"),
            root,
            backups_dir,
            cache_dir,
            logs_dir,
            settings_dir,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for d in [
            &self.root,
            &self.backups_dir,
            &self.cache_dir,
            &self.logs_dir,
            &self.settings_dir,
        ] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }

    pub fn migrate_legacy(&self, handle: &AppHandle) -> MigrationReport {
        let mut report = MigrationReport::default();
        #[cfg(debug_assertions)]
        if data_dir_override().is_some() {
            return report;
        }
        let legacy_root = match handle.path().app_config_dir() {
            Ok(p) => p,
            Err(e) => {
                report.errors.push(format!("legacy app_config_dir: {e}"));
                return report;
            }
        };
        self.migrate_from(&legacy_root, &mut report);
        report
    }

    pub fn migrate_from(&self, legacy_root: &Path, report: &mut MigrationReport) {
        if !legacy_root.exists() || legacy_root == self.root {
            return;
        }
        report.legacy_root = Some(legacy_root.to_path_buf());

        let legacy_backups = legacy_root.join("backups");
        let legacy_db = legacy_root.join("backups.db");
        let legacy_settings = legacy_root.join("settings.json");
        let legacy_catalog = legacy_root.join("catalog.json");

        if legacy_backups.exists() {
            match move_tree(&legacy_backups, &self.backups_dir) {
                Ok(n) => report.moved_files = n,
                Err(e) => report.errors.push(format!("move backups: {e}")),
            }
        }

        if legacy_db.exists() && !self.backups_db.exists() {
            if let Some(parent) = self.backups_db.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = move_or_copy_file(&legacy_db, &self.backups_db) {
                report.errors.push(format!("move db: {e}"));
            }
        }

        if self.backups_db.exists() {
            let old_str = legacy_backups.to_string_lossy().to_string();
            let new_str = self.backups_dir.to_string_lossy().to_string();
            match backup_store::BackupStore::rewrite_path_prefix(
                &self.backups_db,
                &old_str,
                &new_str,
            ) {
                Ok(n) => report.rewrote_db_rows = n,
                Err(e) => report.errors.push(format!("rewrite db: {e}")),
            }
        }

        if legacy_settings.exists() && !self.settings_file.exists() {
            match move_or_copy_file(&legacy_settings, &self.settings_file) {
                Ok(()) => report.copied_settings = true,
                Err(e) => report.errors.push(format!("settings copy: {e}")),
            }
        }

        if legacy_catalog.exists() && !self.catalog_cache.exists() {
            match move_or_copy_file(&legacy_catalog, &self.catalog_cache) {
                Ok(()) => report.copied_catalog = true,
                Err(e) => report.errors.push(format!("catalog copy: {e}")),
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathGuardError {
    #[error("path escapes the permitted root: {0}")]
    OutsideRoot(String),
    #[error("path is not a .dll file: {0}")]
    NotDll(String),
    #[error("refusing to operate on a protected system location: {0}")]
    SystemDir(String),
    #[error("refusing to follow a symlink: {0}")]
    Symlink(String),
    #[error("refusing to scan an unsafe or too-shallow path: {0}")]
    UnsafeScanDir(String),
}

/// Containment guards shared by every restore/rollback/scan write path. The
/// backup database stores `backup_path`/`original_path` as opaque strings, so a
/// tampered row is an untrusted file pointer until these checks pass. Centralised
/// here so `restore_backup`, `rollback_all`, and apply-rollback all enforce the
/// same invariant instead of each re-deriving it.
pub struct PathGuard;

impl PathGuard {
    /// The path must live inside `root`. `Path::starts_with` is component-wise,
    /// so `/data/BackupsEvil` does not match a `/data/Backups` root.
    pub fn assert_under_root(path: &Path, root: &Path) -> Result<(), PathGuardError> {
        if path.starts_with(root) {
            Ok(())
        } else {
            Err(PathGuardError::OutsideRoot(path.display().to_string()))
        }
    }

    /// Restore targets are always game DLLs; reject anything else so a tampered
    /// `original_path` cannot be aimed at an `.exe`, startup-folder shortcut, etc.
    pub fn assert_dll_ext(path: &Path) -> Result<(), PathGuardError> {
        let is_dll = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("dll"));
        if is_dll {
            Ok(())
        } else {
            Err(PathGuardError::NotDll(path.display().to_string()))
        }
    }

    /// Reject writes anywhere under the Windows directory (System32/SysWOW64 and
    /// friends). Game libraries legitimately live under Program Files, so only the
    /// OS tree is denied.
    pub fn deny_system_dir(path: &Path) -> Result<(), PathGuardError> {
        let lower = path
            .to_string_lossy()
            .to_ascii_lowercase()
            .replace('/', "\\");
        let windir = std::env::var("WINDIR")
            .or_else(|_| std::env::var("SystemRoot"))
            .unwrap_or_else(|_| "C:\\Windows".to_string())
            .to_ascii_lowercase()
            .replace('/', "\\");
        let under_windir = lower.starts_with(&format!("{windir}\\")) || lower == windir;
        let under_sys =
            lower.contains("\\windows\\system32\\") || lower.contains("\\windows\\syswow64\\");
        if under_windir || under_sys {
            Err(PathGuardError::SystemDir(path.display().to_string()))
        } else {
            Ok(())
        }
    }

    /// A scan target supplied by the webview must be a real game directory, not a
    /// bare drive root or a system location — otherwise a crafted `install_dir`
    /// turns the recursive scanners into an arbitrary filesystem-enumeration / DoS
    /// lever. Requires at least one normal path component and rejects the OS tree.
    pub fn assert_safe_scan_dir(path: &Path) -> Result<(), PathGuardError> {
        Self::deny_system_dir(path)
            .map_err(|_| PathGuardError::UnsafeScanDir(path.display().to_string()))?;
        let normal_components = path
            .components()
            .filter(|c| matches!(c, std::path::Component::Normal(_)))
            .count();
        if normal_components < 1 {
            return Err(PathGuardError::UnsafeScanDir(path.display().to_string()));
        }
        Ok(())
    }

    /// Refuse to write through a symlink/junction: a local attacker could plant
    /// one at a previously-backed-up DLL path to redirect the write elsewhere.
    pub fn assert_not_symlink(path: &Path) -> Result<(), PathGuardError> {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                Err(PathGuardError::Symlink(path.display().to_string()))
            }
            _ => Ok(()),
        }
    }
}

fn root_subdir() -> &'static str {
    if cfg!(target_os = "windows") {
        "DLSSync"
    } else {
        ".dlssync"
    }
}

pub fn default_root() -> Option<PathBuf> {
    let home = if cfg!(target_os = "windows") {
        std::env::var_os("USERPROFILE")
    } else {
        std::env::var_os("HOME")
    }?;
    Some(PathBuf::from(home).join(root_subdir()))
}

fn move_tree(src: &Path, dst: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(dst)?;
    let mut count = 0;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            count += move_tree(&from, &to)?;
            let _ = std::fs::remove_dir(&from);
        } else {
            if to.exists() {
                continue;
            }
            move_or_copy_file(&from, &to)?;
            count += 1;
        }
    }
    Ok(count)
}

fn move_or_copy_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::fs::copy(src, dst)?;
            let _ = std::fs::remove_file(src);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn guard_under_root_is_component_wise() {
        let root = Path::new("/data/DLSSync/Backups");
        assert!(PathGuard::assert_under_root(
            Path::new("/data/DLSSync/Backups/Cyberpunk/sl.dll"),
            root
        )
        .is_ok());
        assert!(PathGuard::assert_under_root(Path::new("/etc/passwd"), root).is_err());
        assert!(
            PathGuard::assert_under_root(Path::new("/data/DLSSync/BackupsEvil/x.dll"), root)
                .is_err()
        );
    }

    #[test]
    fn guard_dll_ext_only() {
        assert!(PathGuard::assert_dll_ext(Path::new("C:/g/nvngx_dlss.DLL")).is_ok());
        assert!(PathGuard::assert_dll_ext(Path::new("C:/g/evil.exe")).is_err());
        assert!(PathGuard::assert_dll_ext(Path::new("C:/g/nodll")).is_err());
    }

    #[test]
    fn guard_denies_system_dirs_but_allows_program_files() {
        assert!(PathGuard::deny_system_dir(Path::new("C:\\Windows\\System32\\ntdll.dll")).is_err());
        assert!(PathGuard::deny_system_dir(Path::new("C:\\Windows\\SysWOW64\\x.dll")).is_err());
        assert!(PathGuard::deny_system_dir(Path::new(
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Game\\nvngx_dlss.dll"
        ))
        .is_ok());
    }

    #[test]
    fn guard_not_symlink_passes_for_missing_and_regular_files() {
        let dir = tempdir().unwrap();
        let regular = dir.path().join("real.dll");
        std::fs::write(&regular, b"x").unwrap();
        assert!(PathGuard::assert_not_symlink(&regular).is_ok());
        assert!(PathGuard::assert_not_symlink(&dir.path().join("missing.dll")).is_ok());
    }

    #[test]
    fn from_root_lays_out_subdirs() {
        let dir = tempdir().unwrap();
        let p = AppPaths::from_root(dir.path().join("DLSSync"));
        assert_eq!(p.backups_dir, dir.path().join("DLSSync").join("Backups"));
        assert_eq!(p.cache_dir, dir.path().join("DLSSync").join("Cache"));
        assert_eq!(p.logs_dir, dir.path().join("DLSSync").join("Logs"));
        assert_eq!(p.settings_dir, dir.path().join("DLSSync").join("Settings"));
        assert_eq!(
            p.backups_db,
            dir.path()
                .join("DLSSync")
                .join("Backups")
                .join("backups.db")
        );
        assert_eq!(
            p.settings_file,
            dir.path()
                .join("DLSSync")
                .join("Settings")
                .join("settings.json")
        );
        assert_eq!(
            p.catalog_cache,
            dir.path()
                .join("DLSSync")
                .join("Cache")
                .join("catalog.json")
        );
        assert_eq!(
            p.notifications_db,
            dir.path()
                .join("DLSSync")
                .join("Cache")
                .join("notifications.db")
        );
    }

    #[test]
    fn ensure_dirs_idempotent() {
        let dir = tempdir().unwrap();
        let p = AppPaths::from_root(dir.path().join("DLSSync"));
        p.ensure_dirs().unwrap();
        p.ensure_dirs().unwrap();
        assert!(p.backups_dir.is_dir());
        assert!(p.cache_dir.is_dir());
        assert!(p.logs_dir.is_dir());
        assert!(p.settings_dir.is_dir());
    }

    #[test]
    fn move_tree_moves_nested_files() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::create_dir_all(src.join("a").join("b")).unwrap();
        std::fs::write(src.join("a").join("b").join("x.dll"), b"x").unwrap();
        std::fs::write(src.join("y.dll"), b"y").unwrap();
        let n = move_tree(&src, &dst).unwrap();
        assert_eq!(n, 2);
        assert!(dst.join("a").join("b").join("x.dll").exists());
        assert!(dst.join("y.dll").exists());
    }

    #[test]
    fn move_tree_skips_existing_destinations() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("k.dll"), b"new").unwrap();
        std::fs::write(dst.join("k.dll"), b"existing").unwrap();
        move_tree(&src, &dst).unwrap();
        let body = std::fs::read(dst.join("k.dll")).unwrap();
        assert_eq!(body, b"existing");
    }

    #[test]
    fn root_subdir_platform_appropriate() {
        let sub = root_subdir();
        if cfg!(target_os = "windows") {
            assert_eq!(sub, "DLSSync");
        } else {
            assert_eq!(sub, ".dlssync");
        }
    }

    fn seed_legacy(root: &Path) {
        std::fs::create_dir_all(root.join("backups").join("steam-1091500").join("stamp")).unwrap();
        std::fs::write(
            root.join("backups")
                .join("steam-1091500")
                .join("stamp")
                .join("libxess_fg.dll"),
            b"legacy dll body",
        )
        .unwrap();
        std::fs::write(root.join("settings.json"), b"{\"blacklist\":[]}").unwrap();
        std::fs::write(root.join("catalog.json"), b"{}").unwrap();
        let backups_root = root.join("backups").to_string_lossy().into_owned();
        let db = root.join("backups.db");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE backups (id TEXT PRIMARY KEY, game_id TEXT NOT NULL,
              dll_family TEXT NOT NULL, dll_filename TEXT NOT NULL,
              original_path TEXT NOT NULL, backup_path TEXT NOT NULL,
              previous_version TEXT, previous_sha256 TEXT,
              created_at TEXT NOT NULL, restored_at TEXT);",
        )
        .unwrap();
        let bp = format!("{backups_root}\\steam-1091500\\stamp\\libxess_fg.dll");
        conn.execute(
            "INSERT INTO backups VALUES ('row-1', 'steam-1091500', 'xess_fg', 'libxess_fg.dll',
              'C:\\Games\\Cyberpunk\\libxess_fg.dll', ?1,
              '3.0', 'sha-old', '2026-05-20T00:00:00Z', NULL)",
            rusqlite::params![bp],
        )
        .unwrap();
    }

    #[test]
    fn migrate_from_fresh_install_is_noop() {
        let dir = tempdir().unwrap();
        let new_paths = AppPaths::from_root(dir.path().join("DLSSync"));
        new_paths.ensure_dirs().unwrap();
        let mut report = MigrationReport::default();
        new_paths.migrate_from(&dir.path().join("does-not-exist"), &mut report);
        assert!(report.legacy_root.is_none());
        assert_eq!(report.moved_files, 0);
        assert_eq!(report.rewrote_db_rows, 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn migrate_from_moves_files_and_rewrites_db() {
        let dir = tempdir().unwrap();
        let legacy = dir.path().join("legacy-roaming");
        seed_legacy(&legacy);
        let new_paths = AppPaths::from_root(dir.path().join("DLSSync"));
        new_paths.ensure_dirs().unwrap();
        let mut report = MigrationReport::default();
        new_paths.migrate_from(&legacy, &mut report);

        assert_eq!(report.moved_files, 1);
        assert!(report.copied_settings);
        assert!(report.copied_catalog);
        assert!(report.errors.is_empty());

        let moved = new_paths
            .backups_dir
            .join("steam-1091500")
            .join("stamp")
            .join("libxess_fg.dll");
        assert!(moved.exists(), "moved dll missing at {}", moved.display());
        assert!(new_paths.settings_file.exists());
        assert!(new_paths.catalog_cache.exists());
        assert!(new_paths.backups_db.exists());

        let conn = rusqlite::Connection::open(&new_paths.backups_db).unwrap();
        let bp: String = conn
            .query_row(
                "SELECT backup_path FROM backups WHERE id = 'row-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let expected = new_paths
            .backups_dir
            .join("steam-1091500")
            .join("stamp")
            .join("libxess_fg.dll")
            .to_string_lossy()
            .into_owned();
        assert_eq!(bp, expected);
    }

    #[test]
    fn migrate_from_is_idempotent_on_second_run() {
        let dir = tempdir().unwrap();
        let legacy = dir.path().join("legacy");
        seed_legacy(&legacy);
        let new_paths = AppPaths::from_root(dir.path().join("DLSSync"));
        new_paths.ensure_dirs().unwrap();

        let mut first = MigrationReport::default();
        new_paths.migrate_from(&legacy, &mut first);
        assert_eq!(first.moved_files, 1);

        let mut second = MigrationReport::default();
        new_paths.migrate_from(&legacy, &mut second);
        assert_eq!(second.moved_files, 0);
        assert!(!second.copied_settings);
        assert!(!second.copied_catalog);

        let dll_body = std::fs::read(
            new_paths
                .backups_dir
                .join("steam-1091500")
                .join("stamp")
                .join("libxess_fg.dll"),
        )
        .unwrap();
        assert_eq!(dll_body, b"legacy dll body");
    }

    #[test]
    fn migrate_from_skips_when_destinations_exist() {
        let dir = tempdir().unwrap();
        let legacy = dir.path().join("legacy");
        seed_legacy(&legacy);
        let new_paths = AppPaths::from_root(dir.path().join("DLSSync"));
        new_paths.ensure_dirs().unwrap();
        std::fs::write(&new_paths.settings_file, b"{\"blacklist\":[\"existing\"]}").unwrap();
        std::fs::write(&new_paths.catalog_cache, b"{\"existing\":true}").unwrap();
        let mut report = MigrationReport::default();
        new_paths.migrate_from(&legacy, &mut report);
        assert!(!report.copied_settings);
        assert!(!report.copied_catalog);
        let settings_body = std::fs::read(&new_paths.settings_file).unwrap();
        assert_eq!(settings_body, b"{\"blacklist\":[\"existing\"]}");
    }
}
