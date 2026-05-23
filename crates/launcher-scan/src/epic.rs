use crate::{DetectedGame, LauncherKind, LauncherScanner, ScanError};
use std::path::PathBuf;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

pub struct EpicScanner;

impl LauncherScanner for EpicScanner {
    fn kind(&self) -> LauncherKind {
        LauncherKind::Epic
    }

    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        let manifests_dir = match find_manifests_dir() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let entries = match std::fs::read_dir(&manifests_dir) {
            Ok(d) => d,
            Err(_) => return Ok(Vec::new()),
        };
        let mut games = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                != Some("item".to_string())
            {
                continue;
            }
            if let Some(game) = parse_manifest(&path) {
                games.push(game);
            }
        }
        Ok(games)
    }
}

fn find_manifests_dir() -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [
        "SOFTWARE\\WOW6432Node\\Epic Games\\EpicGamesLauncher",
        "SOFTWARE\\Epic Games\\EpicGamesLauncher",
    ] {
        if let Ok(k) = hklm.open_subkey(subkey) {
            if let Ok(path) = k.get_value::<String, _>("AppDataPath") {
                let dir = PathBuf::from(path).join("Manifests");
                if dir.exists() {
                    return Some(dir);
                }
            }
        }
    }
    let fallback = PathBuf::from("C:\\ProgramData\\Epic\\EpicGamesLauncher\\Data\\Manifests");
    if fallback.exists() {
        return Some(fallback);
    }
    None
}

fn parse_manifest(path: &std::path::Path) -> Option<DetectedGame> {
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let install_location = v.get("InstallLocation")?.as_str()?.to_string();
    let install_dir = PathBuf::from(&install_location);
    if !install_dir.exists() {
        return None;
    }
    let app_name = v.get("AppName").and_then(|x| x.as_str()).unwrap_or("");
    let display = v
        .get("DisplayName")
        .and_then(|x| x.as_str())
        .unwrap_or(app_name)
        .to_string();
    let catalog_item_id = v
        .get("CatalogItemId")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    Some(DetectedGame {
        id: format!("epic-{}", app_name),
        name: display,
        launcher: LauncherKind::Epic,
        install_dir,
        app_id: catalog_item_id.or_else(|| Some(app_name.to_string())),
        image_url: None,
        size_bytes: None,
    })
}
