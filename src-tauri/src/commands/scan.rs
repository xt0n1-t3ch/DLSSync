use crate::constants::{
    ART_HTTP_TIMEOUT_SECS, SGDB_API_BASE, SGDB_GRID_DIMS, SGDB_HERO_DIMS, SGDB_USER_AGENT,
    STEAM_CAPSULE_PATH, STEAM_CDN_BASE, STEAM_HEADER_PATH, STEAM_HERO_PATH, STEAM_STORESEARCH,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use launcher_scan::{DetectedGame, LauncherKind};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::State;

#[tauri::command]
pub async fn scan_libraries(
    state: State<'_, AppState>,
    launchers: Vec<LauncherKind>,
) -> AppResult<Vec<DetectedGame>> {
    let (overrides, custom_folders) = {
        let s = state.settings.read();
        (
            s.launcher_overrides.clone(),
            s.launcher_overrides.custom.clone(),
        )
    };

    let games = tokio::task::spawn_blocking(move || {
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
    .map_err(|e| AppError::Other(e.to_string()))?;

    Ok(games)
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
        let id = format!(
            "custom-{}",
            sanitize_id(&format!("{}|{}", root.to_string_lossy(), name))
        );
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

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(48)
        .collect()
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
    let _ = SGDB_USER_AGENT;
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
