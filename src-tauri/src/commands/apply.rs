use crate::error::{AppError, AppResult};
use crate::state::AppState;
use backup_store::BackupEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Deserialize)]
pub struct ApplyRequest {
    pub apply_id: String,
    pub game_id: String,
    pub dll_path: String,
    pub vendor: String,
    pub family: String,
    pub target_version: String,
    #[serde(default)]
    pub game_label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApplyResult {
    pub apply_id: String,
    pub backup_id: String,
    pub previous_version: Option<String>,
    pub new_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyProgress {
    pub apply_id: String,
    pub stage: String,
    pub message: String,
    pub progress: Option<f64>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn apply_update(
    handle: AppHandle,
    state: State<'_, AppState>,
    request: ApplyRequest,
) -> AppResult<ApplyResult> {
    let apply_id = request.apply_id.clone();
    let emit = |stage: &str, message: &str, progress: Option<f64>, error: Option<String>| {
        let _ = handle.emit(
            "apply_progress",
            ApplyProgress {
                apply_id: apply_id.clone(),
                stage: stage.to_string(),
                message: message.to_string(),
                progress,
                error,
            },
        );
    };

    let release = {
        let guard = state.catalog.read();
        let catalog = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("catalog not loaded".into()))?;
        catalog
            .find(&request.vendor, &request.family, &request.target_version)
            .ok_or_else(|| {
                AppError::Other(format!(
                    "release {}::{}::{} not in catalog",
                    request.vendor, request.family, request.target_version
                ))
            })?
    };

    let dll_path = PathBuf::from(&request.dll_path);
    if !dll_path.exists() {
        emit(
            "failed",
            "DLL file disappeared",
            None,
            Some("missing".into()),
        );
        return Err(AppError::Other(format!(
            "dll not found: {}",
            dll_path.display()
        )));
    }

    if let Err(reason) = ensure_writable(&dll_path) {
        let msg = format!("{} — close the game and retry", reason);
        emit("failed", "DLL is locked", None, Some(msg.clone()));
        return Err(AppError::Other(msg));
    }

    let backup_root = {
        let guard = state.backups.read();
        let store = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
        store.root_dir.clone()
    };

    emit(
        "download",
        &format!("Downloading {} v{}", release.filename, release.version),
        Some(0.0),
        None,
    );
    let staging = tempfile::tempdir_in(&backup_root)?;
    let http = state.http.clone();
    let release_clone = release.clone();
    let staging_path = staging.path().to_path_buf();
    let staged_dll =
        match dll_catalog::download_and_extract_dll(&http, &release_clone, &staging_path).await {
            Ok(p) => {
                emit("download", "Downloaded", Some(1.0), None);
                p
            }
            Err(e) => {
                emit("failed", "Download failed", None, Some(e.to_string()));
                return Err(e.into());
            }
        };

    let algo = dll_catalog::HashAlgo::from_hex_len(&release.sha256)
        .unwrap_or(dll_catalog::HashAlgo::Sha256);
    let algo_label = match algo {
        dll_catalog::HashAlgo::Sha256 => "SHA-256",
        dll_catalog::HashAlgo::Md5 => "MD5",
    };
    emit("verify_sha", &format!("Verifying {algo_label}"), None, None);
    let new_hash = tokio::task::spawn_blocking({
        let staged = staged_dll.clone();
        move || dll_catalog::hash_file_with(&staged, algo)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))??;
    if !new_hash.eq_ignore_ascii_case(&release.sha256) {
        let err = format!(
            "{algo_label} mismatch: expected {} got {}",
            release.sha256, new_hash
        );
        emit("failed", "Integrity check failed", None, Some(err.clone()));
        return Err(AppError::Other(err));
    }
    emit("verify_sha", &format!("{algo_label} OK"), None, None);

    // FR-043 — Authenticode publisher gate.
    let allow_unsigned = state.settings.read().advanced.allow_unsigned_dlls;
    emit(
        "verify_signature",
        "Reading Authenticode signature",
        None,
        None,
    );
    let auth_info = tokio::task::spawn_blocking({
        let staged = staged_dll.clone();
        move || pe_version::read_authenticode(&staged)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?;
    match auth_info {
        Some(info) => match pe_version::enforce_subject(&info, &request.vendor) {
            Ok(()) => {
                let trust_tag = if info.trusted {
                    "trusted"
                } else {
                    "untrusted-chain"
                };
                emit(
                    "verify_signature",
                    &format!(
                        "Signed by {} ({trust_tag})",
                        info.subject_cn.as_deref().unwrap_or("?")
                    ),
                    None,
                    None,
                );
            }
            Err(reason) if allow_unsigned => {
                tracing::warn!("signature gate bypassed (allow_unsigned_dlls=true): {reason}");
                emit(
                    "verify_signature",
                    &format!("Signature mismatch ignored: {reason}"),
                    None,
                    None,
                );
            }
            Err(reason) => {
                let with_hint = enrich_signature_error(&reason);
                emit(
                    "failed",
                    "Signature rejected",
                    None,
                    Some(with_hint.clone()),
                );
                return Err(AppError::Other(with_hint));
            }
        },
        None if allow_unsigned => {
            emit(
                "verify_signature",
                "No signature data (unsigned mode enabled)",
                None,
                None,
            );
        }
        None => {
            let err = "Authenticode signature could not be read — try enabling \
                       'Allow unsigned DLLs' in Settings → Advanced if this vendor \
                       ships unsigned binaries"
                .to_string();
            emit("failed", "Signature unreadable", None, Some(err.clone()));
            return Err(AppError::Other(err));
        }
    }

    let previous_sha = tokio::task::spawn_blocking({
        let dll_path = dll_path.clone();
        move || dll_catalog::hex_sha256_file(&dll_path)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))??;

    let previous_version = tokio::task::spawn_blocking({
        let dll_path = dll_path.clone();
        move || pe_version::read_dll_version(&dll_path).ok()
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
    .map(|v| v.file_version);

    emit("backup", "Backing up current DLL", None, None);
    let entry_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now();
    let filename = dll_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.dll")
        .to_string();
    let backup_path = {
        let guard = state.backups.read();
        let store = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
        let label = request
            .game_label
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&request.game_id);
        store.allocate_backup_path(label, created_at, &filename)?
    };
    std::fs::copy(&dll_path, &backup_path)?;
    let entry = BackupEntry {
        id: entry_id.clone(),
        game_id: request.game_id.clone(),
        dll_family: request.family.clone(),
        dll_filename: filename.clone(),
        original_path: dll_path.clone(),
        backup_path: backup_path.clone(),
        previous_version: previous_version.clone(),
        previous_sha256: Some(previous_sha.clone()),
        created_at,
        restored_at: None,
        size_bytes: std::fs::metadata(&backup_path).ok().map(|m| m.len()),
    };
    {
        let guard = state.backups.read();
        let store = guard
            .as_ref()
            .ok_or_else(|| AppError::Other("backup store not initialized".into()))?;
        store.insert(&entry)?;
    }
    emit("backup", "Backup created", None, None);

    emit("replace", "Installing new DLL", None, None);
    // Edge: symlink check — refuse to clobber link targets we don't own.
    if let Ok(meta) = std::fs::symlink_metadata(&dll_path) {
        if meta.file_type().is_symlink() {
            let err = format!("refusing to replace symlink: {}", dll_path.display());
            emit("failed", "Symlink detected", None, Some(err.clone()));
            return Err(AppError::Other(err));
        }
    }
    if let Err(e) = atomic_replace(&staged_dll, &dll_path) {
        emit(
            "failed",
            "Replace failed, rolling back",
            None,
            Some(e.to_string()),
        );
        if let Err(roll) = std::fs::copy(&backup_path, &dll_path) {
            emit(
                "failed",
                "Rollback also failed",
                None,
                Some(roll.to_string()),
            );
        }
        return Err(AppError::Other(format!("atomic replace failed: {e}")));
    }
    emit("replace", "Installed", None, None);
    drop(staging);

    emit("verify_post", "Reading new DLL version", None, None);
    let new_version = tokio::task::spawn_blocking({
        let dll_path = dll_path.clone();
        move || pe_version::read_dll_version(&dll_path).ok()
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
    .map(|v| v.file_version)
    .unwrap_or_else(|| release.version.clone());
    emit(
        "verify_post",
        &format!("Installed version: {new_version}"),
        None,
        None,
    );

    emit(
        "complete",
        &format!("Updated {} to v{}", filename, new_version),
        Some(1.0),
        None,
    );

    Ok(ApplyResult {
        apply_id,
        backup_id: entry_id,
        previous_version,
        new_version,
    })
}

/// Opens the file with write+read sharing to see if any other process holds it
/// exclusively. ERROR_SHARING_VIOLATION (32) is the smoking gun of "game
/// process has this DLL mapped".
fn ensure_writable(path: &std::path::Path) -> Result<(), String> {
    use std::fs::OpenOptions;
    match OpenOptions::new().read(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            let code = e.raw_os_error().unwrap_or(0);
            if code == 32 || code == 33 {
                Err(format!(
                    "file is locked by another process ({})",
                    path.display()
                ))
            } else if code == 5 {
                Err(format!(
                    "access denied to {} (try running as administrator)",
                    path.display()
                ))
            } else {
                Err(format!("cannot open {} for writing: {}", path.display(), e))
            }
        }
    }
}

fn atomic_replace(source: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
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

fn enrich_signature_error(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    let no_match = lower.contains("crypt_e_no_match")
        || lower.contains("notsigned")
        || lower.contains("not_signed")
        || lower.contains("could not be read")
        || lower.contains("no authenticode subject");
    if no_match {
        format!(
            "{reason}\n\nHint: this DLL ships unsigned by the vendor. \
             Enable 'Allow unsigned DLLs' in Settings → Advanced to override (SHA-256 \
             integrity is still enforced)."
        )
    } else {
        reason.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrich_appends_hint_on_crypt_no_match() {
        let out = enrich_signature_error("CryptQueryObject: 0x80092009 (CRYPT_E_NO_MATCH)");
        assert!(out.contains("Allow unsigned DLLs"));
        assert!(out.contains("CRYPT_E_NO_MATCH"));
    }

    #[test]
    fn enrich_appends_hint_on_no_subject() {
        let out = enrich_signature_error("no Authenticode subject extracted (status: NotSigned)");
        assert!(out.contains("Allow unsigned DLLs"));
    }

    #[test]
    fn enrich_leaves_subject_allowlist_errors_alone() {
        let original = "Authenticode subject 'WrongCorp' not in nvidia allowlist";
        let out = enrich_signature_error(original);
        assert_eq!(out, original);
    }
}
