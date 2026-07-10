use crate::constants::{
    ART_HTTP_TIMEOUT_SECS, SGDB_API_BASE, SGDB_GRID_DIMS, SGDB_HERO_DIMS, STEAM_CAPSULE_PATH,
    STEAM_CDN_BASE, STEAM_HEADER_PATH, STEAM_HERO_PATH, STEAM_STORESEARCH,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use dlssync_contracts::{OperationActor, OperationKind, OperationRecord, OperationStatus};
use launcher_scan::{DetectedGame, LauncherKind};
use once_cell::sync::Lazy;
use operation_journal::{JournalError, JournalStore};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::State;

#[tauri::command]
pub async fn scan_libraries(
    state: State<'_, AppState>,
    launchers: Vec<LauncherKind>,
) -> AppResult<Vec<DetectedGame>> {
    let launchers = effective_launchers(launchers, e2e_mode_enabled());
    let launcher_count = launchers.len();
    let (overrides, custom_folders) = {
        let s = state.settings.read();
        (
            s.launcher_overrides.clone(),
            s.launcher_overrides.custom.clone(),
        )
    };

    let started = Instant::now();
    let result = tokio::task::spawn_blocking(move || {
        let mut all = launcher_scan::scan_all(&launchers).unwrap_or_default();

        for folder in custom_folders {
            let path = PathBuf::from(&folder);
            if let Some(games) = scan_custom_folder(&path) {
                all.extend(games);
            }
        }

        if overrides.steam.iter().any(|p| !p.is_empty()) {
            for extra in &overrides.steam {
                let p = PathBuf::from(extra);
                if let Some(games) = scan_custom_folder(&p) {
                    all.extend(games);
                }
            }
        }

        deduplicate(all)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()));

    let duration_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    let outcome = match &result {
        Ok(games) => Ok(games.len()),
        Err(err) => Err(err.to_string()),
    };
    if let Some(journal) = state.journal.read().as_ref() {
        if let Err(err) = record_scan(journal, launcher_count, duration_ms, outcome) {
            tracing::warn!(error = %err, "failed to journal library scan");
        }
    }

    result
}

/// Write one `OperationKind::Scan` journal record for a library scan. Success
/// carries the detected-game count; failure carries an actionable message. The
/// target is always `None` — a scan's journal entry never records Tony's paths or
/// private library names (the app-wide redaction guard handles any stray detail).
fn record_scan(
    journal: &JournalStore,
    launcher_count: usize,
    duration_ms: u32,
    outcome: Result<usize, String>,
) -> Result<(), JournalError> {
    let (status, summary, games_detected, error) = match outcome {
        Ok(count) => (
            OperationStatus::Succeeded,
            "Library scan completed",
            count,
            None,
        ),
        Err(message) => (
            OperationStatus::Failed,
            "Library scan failed",
            0,
            Some(message),
        ),
    };
    let details = BTreeMap::from([
        ("games_detected".to_string(), games_detected.to_string()),
        ("launchers_scanned".to_string(), launcher_count.to_string()),
    ]);
    journal.append(&OperationRecord {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        actor: OperationActor::Gui,
        kind: OperationKind::Scan,
        status,
        target: None,
        summary: summary.to_string(),
        details,
        duration_ms: Some(duration_ms),
        backup_id: None,
        error,
    })
}

fn effective_launchers(launchers: Vec<LauncherKind>, e2e_mode: bool) -> Vec<LauncherKind> {
    if e2e_mode {
        Vec::new()
    } else {
        launchers
    }
}

#[cfg(debug_assertions)]
fn e2e_mode_enabled() -> bool {
    std::env::var_os("DLSSYNC_E2E").as_deref() == Some(std::ffi::OsStr::new("1"))
}

#[cfg(not(debug_assertions))]
fn e2e_mode_enabled() -> bool {
    false
}

#[tauri::command]
pub async fn detect_dlls(
    _state: State<'_, AppState>,
    install_dir: String,
) -> AppResult<Vec<dll_scanner::DllRecord>> {
    let path = PathBuf::from(install_dir);
    crate::paths::PathGuard::assert_safe_scan_dir(&path)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let records = tokio::task::spawn_blocking(move || dll_scanner::scan_install(&path))
        .await
        .map_err(|e| AppError::Other(e.to_string()))??;
    Ok(records)
}

#[tauri::command]
pub async fn detect_dlss_enabler(
    _state: State<'_, AppState>,
    install_dir: String,
) -> AppResult<bool> {
    let path = PathBuf::from(install_dir);
    let present = tokio::task::spawn_blocking(move || dll_scanner::detect_dlss_enabler(&path))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(present)
}

fn deduplicate(games: Vec<DetectedGame>) -> Vec<DetectedGame> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(games.len());
    for g in games {
        if seen.insert(g.install_dir.clone()) {
            out.push(g);
        }
    }
    out
}

