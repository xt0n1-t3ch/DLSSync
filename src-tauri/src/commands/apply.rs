use crate::commands::apply_decisions::{
    classify_error, enrich_signature_error, failure_outcome, group_id_for, streamline_block_reason,
    version_major,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use backup_store::BackupEntry;
use dll_catalog::{DownloadOptions, DownloadProgress, Release};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

pub const EVENT_APPLY_PROGRESS: &str = "apply_progress";
pub const EVENT_DOWNLOAD_PROGRESS: &str = "download_progress";
pub const EVENT_APPLY_INFLIGHT: &str = "apply_inflight";

pub const STAGE_DOWNLOAD: &str = "download";
pub const STAGE_VERIFY_SHA: &str = "verify_sha";
pub const STAGE_VERIFY_SIGNATURE: &str = "verify_signature";
pub const STAGE_BACKUP: &str = "backup";
pub const STAGE_REPLACE: &str = "replace";
pub const STAGE_VERIFY_POST: &str = "verify_post";
pub const STAGE_COMPLETE: &str = "complete";
pub const STAGE_FAILED: &str = "failed";
pub const STAGE_CANCELLED: &str = "cancelled";

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize)]
pub struct ApplyBatchRequest {
    pub items: Vec<ApplyRequest>,
}

#[derive(Debug, Serialize)]
pub struct ApplyResult {
    pub apply_id: String,
    pub backup_id: String,
    pub previous_version: Option<String>,
    pub new_version: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyBatchResult {
    pub outcomes: Vec<ApplyOutcome>,
}

#[derive(Debug, Serialize)]
pub struct ApplyOutcome {
    pub apply_id: String,
    pub success: bool,
    pub backup_id: Option<String>,
    pub previous_version: Option<String>,
    pub new_version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyProgress {
    pub apply_id: String,
    pub group_id: String,
    pub stage: String,
    pub message: String,
    pub progress: Option<f64>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupDownloadProgress {
    pub group_id: String,
    pub url: String,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_sec: f64,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct InflightSnapshot {
    pub in_flight: usize,
}

#[tauri::command]
pub async fn apply_update(
    handle: AppHandle,
    state: State<'_, AppState>,
    request: ApplyRequest,
) -> AppResult<ApplyResult> {
    let registry = state.apply_registry.clone();
    let cancel = registry.register(&request.apply_id);
    emit_inflight(&handle, registry.in_flight());
    let handles = state.inner().clone_handles();
    let result = apply_single_item(&handle, &handles, &request, cancel).await;
    registry.release(&request.apply_id);
    emit_inflight(&handle, registry.in_flight());
    match result {
        Ok(outcome) if outcome.success => Ok(ApplyResult {
            apply_id: outcome.apply_id,
            backup_id: outcome
                .backup_id
                .ok_or_else(|| AppError::Other("missing backup id on success".into()))?,
            previous_version: outcome.previous_version,
            new_version: outcome
                .new_version
                .ok_or_else(|| AppError::Other("missing new version on success".into()))?,
        }),
        Ok(outcome) => Err(AppError::Other(
            outcome.error.unwrap_or_else(|| "apply failed".into()),
        )),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn apply_update_batch(
    handle: AppHandle,
    state: State<'_, AppState>,
    request: ApplyBatchRequest,
) -> AppResult<ApplyBatchResult> {
    if request.items.is_empty() {
        return Ok(ApplyBatchResult { outcomes: vec![] });
    }
    let handles_for_lookup = state.inner().clone_handles();
    let mut by_group: HashMap<String, Vec<ApplyRequest>> = HashMap::new();
    for item in &request.items {
        let release = lookup_release(&handles_for_lookup, item).await?;
        let gid = group_id_for(&release.cdn_url);
        by_group.entry(gid).or_default().push(item.clone());
    }
    let registry = state.apply_registry.clone();
    let tokens: HashMap<String, CancellationToken> = request
        .items
        .iter()
        .map(|r| (r.apply_id.clone(), registry.register(&r.apply_id)))
        .collect();
    emit_inflight(&handle, registry.in_flight());

    let concurrency = state.settings.read().effective_apply_concurrency() as usize;
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut group_tasks = Vec::with_capacity(by_group.len());
    for (_gid, items) in by_group.into_iter() {
        let handle_c = handle.clone();
        let state_c = state.inner().clone_handles();
        let sem = semaphore.clone();
        let tokens_c = tokens.clone();
        let apply_ids_for_group: Vec<String> = items.iter().map(|r| r.apply_id.clone()).collect();
        let task = tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.ok();
            let mut group_outcomes = Vec::with_capacity(items.len());
            for item in items {
                let token = tokens_c
                    .get(&item.apply_id)
                    .cloned()
                    .unwrap_or_else(CancellationToken::new);
                let outcome = apply_single_item(&handle_c, &state_c, &item, token).await;
                group_outcomes.push((item.apply_id.clone(), outcome));
            }
            group_outcomes
        });
        group_tasks.push((apply_ids_for_group, task));
    }
    let mut outcomes = Vec::with_capacity(request.items.len());
    for (group_apply_ids, task) in group_tasks {
        match task.await {
            Ok(group_outcomes) => {
                for (apply_id, o) in group_outcomes {
                    match o {
                        Ok(out) => outcomes.push(out),
                        Err(e) => outcomes.push(ApplyOutcome {
                            apply_id,
                            success: false,
                            backup_id: None,
                            previous_version: None,
                            new_version: None,
                            error: Some(e.to_string()),
                        }),
                    }
                }
            }
            Err(join_err) => {
                for apply_id in group_apply_ids {
                    outcomes.push(ApplyOutcome {
                        apply_id,
                        success: false,
                        backup_id: None,
                        previous_version: None,
                        new_version: None,
                        error: Some(format!("task join failed: {join_err}")),
                    });
                }
            }
        }
    }
    for r in &request.items {
        registry.release(&r.apply_id);
    }
    emit_inflight(&handle, registry.in_flight());
    Ok(ApplyBatchResult { outcomes })
}

#[tauri::command]
pub async fn cancel_apply(state: State<'_, AppState>, apply_id: String) -> AppResult<bool> {
    Ok(state.apply_registry.cancel(&apply_id))
}

#[tauri::command]
pub async fn cancel_all_applies(state: State<'_, AppState>) -> AppResult<usize> {
    Ok(state.apply_registry.cancel_all())
}

pub(crate) async fn lookup_release(
    state: &StateHandles,
    request: &ApplyRequest,
) -> AppResult<Release> {
    let guard = state.catalog.read();
    let catalog = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("catalog not loaded".into()))?;
    let filename = std::path::Path::new(&request.dll_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    catalog
        .find_file(
            &request.vendor,
            &request.family,
            &request.target_version,
            filename,
        )
        .or_else(|| catalog.find(&request.vendor, &request.family, &request.target_version))
        .ok_or_else(|| {
            AppError::Other(format!(
                "release {}::{}::{} not in catalog",
                request.vendor, request.family, request.target_version
            ))
        })
}

pub(crate) async fn apply_single_item(
    handle: &AppHandle,
    state: &StateHandles,
    request: &ApplyRequest,
    cancel: CancellationToken,
) -> AppResult<ApplyOutcome> {
    let release = match lookup_release(state, request).await {
        Ok(r) => r,
        Err(e) => return Ok(failure_outcome(request, "_", e.to_string())),
    };
    let group_id = group_id_for(&release.cdn_url);
    let ctx = StageContext {
        handle: handle.clone(),
        apply_id: request.apply_id.clone(),
        group_id: group_id.clone(),
    };

    let dll_path = PathBuf::from(&request.dll_path);
    if !dll_path.exists() {
        ctx.fail(
            "DLL file disappeared",
            "missing".to_string(),
            Some("missing"),
        );
        return Ok(failure_outcome(
            request,
            &group_id,
            format!("dll not found: {}", dll_path.display()),
        ));
    }

    if let Some(reason) = streamline_guard(state, &dll_path, &request.target_version) {
        ctx.fail(
            "Streamline plugin skipped",
            reason.clone(),
            Some("streamline_locked"),
        );
        return Ok(failure_outcome(request, &group_id, reason));
    }
    if let Err(reason) = ensure_writable(&dll_path) {
        let class = if reason.contains("locked") {
            "lock"
        } else if reason.contains("access denied") {
            "permission"
        } else {
            "other"
        };
        ctx.fail("DLL is locked", reason.clone(), Some(class));
        return Ok(failure_outcome(request, &group_id, reason));
    }

    let backup_root = match state.backups.read().as_ref().map(|s| s.root_dir.clone()) {
        Some(p) => p,
        None => {
            let err = "backup store not initialized".to_string();
            ctx.fail(&err, err.clone(), Some("backup"));
            return Ok(failure_outcome(request, &group_id, err));
        }
    };

    ctx.stage(
        STAGE_DOWNLOAD,
        &format!("Downloading {} v{}", release.filename, release.version),
        Some(0.0),
        None,
    );
    let staging = match tempfile::tempdir_in(&backup_root) {
        Ok(t) => t,
        Err(e) => {
            ctx.fail("Staging dir failed", e.to_string(), Some("other"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
    };
    let staged_dll =
        match stage_download(state, &release, staging.path(), &ctx, cancel.clone()).await {
            Ok(p) => p,
            Err(err) => {
                let class = classify_error(&err);
                ctx.fail("Download failed", err.clone(), Some(class));
                return Ok(failure_outcome(request, &group_id, err));
            }
        };

    let algo = dll_catalog::HashAlgo::from_hex_len(&release.sha256)
        .unwrap_or(dll_catalog::HashAlgo::Sha256);
    let algo_label = match algo {
        dll_catalog::HashAlgo::Sha256 => "SHA-256",
        dll_catalog::HashAlgo::Md5 => "MD5",
    };
    ctx.stage(
        STAGE_VERIFY_SHA,
        &format!("Verifying {algo_label}"),
        None,
        None,
    );
    let new_hash = match tokio::task::spawn_blocking({
        let staged = staged_dll.clone();
        move || dll_catalog::hash_file_with(&staged, algo)
    })
    .await
    {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => {
            ctx.fail("Hash failed", e.to_string(), Some("hash"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
        Err(e) => {
            ctx.fail("Hash task failed", e.to_string(), Some("other"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
    };
    if !new_hash.eq_ignore_ascii_case(&release.sha256) {
        let err = format!(
            "{algo_label} mismatch: expected {} got {}",
            release.sha256, new_hash
        );
        ctx.fail("Integrity check failed", err.clone(), Some("hash"));
        return Ok(failure_outcome(request, &group_id, err));
    }
    ctx.stage(STAGE_VERIFY_SHA, &format!("{algo_label} OK"), None, None);

    let allow_unsigned = state.settings.read().advanced.allow_unsigned_dlls;
    ctx.stage(
        STAGE_VERIFY_SIGNATURE,
        "Reading Authenticode signature",
        None,
        None,
    );
    let auth_info = match tokio::task::spawn_blocking({
        let staged = staged_dll.clone();
        move || pe_version::read_authenticode(&staged)
    })
    .await
    {
        Ok(info) => info,
        Err(e) => {
            ctx.fail("Signature task failed", e.to_string(), Some("other"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
    };
    match auth_info {
        Some(info) => match pe_version::enforce_subject(&info, &request.vendor) {
            Ok(()) => {
                let trust_tag = if info.trusted {
                    "trusted"
                } else {
                    "untrusted-chain"
                };
                ctx.stage(
                    STAGE_VERIFY_SIGNATURE,
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
                ctx.stage(
                    STAGE_VERIFY_SIGNATURE,
                    &format!("Signature mismatch ignored: {reason}"),
                    None,
                    None,
                );
            }
            Err(reason) => {
                let with_hint = enrich_signature_error(&reason);
                ctx.fail("Signature rejected", with_hint.clone(), Some("signature"));
                return Ok(failure_outcome(request, &group_id, with_hint));
            }
        },
        None if allow_unsigned => {
            ctx.stage(
                STAGE_VERIFY_SIGNATURE,
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
            ctx.fail("Signature unreadable", err.clone(), Some("signature"));
            return Ok(failure_outcome(request, &group_id, err));
        }
    }

    if cancel.is_cancelled() {
        ctx.cancelled();
        return Ok(failure_outcome(request, &group_id, "cancelled".into()));
    }

    let previous_sha = match tokio::task::spawn_blocking({
        let dll_path = dll_path.clone();
        move || dll_catalog::hex_sha256_file(&dll_path)
    })
    .await
    {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => {
            ctx.fail("Hash old DLL failed", e.to_string(), Some("hash"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
        Err(e) => {
            ctx.fail("Hash task failed", e.to_string(), Some("other"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
    };
    let previous_version = tokio::task::spawn_blocking({
        let dll_path = dll_path.clone();
        move || pe_version::read_dll_version(&dll_path).ok()
    })
    .await
    .ok()
    .flatten()
    .map(|v| v.file_version);

    ctx.stage(STAGE_BACKUP, "Backing up current DLL", None, None);
    let entry_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now();
    let filename = dll_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.dll")
        .to_string();
    let backup_path = match state.backups.read().as_ref().and_then(|store| {
        let label = request
            .game_label
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&request.game_id);
        store
            .allocate_backup_path(label, created_at, &filename)
            .ok()
    }) {
        Some(p) => p,
        None => {
            let err = "could not allocate backup path".to_string();
            ctx.fail(&err, err.clone(), Some("backup"));
            return Ok(failure_outcome(request, &group_id, err));
        }
    };
    if cancel.is_cancelled() {
        ctx.cancelled();
        return Ok(failure_outcome(request, &group_id, "cancelled".into()));
    }
    let copy_src = dll_path.clone();
    let copy_dst = backup_path.clone();
    match tokio::task::spawn_blocking(move || std::fs::copy(&copy_src, &copy_dst)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            ctx.fail("Backup copy failed", e.to_string(), Some("backup"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
        Err(e) => {
            ctx.fail("Backup copy task failed", e.to_string(), Some("other"));
            return Ok(failure_outcome(request, &group_id, e.to_string()));
        }
    }
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
        backup_type: "dll".to_string(),
        device_class: None,
        hardware_id: None,
        driver_provider: None,
    };
    if let Err(e) = state
        .backups
        .read()
        .as_ref()
        .map(|store| store.insert(&entry))
        .transpose()
    {
        ctx.fail("Backup insert failed", e.to_string(), Some("backup"));
        return Ok(failure_outcome(request, &group_id, e.to_string()));
    }
    ctx.stage(STAGE_BACKUP, "Backup created", None, None);

    ctx.stage(STAGE_REPLACE, "Installing new DLL", None, None);
    if let Ok(meta) = std::fs::symlink_metadata(&dll_path) {
        if meta.file_type().is_symlink() {
            let err = format!("refusing to replace symlink: {}", dll_path.display());
            ctx.fail("Symlink detected", err.clone(), Some("permission"));
            return Ok(failure_outcome(request, &group_id, err));
        }
    }
    if cancel.is_cancelled() {
        ctx.cancelled();
        return Ok(failure_outcome(request, &group_id, "cancelled".into()));
    }
    if let Err(e) = atomic_replace(&staged_dll, &dll_path) {
        let os = e.raw_os_error().unwrap_or(0);
        let (class, msg) = if os == 32 || os == 33 {
            (
                "lock",
                format!(
                    "{filename} is locked — the game may still be running. Close it and retry."
                ),
            )
        } else {
            ("other", format!("atomic replace failed: {e}"))
        };
        let roll_src = backup_path.clone();
        let roll_dst = dll_path.clone();
        match tokio::task::spawn_blocking(move || std::fs::copy(&roll_src, &roll_dst)).await {
            Ok(Err(roll)) => {
                tracing::error!(error = %roll, "rollback after replace failure also failed");
            }
            Err(join_err) => {
                tracing::error!(error = %join_err, "rollback task after replace failure failed to join");
            }
            Ok(Ok(_)) => {}
        }
        ctx.fail("Replace failed", msg.clone(), Some(class));
        return Ok(failure_outcome(request, &group_id, msg));
    }
    ctx.stage(STAGE_REPLACE, "Installed", None, None);
    drop(staging);

    let post_algo = dll_catalog::HashAlgo::from_hex_len(&release.sha256)
        .unwrap_or(dll_catalog::HashAlgo::Sha256);
    let installed_hash = tokio::task::spawn_blocking({
        let dll_path = dll_path.clone();
        move || dll_catalog::hash_file_with(&dll_path, post_algo)
    })
    .await
    .ok()
    .and_then(|r| r.ok());
    if !installed_hash
        .as_deref()
        .is_some_and(|h| h.eq_ignore_ascii_case(&release.sha256))
    {
        let roll_src = backup_path.clone();
        let roll_dst = dll_path.clone();
        match tokio::task::spawn_blocking(move || std::fs::copy(&roll_src, &roll_dst)).await {
            Ok(Err(roll)) => {
                tracing::error!(error = %roll, "rollback after post-swap hash mismatch also failed");
            }
            Err(join_err) => {
                tracing::error!(error = %join_err, "rollback task after post-swap hash mismatch failed to join");
            }
            Ok(Ok(_)) => {}
        }
        let got = installed_hash.unwrap_or_else(|| "unreadable".to_string());
        let err = format!(
            "post-swap integrity check failed for {filename}: expected {} got {} — rolled back to \
             the previous DLL",
            release.sha256, got
        );
        ctx.fail("Post-swap verify failed", err.clone(), Some("hash"));
        return Ok(failure_outcome(request, &group_id, err));
    }

    ctx.stage(STAGE_VERIFY_POST, "Reading new DLL version", None, None);
    let new_version = match tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking({
            let dll_path = dll_path.clone();
            move || pe_version::read_dll_version(&dll_path).ok()
        }),
    )
    .await
    {
        Ok(Ok(Some(v))) => v.file_version,
        _ => release.version.clone(),
    };
    ctx.stage(
        STAGE_VERIFY_POST,
        &format!("Installed version: {new_version}"),
        None,
        None,
    );
    ctx.stage(
        STAGE_COMPLETE,
        &format!("Updated {} to v{}", filename, new_version),
        Some(1.0),
        None,
    );

    Ok(ApplyOutcome {
        apply_id: request.apply_id.clone(),
        success: true,
        backup_id: Some(entry_id),
        previous_version,
        new_version: Some(new_version),
        error: None,
    })
}

async fn stage_download(
    state: &StateHandles,
    release: &Release,
    staging_dir: &std::path::Path,
    ctx: &StageContext,
    cancel: CancellationToken,
) -> Result<PathBuf, String> {
    let (tx, mut rx) = mpsc::unbounded_channel::<DownloadProgress>();
    let net = state.settings.read().network.clone();
    let opts = DownloadOptions {
        max_retries: net.retry_attempts.max(1),
        chunk_timeout: Duration::from_secs(net.chunk_timeout_secs.max(5)),
        progress_tx: Some(tx),
        cancel: Some(cancel.clone()),
    };
    let pump_handle = ctx.handle.clone();
    let pump_group = ctx.group_id.clone();
    let pump_url = release.cdn_url.clone();
    let pump = tokio::spawn(async move {
        while let Some(p) = rx.recv().await {
            let _ = pump_handle.emit(
                EVENT_DOWNLOAD_PROGRESS,
                GroupDownloadProgress {
                    group_id: pump_group.clone(),
                    url: pump_url.clone(),
                    bytes_downloaded: p.bytes_downloaded,
                    bytes_total: p.bytes_total,
                    bytes_per_sec: p.bytes_per_sec,
                    attempt: p.attempt,
                },
            );
        }
    });

    let result = dll_catalog::download_and_extract_dll_cached(
        &state.download_cache,
        &state.http_downloads,
        release,
        staging_dir,
        opts,
    )
    .await;

    drop(pump);
    match result {
        Ok(path) => {
            ctx.stage(STAGE_DOWNLOAD, "Downloaded", Some(1.0), None);
            Ok(path)
        }
        Err(e) => Err(e.to_string()),
    }
}

struct StageContext {
    handle: AppHandle,
    apply_id: String,
    group_id: String,
}

impl StageContext {
    fn stage(&self, stage: &str, message: &str, progress: Option<f64>, attempt: Option<u32>) {
        let _ = self.handle.emit(
            EVENT_APPLY_PROGRESS,
            ApplyProgress {
                apply_id: self.apply_id.clone(),
                group_id: self.group_id.clone(),
                stage: stage.to_string(),
                message: message.to_string(),
                progress,
                error: None,
                error_class: None,
                attempt,
            },
        );
    }

    fn fail(&self, message: &str, error: String, error_class: Option<&str>) {
        let _ = self.handle.emit(
            EVENT_APPLY_PROGRESS,
            ApplyProgress {
                apply_id: self.apply_id.clone(),
                group_id: self.group_id.clone(),
                stage: STAGE_FAILED.to_string(),
                message: message.to_string(),
                progress: None,
                error: Some(error),
                error_class: error_class.map(|s| s.to_string()),
                attempt: None,
            },
        );
    }

    fn cancelled(&self) {
        let _ = self.handle.emit(
            EVENT_APPLY_PROGRESS,
            ApplyProgress {
                apply_id: self.apply_id.clone(),
                group_id: self.group_id.clone(),
                stage: STAGE_CANCELLED.to_string(),
                message: "Cancelled".to_string(),
                progress: None,
                error: Some("cancelled".to_string()),
                error_class: Some("cancelled".to_string()),
                attempt: None,
            },
        );
    }
}

fn emit_inflight(handle: &AppHandle, in_flight: usize) {
    let _ = handle.emit(EVENT_APPLY_INFLIGHT, InflightSnapshot { in_flight });
    crate::tray::update_inflight(handle, in_flight);
}

/// Authoritative crash-safety gate for the NVIDIA Streamline version-locked set.
/// Returns `Some(reason)` when an `sl.*` plugin must NOT be swapped: the user has
/// not opted in, or the target crosses the installed Streamline MAJOR (v1↔v2 is
/// the real cause of the launch crashes — older games ship SL 1.x and break when
/// newer 2.x plugins are swapped in). A same-major swap is allowed even when a
/// DLSS Enabler is present, because the Enabler requires Streamline >= 2.11 yet
/// does not update it, so the user must update the set themselves. NGX DLLs
/// (`nvngx_*.dll`) are never gated — the driver loads them independently and they
/// are safe to swap on their own.
pub(crate) fn streamline_guard(
    state: &StateHandles,
    dll_path: &std::path::Path,
    target_version: &str,
) -> Option<String> {
    let filename = dll_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if !dll_scanner::is_streamline_plugin(filename) {
        return None;
    }
    let allow_streamline = state.settings.read().update_prefs.update_streamline;
    let installed_major = pe_version::read_dll_version(dll_path)
        .ok()
        .and_then(|v| version_major(&v.file_version));
    let target_major = version_major(target_version);
    streamline_block_reason(filename, allow_streamline, installed_major, target_major)
}

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

pub(crate) struct StateHandles {
    pub catalog: Arc<parking_lot::RwLock<Option<dll_catalog::Catalog>>>,
    pub backups: Arc<parking_lot::RwLock<Option<backup_store::BackupStore>>>,
    pub settings: Arc<parking_lot::RwLock<crate::commands::settings::AppSettings>>,
    pub http_downloads: reqwest::Client,
    pub download_cache: Arc<dll_catalog::DownloadCache>,
}

impl AppState {
    pub(crate) fn clone_handles(&self) -> StateHandles {
        StateHandles {
            catalog: self.catalog.clone(),
            backups: self.backups.clone(),
            settings: self.settings.clone(),
            http_downloads: self.http_downloads.clone(),
            download_cache: self.download_cache.clone(),
        }
    }
}
