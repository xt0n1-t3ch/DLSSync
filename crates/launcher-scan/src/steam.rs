use crate::{DetectedGame, LauncherKind, LauncherScanner, ScanError};
use std::path::{Path, PathBuf};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

pub struct SteamScanner;

impl LauncherScanner for SteamScanner {
    fn kind(&self) -> LauncherKind {
        LauncherKind::Steam
    }

    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        let steam_path = find_steam_install()?;
        let libraries = parse_library_folders(&steam_path)?;
        let mut games = Vec::new();
        for lib in libraries {
            let apps_dir = lib.join("steamapps");
            let read_dir = match std::fs::read_dir(&apps_dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                let s = name.to_string_lossy();
                if !s.starts_with("appmanifest_") || !s.ends_with(".acf") {
                    continue;
                }
                if let Some(game) = parse_appmanifest(&entry.path(), &apps_dir) {
                    games.push(game);
                }
            }
        }
        Ok(games)
    }
}

fn find_steam_install() -> Result<PathBuf, ScanError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [
        "SOFTWARE\\WOW6432Node\\Valve\\Steam",
        "SOFTWARE\\Valve\\Steam",
    ] {
        if let Ok(k) = hklm.open_subkey(subkey) {
            if let Ok(path) = k.get_value::<String, _>("InstallPath") {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Err(ScanError::Registry("Steam install path not found".into()))
}

fn parse_library_folders(steam_path: &Path) -> Result<Vec<PathBuf>, ScanError> {
    let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");
    let content = std::fs::read_to_string(&vdf_path)?;
    let mut libs = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("\"path\"") {
            let rest = rest.trim();
            if let Some(start) = rest.find('"') {
                let after = &rest[start + 1..];
                if let Some(end) = after.find('"') {
                    let raw = &after[..end];
                    let normalized = raw.replace("\\\\", "\\");
                    let p = PathBuf::from(normalized);
                    if p.exists() {
                        libs.push(p);
                    }
                }
            }
        }
    }
    if libs.is_empty() {
        libs.push(steam_path.to_path_buf());
    }
    Ok(libs)
}

const EXCLUDED_APPIDS: &[&str] = &[
    "228980", "1493710", "1391110", "1070560", "1628350", "228983",
];

const EXCLUDED_NAME_PREFIXES: &[&str] = &[
    "Steam Linux Runtime",
    "Proton ",
    "Proton-",
    "Proton Hotfix",
    "Steamworks ",
];

fn parse_appmanifest(path: &Path, apps_dir: &Path) -> Option<DetectedGame> {
    let content = std::fs::read_to_string(path).ok()?;
    let appid = extract_key(&content, "appid")?;
    if EXCLUDED_APPIDS.iter().any(|x| *x == appid) {
        return None;
    }
    let name = extract_key(&content, "name")?;
    if EXCLUDED_NAME_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return None;
    }
    let installdir = extract_key(&content, "installdir")?;
    let size = extract_key(&content, "SizeOnDisk").and_then(|s| s.parse::<u64>().ok());
    let install_path = apps_dir.join("common").join(&installdir);
    if !install_path.exists() {
        return None;
    }
    let image_url = Some(format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
        appid
    ));
    Some(DetectedGame {
        id: format!("steam-{}", appid),
        name,
        launcher: LauncherKind::Steam,
        install_dir: install_path,
        app_id: Some(appid),
        image_url,
        size_bytes: size,
    })
}

fn extract_key(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let idx = content.find(&needle)?;
    let after = &content[idx + needle.len()..];
    let q1 = after.find('"')?;
    let after = &after[q1 + 1..];
    let q2 = after.find('"')?;
    Some(after[..q2].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_appmanifest_keys() {
        let acf = r#""AppState"
{
    "appid"        "440"
    "name"         "Team Fortress 2"
    "installdir"   "Team Fortress 2"
    "SizeOnDisk"   "17179869184"
}"#;
        assert_eq!(extract_key(acf, "appid"), Some("440".into()));
        assert_eq!(extract_key(acf, "name"), Some("Team Fortress 2".into()));
        assert_eq!(
            extract_key(acf, "installdir"),
            Some("Team Fortress 2".into())
        );
        assert_eq!(extract_key(acf, "SizeOnDisk"), Some("17179869184".into()));
    }
}