#[cfg(test)]
mod e2e_isolation_tests {
    use super::*;
    use dlssync_contracts::JournalFilter;

    #[test]
    fn e2e_mode_disables_host_launcher_discovery() {
        let launchers = vec![LauncherKind::Steam, LauncherKind::Epic];

        assert!(effective_launchers(launchers, true).is_empty());
    }

    #[test]
    fn normal_mode_preserves_requested_launchers() {
        let launchers = vec![LauncherKind::Steam, LauncherKind::Epic];

        assert_eq!(effective_launchers(launchers.clone(), false), launchers);
    }

    #[test]
    fn custom_game_ids_are_distinct_under_a_long_common_root() {
        let root = Path::new(
            r"C:\Users\someone\AppData\Local\Temp\dlssync-e2e-abcdef1234567890\FixtureGames",
        );
        let a = custom_game_id(&root.join("Aurora Protocol"), "Aurora Protocol");
        let b = custom_game_id(&root.join("Neon Divide"), "Neon Divide");
        assert_ne!(
            a, b,
            "distinct games under a long common root must get distinct ids (both were {a})"
        );
    }

    #[test]
    fn custom_game_id_is_stable_and_deterministic() {
        let dir = Path::new(r"C:\Games\Neon Divide");
        assert_eq!(
            custom_game_id(dir, "Neon Divide"),
            custom_game_id(dir, "Neon Divide")
        );
    }

    #[test]
    fn custom_game_id_is_readable_and_path_sensitive() {
        let id = custom_game_id(Path::new(r"C:\Games\Neon Divide"), "Neon Divide");
        assert!(
            id.starts_with("custom-"),
            "id must keep the custom- prefix: {id}"
        );
        assert!(
            id.to_lowercase().contains("neon"),
            "id must carry a readable name slug: {id}"
        );
        // Same name in a different folder is a different game -> different id.
        let other = custom_game_id(Path::new(r"D:\Library\Neon Divide"), "Neon Divide");
        assert_ne!(id, other);
    }

    #[test]
    fn custom_game_id_normalizes_windows_path_identity() {
        // Case + separator differences describe the same Windows path -> same id.
        let a = custom_game_id(Path::new(r"C:\Games\Neon Divide"), "Neon Divide");
        let b = custom_game_id(Path::new("c:/games/neon divide"), "Neon Divide");
        if cfg!(windows) {
            assert_eq!(
                a, b,
                "windows path identity must be case/separator-insensitive"
            );
        }
    }

