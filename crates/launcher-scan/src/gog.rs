use crate::{DetectedGame, LauncherKind, LauncherScanner, ScanError};
use std::path::PathBuf;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

pub struct GogScanner;

impl LauncherScanner for GogScanner {
    fn kind(&self) -> LauncherKind {
        LauncherKind::Gog
    }

    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let root = match hklm.open_subkey("SOFTWARE\\WOW6432Node\\GOG.com\\Games") {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };
        let mut games = Vec::new();
        for name in root.enum_keys().flatten() {
            if let Ok(k) = root.open_subkey(&name) {
                let path: String = k.get_value("PATH").unwrap_or_default();
                let game_name: String = k.get_value("GAMENAME").unwrap_or_else(|_| name.clone());
                let p = PathBuf::from(&path);
                if !p.exists() {
                    continue;
                }
                games.push(DetectedGame {
                    id: format!("gog-{}", name),
                    name: game_name,
                    launcher: LauncherKind::Gog,
                    install_dir: p,
                    app_id: Some(name),
                    image_url: None,
                    size_bytes: None,
                });
            }
        }
        Ok(games)
    }
}
