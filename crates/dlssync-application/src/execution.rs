use backup_store::{BackupEntry, BackupStore};
use dll_catalog::Catalog;
use dlssync_contracts::{ApplyPlanResult, RollbackPlanResult, UpdatePlan};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("update plan is stale: {0}")]
    Stale(String),
    #[error("unsafe target path: {0}")]
    UnsafeTarget(String),
    #[error("catalog release missing for {0}")]
    MissingRelease(String),
    #[error("download or integrity verification failed: {0}")]
    Catalog(#[from] dll_catalog::CatalogError),
    #[error("backup failed: {0}")]
    Backup(#[from] backup_store::BackupError),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Authenticode verification failed: {0}")]
    Authenticode(String),
}

pub async fn apply_update_plan(
    catalog: &Catalog,
    plan: &UpdatePlan,
    client: &reqwest::Client,
    backups: &BackupStore,
) -> Result<ApplyPlanResult, ExecutionError> {
    if plan.stale || plan.catalog_generated_at != catalog.generated_at.to_rfc3339() {
        return Err(ExecutionError::Stale(plan.id.clone()));
    }

    let selected: Vec<_> = plan.items.iter().filter(|item| item.selected).collect();
    let staging = tempfile::tempdir_in(&backups.root_dir)?;
    let mut prepared = Vec::with_capacity(selected.len());
    for item in selected {
        let target = checked_target(&item.dll_path)?;
        let filename = target
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ExecutionError::UnsafeTarget(item.dll_path.clone()))?;
        let vendor = family_vendor(&item.family);
        let release = catalog
            .find_file(vendor, &item.family, &item.target_version, filename)
            .or_else(|| catalog.find(vendor, &item.family, &item.target_version))
            .ok_or_else(|| ExecutionError::MissingRelease(item.id.clone()))?;
        let staged =
            dll_catalog::download_and_extract_dll(client, &release, staging.path()).await?;
        verify_publisher(&staged, vendor)?;
        prepared.push((item, target, staged));
    }

    let now = chrono::Utc::now();
    let mut backup_entries = Vec::with_capacity(prepared.len());
    for (item, target, _) in &prepared {
        let filename = target
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ExecutionError::UnsafeTarget(item.dll_path.clone()))?;
        let backup_path = PathBuf::from(&item.backup_path);
        if !backup_path.starts_with(&backups.root_dir) {
            return Err(ExecutionError::UnsafeTarget(
                backup_path.display().to_string(),
            ));
        }
        copy_and_sync(target, &backup_path)?;
        let entry = BackupEntry {
            id: uuid::Uuid::new_v4().to_string(),
            game_id: item.game_id.clone(),
            dll_family: item.family.clone(),
            dll_filename: filename.into(),
            original_path: target.clone(),
            backup_path,
            previous_version: item.current_version.clone(),
            previous_sha256: item.trust.observed_sha256.clone(),
            created_at: now,
            restored_at: None,
            size_bytes: std::fs::metadata(target).ok().map(|value| value.len()),
            backup_type: "dll".into(),
            device_class: None,
            hardware_id: None,
            driver_provider: None,
        };
        backups.insert(&entry)?;
        backup_entries.push(entry);
    }

    let mut replaced = 0usize;
    for ((_, target, staged), backup) in prepared.iter().zip(&backup_entries) {
        if let Err(error) = replace_atomic(staged, target) {
            for restored in backup_entries[..replaced].iter().rev() {
                let _ = replace_atomic(&restored.backup_path, &restored.original_path);
            }
            return Err(ExecutionError::Io(error));
        }
        let observed = dll_catalog::hex_sha256_file(target)?;
        let expected = dll_catalog::hex_sha256_file(staged)?;
        if observed != expected {
            for restored in backup_entries[..=replaced].iter().rev() {
                let _ = replace_atomic(&restored.backup_path, &restored.original_path);
            }
            return Err(ExecutionError::UnsafeTarget(format!(
                "post-write hash mismatch for {}",
                target.display()
            )));
        }
        let _ = backup;
        replaced += 1;
    }

    Ok(ApplyPlanResult {
        plan_id: plan.id.clone(),
        applied: replaced as u32,
        backup_paths: backup_entries
            .iter()
            .map(|entry| entry.backup_path.display().to_string())
            .collect(),
    })
}