    #[test]
    fn successful_scan_writes_one_gui_scan_journal_record() {
        let dir = tempfile::tempdir().unwrap();
        let journal = JournalStore::open(dir.path().join("journal.db")).unwrap();
        record_scan(&journal, 2, 123, Ok(4)).unwrap();
        let rows = journal.list(&JournalFilter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        let rec = &rows[0];
        assert_eq!(rec.kind, OperationKind::Scan);
        assert_eq!(rec.actor, OperationActor::Gui);
        assert_eq!(rec.status, OperationStatus::Succeeded);
        assert_eq!(rec.duration_ms, Some(123));
        assert_eq!(
            rec.details.get("games_detected").map(String::as_str),
            Some("4")
        );
        assert!(
            rec.target.is_none(),
            "scan journal must never carry a path or library-name target"
        );
    }

    #[test]
    fn failed_scan_records_failure_without_leaking_paths() {
        let dir = tempfile::tempdir().unwrap();
        let journal = JournalStore::open(dir.path().join("journal.db")).unwrap();
        record_scan(&journal, 1, 5, Err("scan worker crashed".to_string())).unwrap();
        let rows = journal.list(&JournalFilter::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, OperationStatus::Failed);
        assert_eq!(rows[0].error.as_deref(), Some("scan worker crashed"));
        assert!(rows[0].target.is_none());
        let export = journal
            .export_redacted_json(&JournalFilter::default())
            .unwrap();
        assert!(
            !export.contains(":\\"),
            "redacted export must not leak windows paths"
        );
    }
}

fn scan_custom_folder(root: &Path) -> Option<Vec<DetectedGame>> {
    if !root.exists() {
        return None;
    }
    let mut games = Vec::new();
    let read = match std::fs::read_dir(root) {
        Ok(r) => r,
        Err(_) => return None,
    };
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        if !is_likely_game(&path, &name) {
            tracing::debug!(folder = %name, "skipped custom folder — not a game");
            continue;
        }
        let id = custom_game_id(&path, &name);
        games.push(DetectedGame {
            id,
            name,
            launcher: LauncherKind::Manual,
            install_dir: path,
            app_id: None,
            image_url: None,
            size_bytes: None,
        });
    }
    Some(games)
}

/// Stable, collision-free id for a custom-folder game. The game name provides a
/// readable, bounded slug; the full install path — normalized for Windows
/// case/separator identity — provides a SHA-256 suffix so two games under the
/// same long root can never collapse to one id. Deterministic across processes
/// (unlike a randomized `DefaultHasher`).
fn custom_game_id(install_dir: &Path, name: &str) -> String {
    format!(
        "custom-{}-{}",
        name_slug(name),
        stable_path_hash(install_dir)
    )
}

/// Readable, bounded ascii kebab slug of a game name; empty input -> "game".
fn name_slug(name: &str) -> String {
    let mut slug = String::with_capacity(40);
    let mut pending_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch.to_ascii_lowercase());
            if slug.len() >= 40 {
                break;
            }
        } else {
            pending_dash = true;
        }
    }
    if slug.is_empty() {
        "game".to_string()
    } else {
        slug
    }
}

/// Case/separator-normalized string identity of a path. Windows paths are
/// case-insensitive and accept either separator, so the same location always
/// hashes to the same value.
fn normalize_path_identity(path: &Path) -> String {
    let unified = path.to_string_lossy().replace('/', "\\");
    if cfg!(windows) {
        unified.to_lowercase()
    } else {
        unified
    }
}

/// First 12 hex chars of the SHA-256 of the normalized path — deterministic and
/// stable across runs.
fn stable_path_hash(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(normalize_path_identity(path).as_bytes());
    digest[..6].iter().map(|b| format!("{:02x}", b)).collect()
}

