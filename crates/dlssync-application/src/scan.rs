use dll_catalog::Catalog;
use dlssync_contracts::{ScannedComponent, ScannedGame, TrustEvidence, UpdatePlanItem};
use launcher_scan::LauncherKind;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ScanUseCaseError {
    #[error("launcher scan: {0}")]
    Launcher(#[from] launcher_scan::ScanError),
    #[error("component scan: {0}")]
    Component(#[from] dll_scanner::ScanError),
    #[error("scan path has no display name: {0}")]
    InvalidPath(String),
}

pub fn scan_path(root: &Path) -> Result<ScannedGame, ScanUseCaseError> {
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ScanUseCaseError::InvalidPath(root.display().to_string()))?;
    Ok(ScannedGame {
        id: stable_game_id(root),
        name: name.to_string(),
        launcher: "manual".into(),
        install_dir: root.display().to_string(),
        components: scan_components(root)?,
    })
}

pub fn scan_installed_games() -> Result<Vec<ScannedGame>, ScanUseCaseError> {
    let launchers = [
        LauncherKind::Steam,
        LauncherKind::Epic,
        LauncherKind::Gog,
        LauncherKind::Ubisoft,
        LauncherKind::EaDesktop,
        LauncherKind::Xbox,
        LauncherKind::Battlenet,
    ];
    launcher_scan::scan_all(&launchers)?
        .into_iter()
        .map(|game| {
            Ok(ScannedGame {
                id: game.id,
                name: game.name,
                launcher: serde_json::to_value(game.launcher)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "unknown".into()),
                install_dir: game.install_dir.display().to_string(),
                components: scan_components(&game.install_dir)?,
            })
        })
        .collect()
}

pub fn plan_items(
    catalog: &Catalog,
    games: &[ScannedGame],
    backup_root: &Path,
    game_filter: Option<&str>,
) -> Vec<UpdatePlanItem> {
    let mut items = Vec::new();
    for game in games
        .iter()
        .filter(|game| game_filter.is_none_or(|filter| game.id == filter))
    {
        for component in &game.components {
            let path = Path::new(&component.path);
            let Some(filename) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let vendor = family_vendor(&component.family);
            let Some(release) = catalog.find_latest_for_file(vendor, &component.family, filename)
            else {
                continue;
            };
            if component.current_version.as_deref() == Some(release.version.as_str()) {
                continue;
            }
            let id = format!("{}:{}:{}", game.id, component.family, filename);
            items.push(UpdatePlanItem {
                id,
                game_id: game.id.clone(),
                game_name: game.name.clone(),
                dll_path: component.path.clone(),
                family: component.family.clone(),
                current_version: component.current_version.clone(),
                target_version: release.version,
                backup_path: backup_root
                    .join(&game.id)
                    .join(filename)
                    .display()
                    .to_string(),
                selected: true,
                trust: TrustEvidence {
                    source_url: release.cdn_url,
                    expected_sha256: release.sha256,
                    observed_sha256: component.sha256.clone(),
                    signature_subject: release.signature_subject,
                    signature_verified: release.signed,
                    anti_cheat_risk: None,
                },
            });
        }
    }
    items
}

fn scan_components(root: &Path) -> Result<Vec<ScannedComponent>, dll_scanner::ScanError> {
    dll_scanner::scan_install(root).map(|records| {
        records
            .into_iter()
            .map(|record| ScannedComponent {
                family: record.family.catalog_key().into(),
                path: record.path.display().to_string(),
                current_version: record.current_version,
                sha256: record.sha256,
            })
            .collect()
    })
}

fn stable_game_id(root: &Path) -> String {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(root.to_string_lossy().to_ascii_lowercase().as_bytes());
    format!("manual-{}", &hex::encode(digest)[..16])
}

fn family_vendor(family: &str) -> &'static str {
    match family {
        "xess_sr" | "xess_fg" | "xell" => "intel",
        "fsr_upscaler" | "fsr_fg" | "fsr_denoiser" => "amd",
        "direct_storage" | "direct_storage_core" => "microsoft",
        _ => "nvidia",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manual_scan_has_a_stable_non_path_id() {
        let dir = tempdir().unwrap();
        let first = scan_path(dir.path()).unwrap();
        let second = scan_path(dir.path()).unwrap();
        assert_eq!(first.id, second.id);
        assert!(first.id.starts_with("manual-"));
        assert!(!first.id.contains('\\'));
    }
}
