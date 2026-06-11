use crate::error::{AppError, AppResult};
use crate::paths::PathGuard;
use crate::state::AppState;
use backup_store::{BackupEntry, DeleteOutcome};
use tauri::State;

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> AppResult<Vec<BackupEntry>> {
    let guard = state.backups.read();
    let store = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
    Ok(store.list()?)
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, backup_id: String) -> AppResult<()> {
    let (entry, root_dir) = {
        let guard = state.backups.read();
        let store = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
        (store.get(&backup_id)?, store.root_dir.clone())
    };

    PathGuard::assert_under_root(&entry.backup_path, &root_dir)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    PathGuard::assert_dll_ext(&entry.original_path)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    PathGuard::deny_system_dir(&entry.original_path)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    PathGuard::assert_not_symlink(&entry.original_path)
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if !entry.backup_path.exists() {
        return Err(AppError::Other(format!(
            "backup file missing: {}",
            entry.backup_path.display()
        )));
    }

    if let Some(expected) = &entry.previous_sha256 {
        let backup_path = entry.backup_path.clone();
        let expected_owned = expected.clone();
        let actual =
            tokio::task::spawn_blocking(move || dll_catalog::hex_sha256_file(&backup_path))
                .await
                .map_err(|e| AppError::Other(e.to_string()))??;
        if !actual.eq_ignore_ascii_case(&expected_owned) {
            return Err(AppError::Other(format!(
                "backup integrity check failed: expected sha256 {expected_owned}, got {actual}"
            )));
        }
    }

    let backup_path = entry.backup_path.clone();
    let original_path = entry.original_path.clone();
    tokio::task::spawn_blocking(move || atomic_replace(&backup_path, &original_path))
        .await
        .map_err(|e| AppError::Other(e.to_string()))??;

    {
        let guard = state.backups.read();
        let store = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
        store.mark_restored(&backup_id, chrono::Utc::now())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_backup(
    state: State<'_, AppState>,
    backup_id: String,
) -> AppResult<DeleteOutcome> {
    let guard = state.backups.read();
    let store = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
    Ok(store.delete(&backup_id, true)?)
}

pub(crate) fn atomic_replace(
    source: &std::path::Path,
    dest: &std::path::Path,
) -> std::io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination has no parent",
        )
    })?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut src = std::fs::File::open(source)?;
        std::io::copy(&mut src, staged.as_file_mut())?;
        staged.as_file_mut().sync_all()?;
    }
    staged.persist(dest).map_err(|e| e.error)?;
    Ok(())
}