static EXCLUDED_FOLDER_NAMES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "saves",
        "save",
        "savegames",
        "saved games",
        "save games",
        "backup",
        "backups",
        "backup_old",
        "old",
        "archive",
        "archived",
        "cache",
        "caches",
        "cached",
        "logs",
        "log",
        "crashes",
        "crash",
        "temp",
        "tmp",
        "trash",
        "recycle",
        "downloads",
        "download",
        "documents",
        "pictures",
        "tools",
        "tool",
        "utility",
        "utilities",
        "utils",
        "mods",
        "mod",
        "patches",
        "patch",
        "addons",
        "plugins",
        "extracted",
        "extracts",
        "unpacked",
        "build",
        "builds",
        "output",
        "obj",
        "bin",
        "configs",
        "config",
        "configuration",
        "settings",
        "resources",
        "assets",
        "data",
        "workspace",
        "scratch",
        "test",
        "tests",
        "source",
        "src",
        "sources",
        "docs",
        "documentation",
        "common",
        "shared",
        ".cache",
        ".config",
        ".git",
        ".vs",
        ".idea",
        "node_modules",
        "__pycache__",
        "venv",
        "system volume information",
        "$recycle.bin",
    ]
    .into_iter()
    .collect()
});

static EXCLUDED_MODDING_TOOLS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "xedit",
        "fo4edit",
        "fnvedit",
        "fo3edit",
        "sseedit",
        "tes5edit",
        "tes4edit",
        "enderaledit",
        "f76edit",
        "starfieldedit",
        "zedit",
        "fomm",
        "fomm-le",
        "mo",
        "mo2",
        "mod organizer",
        "mod organizer 2",
        "wrye",
        "wrye bash",
        "wrye flash",
        "wrye smash",
        "wrye mash",
        "loot",
        "vortex",
        "nmm",
        "nexus mod manager",
        "bodyslide",
        "bs",
        "bodyslide and outfit studio",
        "outfitstudio",
        "obse",
        "skse",
        "skse64",
        "f4se",
        "mwse",
        "nvse",
        "fose",
        "starfieldse",
        "nemesis",
        "fnis",
        "cathedral assets optimizer",
        "cao",
        "bsa browser",
        "bsa unpacker",
        "bae",
        "xlodgen",
        "dyndolod",
        "tes5lodgen",
        "synthesis",
        "mator smash",
        "creation kit",
        "ck",
        "geck",
        "enbseries",
        "enb",
        "enboost",
        "reshade",
        "reshade-shaders",
        "spriggit",
        "fomod",
        "ddsopt",
        "uvtools",
        "zlibmod",
    ]
    .into_iter()
    .collect()
});

#[derive(Default)]
struct FolderMarkers {
    has_engine_marker: bool,
    max_exe_mb: u64,
}

fn is_likely_game(path: &Path, folder_name: &str) -> bool {
    if folder_name.len() < 3 {
        return false;
    }
    let lower = folder_name.to_lowercase();
    if EXCLUDED_FOLDER_NAMES.contains(lower.as_str()) {
        return false;
    }
    if EXCLUDED_MODDING_TOOLS.contains(lower.as_str()) {
        return false;
    }
    if lower.starts_with('.') || lower.starts_with('$') || lower.starts_with('_') {
        return false;
    }
    if lower.starts_with("backup") || lower.starts_with("temp") || lower.ends_with("-backup") {
        return false;
    }

    let m = scan_folder_markers(path, 0);
    m.has_engine_marker || m.max_exe_mb >= 5
}

fn scan_folder_markers(path: &Path, depth: u8) -> FolderMarkers {
    let mut m = FolderMarkers::default();
    if depth > 2 {
        return m;
    }
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return m,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name_lower = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        if p.is_file() {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "exe" {
                if let Ok(meta) = entry.metadata() {
                    let mb = meta.len() / (1024 * 1024);
                    if mb > m.max_exe_mb {
                        m.max_exe_mb = mb;
                    }
                }
            }
            if ext == "uproject" || ext == "uplugin" || name_lower == "unityplayer.dll" {
                m.has_engine_marker = true;
            }
        } else if p.is_dir() {
            if name_lower == "binaries"
                || name_lower == "bin64"
                || name_lower == "engine"
                || name_lower == "content"
                || name_lower == "paks"
                || name_lower.ends_with("_data")
            {
                m.has_engine_marker = true;
            }
            if depth < 2
                && (name_lower == "binaries"
                    || name_lower == "bin64"
                    || name_lower == "bin"
                    || name_lower == "win64"
                    || name_lower == "x64"
                    || name_lower == "game"
                    || name_lower == "engine")
            {
                let nested = scan_folder_markers(&p, depth + 1);
                if nested.max_exe_mb > m.max_exe_mb {
                    m.max_exe_mb = nested.max_exe_mb;
                }
                if nested.has_engine_marker {
                    m.has_engine_marker = true;
                }
            }
        }
    }
    m
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameArt {
    pub grid_url: Option<String>,
    pub hero_url: Option<String>,
    pub capsule_url: Option<String>,
}