pub fn rollback_update_plan(plan: &UpdatePlan) -> Result<RollbackPlanResult, ExecutionError> {
    let mut restored = 0;
    for item in plan.items.iter().filter(|item| item.selected).rev() {
        let target = checked_target(&item.dll_path)?;
        let backup = PathBuf::from(&item.backup_path);
        if backup.is_file() {
            replace_atomic(&backup, &target)?;
            restored += 1;
        }
    }
    Ok(RollbackPlanResult {
        plan_id: plan.id.clone(),
        restored,
    })
}

fn checked_target(value: &str) -> Result<PathBuf, ExecutionError> {
    let path = PathBuf::from(value);
    if !path.is_file()
        || !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("dll"))
        || std::fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(true)
    {
        return Err(ExecutionError::UnsafeTarget(path.display().to_string()));
    }
    if let Some(windows) = std::env::var_os("WINDIR").map(PathBuf::from) {
        if path.starts_with(windows) {
            return Err(ExecutionError::UnsafeTarget(path.display().to_string()));
        }
    }
    Ok(path)
}

fn verify_publisher(path: &Path, vendor: &str) -> Result<(), ExecutionError> {
    let info = pe_version::read_authenticode(path)
        .ok_or_else(|| ExecutionError::Authenticode("signature is missing".into()))?;
    pe_version::enforce_subject(&info, vendor).map_err(ExecutionError::Authenticode)?;
    if !info.trusted {
        return Err(ExecutionError::Authenticode(
            "the Authenticode chain is not trusted".into(),
        ));
    }
    Ok(())
}

fn copy_and_sync(source: &Path, destination: &Path) -> std::io::Result<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, destination)?;
    std::fs::File::open(destination)?.sync_all()
}

fn replace_atomic(source: &Path, destination: &Path) -> std::io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "target has no parent")
    })?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    let mut input = std::fs::File::open(source)?;
    std::io::copy(&mut input, &mut staged)?;
    staged.as_file().sync_all()?;
    staged.persist(destination).map_err(|error| error.error)?;
    Ok(())
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
    use dlssync_contracts::{TrustEvidence, UpdatePlanItem};
    use tempfile::tempdir;

    #[test]
    fn rollback_restores_backup_bytes_atomically() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("game.dll");
        let backup = dir.path().join("backup.dll");
        std::fs::write(&target, b"new").unwrap();
        std::fs::write(&backup, b"old").unwrap();
        let plan = UpdatePlan {
            id: "plan".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            catalog_generated_at: chrono::Utc::now().to_rfc3339(),
            fingerprint: "fingerprint".into(),
            stale: false,
            items: vec![UpdatePlanItem {
                id: "item".into(),
                game_id: "game".into(),
                game_name: "Game".into(),
                dll_path: target.display().to_string(),
                family: "dlss_sr".into(),
                current_version: None,
                target_version: "1".into(),
                backup_path: backup.display().to_string(),
                selected: true,
                trust: TrustEvidence {
                    source_url: String::new(),
                    expected_sha256: String::new(),
                    observed_sha256: None,
                    signature_subject: None,
                    signature_verified: false,
                    anti_cheat_risk: None,
                },
            }],
        };
        let result = rollback_update_plan(&plan).unwrap();
        assert_eq!(result.restored, 1);
        assert_eq!(std::fs::read(target).unwrap(), b"old");
    }

    #[tokio::test]
    async fn stale_plan_is_rejected_before_network_or_backup_mutation() {
        let dir = tempdir().unwrap();
        let catalog = dll_catalog::embedded_fallback_catalog().unwrap();
        let plan = UpdatePlan {
            id: "stale-plan".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            catalog_generated_at: "2000-01-01T00:00:00+00:00".into(),
            fingerprint: String::new(),
            stale: false,
            items: Vec::new(),
        };
        let backups =
            BackupStore::open(dir.path().join("backups.db"), dir.path().join("files")).unwrap();
        let error = apply_update_plan(&catalog, &plan, &reqwest::Client::new(), &backups)
            .await
            .unwrap_err();
        assert!(matches!(error, ExecutionError::Stale(_)));
        assert!(backups.list().unwrap().is_empty());
    }
}
