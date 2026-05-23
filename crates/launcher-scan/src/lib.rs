use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("registry: {0}")]
    Registry(String),
    #[error("parse: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherKind {
    Steam,
    Epic,
    Gog,
    Ubisoft,
    EaDesktop,
    Xbox,
    Battlenet,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedGame {
    pub id: String,
    pub name: String,
    pub launcher: LauncherKind,
    pub install_dir: PathBuf,
    pub app_id: Option<String>,
    pub image_url: Option<String>,
    pub size_bytes: Option<u64>,
}

pub trait LauncherScanner {
    fn kind(&self) -> LauncherKind;
    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError>;
}

#[cfg(windows)]
mod epic;
#[cfg(windows)]
mod gog;
#[cfg(windows)]
mod steam;
#[cfg(windows)]
mod ubisoft;

#[cfg(windows)]
pub use epic::EpicScanner;
#[cfg(windows)]
pub use gog::GogScanner;
#[cfg(windows)]
pub use steam::SteamScanner;
#[cfg(windows)]
pub use ubisoft::UbisoftScanner;

#[cfg(windows)]
pub struct EaDesktopScanner;
#[cfg(windows)]
impl LauncherScanner for EaDesktopScanner {
    fn kind(&self) -> LauncherKind {
        LauncherKind::EaDesktop
    }
    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        Ok(Vec::new())
    }
}

#[cfg(windows)]
pub struct XboxScanner;
#[cfg(windows)]
impl LauncherScanner for XboxScanner {
    fn kind(&self) -> LauncherKind {
        LauncherKind::Xbox
    }
    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        Ok(Vec::new())
    }
}

#[cfg(windows)]
pub struct BattlenetScanner;
#[cfg(windows)]
impl LauncherScanner for BattlenetScanner {
    fn kind(&self) -> LauncherKind {
        LauncherKind::Battlenet
    }
    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        Ok(Vec::new())
    }
}

pub fn scan_all(launchers: &[LauncherKind]) -> Result<Vec<DetectedGame>, ScanError> {
    let mut out = Vec::new();
    #[cfg(windows)]
    {
        for kind in launchers {
            let result = match kind {
                LauncherKind::Steam => SteamScanner.scan(),
                LauncherKind::Epic => EpicScanner.scan(),
                LauncherKind::Gog => GogScanner.scan(),
                LauncherKind::Ubisoft => UbisoftScanner.scan(),
                LauncherKind::EaDesktop => EaDesktopScanner.scan(),
                LauncherKind::Xbox => XboxScanner.scan(),
                LauncherKind::Battlenet => BattlenetScanner.scan(),
                LauncherKind::Manual => Ok(Vec::new()),
            };
            match result {
                Ok(g) => out.extend(g),
                Err(e) => tracing::warn!(launcher = ?kind, error = %e, "launcher scan failed"),
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = launchers;
    }
    Ok(out)
}