fn empty_art() -> GameArt {
    GameArt {
        grid_url: None,
        hero_url: None,
        capsule_url: None,
    }
}

#[tauri::command]
pub async fn fetch_steam_art(state: State<'_, AppState>, name: String) -> AppResult<GameArt> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Ok(empty_art());
    }
    let _ = ART_HTTP_TIMEOUT_SECS;
    let client = state.http_art.clone();

    let url = format!(
        "{}?term={}&l=english&cc=us",
        STEAM_STORESEARCH,
        encode_path(trimmed)
    );
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, name = %trimmed, "steam storesearch request failed");
            return Ok(empty_art());
        }
    };
    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, name = %trimmed, "steam storesearch parse failed");
            return Ok(empty_art());
        }
    };
    let appid = body
        .get("items")
        .and_then(|v| v.as_array())
        .and_then(|items| {
            items
                .iter()
                .find(|it| it.get("type").and_then(|t| t.as_str()) == Some("app"))
                .and_then(|it| it.get("id"))
                .and_then(|i| i.as_i64())
        });
    let Some(id) = appid else {
        return Ok(empty_art());
    };
    Ok(GameArt {
        grid_url: Some(format!("{STEAM_CDN_BASE}/{id}/{STEAM_HEADER_PATH}")),
        hero_url: Some(format!("{STEAM_CDN_BASE}/{id}/{STEAM_HERO_PATH}")),
        capsule_url: Some(format!("{STEAM_CDN_BASE}/{id}/{STEAM_CAPSULE_PATH}")),
    })
}

#[tauri::command]
pub async fn enrich_game_art(
    state: State<'_, AppState>,
    name: String,
    api_key: String,
) -> AppResult<GameArt> {
    if api_key.trim().is_empty() || name.trim().is_empty() {
        return Ok(empty_art());
    }
    let trimmed = name.trim();
    let client = state.http_art.clone();

    let search_url = format!(
        "{SGDB_API_BASE}/search/autocomplete/{}",
        encode_path(trimmed)
    );
    let search: serde_json::Value = match client.get(&search_url).bearer_auth(&api_key).send().await
    {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, name = %trimmed, "sgdb search parse failed");
                return Ok(empty_art());
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, name = %trimmed, "sgdb search request failed");
            return Ok(empty_art());
        }
    };

    let game_id = search
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|g| g.get("id"))
        .and_then(|i| i.as_i64());
    let Some(gid) = game_id else {
        return Ok(empty_art());
    };

    let grid_url = fetch_first_asset_url(
        &client,
        &api_key,
        &format!("{SGDB_API_BASE}/grids/game/{gid}?dimensions={SGDB_GRID_DIMS}&types=static"),
    )
    .await;
    let hero_url = fetch_first_asset_url(
        &client,
        &api_key,
        &format!("{SGDB_API_BASE}/heroes/game/{gid}?dimensions={SGDB_HERO_DIMS}&types=static"),
    )
    .await;

    Ok(GameArt {
        grid_url,
        hero_url,
        capsule_url: None,
    })
}

async fn fetch_first_asset_url(client: &reqwest::Client, key: &str, url: &str) -> Option<String> {
    let resp = client.get(url).bearer_auth(key).send().await.ok()?;
    let val: serde_json::Value = resp.json().await.ok()?;
    val.get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|g| g.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from)
}

fn encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
