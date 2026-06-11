use crate::commands::apply::{
    apply_single_item, lookup_release, streamline_guard, ApplyOutcome, ApplyRequest, StateHandles,
};
use crate::error::AppResult;
use crate::state::AppState;
use crate::system_info::GpuInfo;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
pub struct StreamlineSetResult {
    pub success: bool,
    pub applied: Vec<ApplyOutcome>,
    pub error: Option<String>,
    pub rolled_back: bool,
}

/// Apply an NVIDIA Streamline plugin set as one all-or-nothing transaction. The
/// `sl.*` plugins are version-locked: every member must come from the same SDK
/// release, and a partially-swapped set crashes the game on launch. So this
/// rejects a mixed-version set, refuses to start when any member is blocked
/// (DLSS Enabler / opt-in off / cross-major), and on any member failure rolls
/// every already-swapped member back to its backup.
#[tauri::command]
pub async fn apply_streamline_set(
    handle: AppHandle,
    state: State<'_, AppState>,
    items: Vec<ApplyRequest>,
) -> AppResult<StreamlineSetResult> {
    let handles = state.inner().clone_handles();
    let guard_handles = state.inner().clone_handles();
    run_set(&handle, &state, handles, items, move |item| {
        let dll_path = PathBuf::from(&item.dll_path);
        streamline_guard(&guard_handles, &dll_path, &item.target_version)
    })
    .await
}

/// Apply any coherent multi-DLL family set (FSR loader/upscaler/frame-generation,
/// XeSS libxess/libxell/libxess_fg) as one all-or-nothing transaction. Members are
/// version-locked to ONE SDK release (the spike-verified FSR SDK 2.2.0 ships
/// loader 2.2.0 + upscaler 4.1.0 + frame-gen 4.0.0 in a single zip), and 4.x FSR
/// members are refused outright when no RDNA4 GPU is present.
#[tauri::command]
pub async fn apply_dll_set(
    handle: AppHandle,
    state: State<'_, AppState>,
    items: Vec<ApplyRequest>,
) -> AppResult<StreamlineSetResult> {
    let gpus = match crate::commands::drivers::ensure_system_info(&state).await {
        Ok(info) => info.gpus,
        Err(e) => {
            tracing::warn!(error = %e, "system info unavailable — FSR4 gate fails closed");
            Vec::new()
        }
    };
    let handles = state.inner().clone_handles();
    run_set(&handle, &state, handles, items, move |item| {
        fsr4_guard(&gpus, &item.family, &item.target_version)
    })
    .await
}

async fn run_set(
    handle: &AppHandle,
    state: &State<'_, AppState>,
    handles: StateHandles,
    items: Vec<ApplyRequest>,
    guard: impl Fn(&ApplyRequest) -> Option<String>,
) -> AppResult<StreamlineSetResult> {
    if items.is_empty() {
        return Ok(StreamlineSetResult {
            success: true,
            applied: vec![],
            error: None,
            rolled_back: false,
        });
    }

    let mut cdn_urls = Vec::with_capacity(items.len());
    for item in &items {
        cdn_urls.push(lookup_release(&handles, item).await?.cdn_url);
    }
    if let Some(reason) = coherence_error(&cdn_urls) {
        return Ok(set_failure(vec![], &reason, false));
    }

    for item in &items {
        if let Some(reason) = guard(item) {
            return Ok(set_failure(vec![], &reason, false));
        }
    }

    let registry = state.apply_registry.clone();
    let mut applied: Vec<ApplyOutcome> = Vec::with_capacity(items.len());
    let mut applied_backup_ids: Vec<String> = Vec::new();
    for item in &items {
        let cancel = registry.register(&item.apply_id);
        let outcome = apply_single_item(handle, &handles, item, cancel).await;
        // Release unconditionally — an early `?` here would otherwise leak the
        // apply_registry slot, permanently inflating the in-flight counter and
        // suppressing the background scheduler until restart.
        registry.release(&item.apply_id);
        let outcome = outcome?;
        if outcome.success {
            if let Some(backup_id) = &outcome.backup_id {
                applied_backup_ids.push(backup_id.clone());
            }
            applied.push(outcome);
        } else {
            let rolled_back = rollback_all(&handles, &applied_backup_ids);
            let reason = outcome
                .error
                .clone()
                .unwrap_or_else(|| "set member failed".to_string());
            applied.push(outcome);
            return Ok(set_failure(applied, &reason, rolled_back));
        }
    }

    Ok(StreamlineSetResult {
        success: true,
        applied,
        error: None,
        rolled_back: false,
    })
}

/// 4.x FSR binaries only run on RDNA4 silicon today; offering or applying them on
/// older AMD (or non-AMD) hardware breaks the game's upscaler outright. Fails
/// closed when the GPU inventory is empty/unknown.
fn fsr4_guard(gpus: &[GpuInfo], family: &str, target_version: &str) -> Option<String> {
    let fsr4_member = matches!(family, "fsr_upscaler" | "fsr_upscaler_vk" | "fsr_fg")
        && target_version
            .split('.')
            .next()
            .and_then(|major| major.parse::<u32>().ok())
            .is_some_and(|major| major >= 4);
    if !fsr4_member {
        return None;
    }
    if gpus.iter().any(|g| g.fsr4_capable) {
        return None;
    }
    Some(
        "FSR 4 requires an AMD RDNA4 GPU (Radeon RX 9000 series) and none was detected — \
         refusing the set. Pick a 3.1.x FSR release for this PC instead."
            .to_string(),
    )
}

/// A Streamline set is version-locked: every member must resolve to the same SDK
/// release (one `cdn_url`). Returns the rejection reason when the members span
/// more than one release, else `None`. Empty/single sets are trivially coherent.
fn coherence_error(cdn_urls: &[String]) -> Option<String> {
    let first = cdn_urls.first()?;
    if cdn_urls.iter().any(|url| url != first) {
        return Some(
            "Streamline set members resolve to different SDK releases — refusing a mixed-version \
             set (the plug-ins are version-locked)."
                .to_string(),
        );
    }
    None
}

fn set_failure(applied: Vec<ApplyOutcome>, error: &str, rolled_back: bool) -> StreamlineSetResult {
    StreamlineSetResult {
        success: false,
        applied,
        error: Some(error.to_string()),
        rolled_back,
    }
}

/// Restore every successfully-swapped member from its backup. Returns whether
/// all restores succeeded.
fn rollback_all(handles: &StateHandles, backup_ids: &[String]) -> bool {
    let guard = handles.backups.read();
    let Some(store) = guard.as_ref() else {
        return false;
    };
    let mut all_restored = true;
    for id in backup_ids {
        match store.get(id) {
            Ok(entry) => {
                if !backup_path_under_root(&entry.backup_path, &store.root_dir) {
                    tracing::warn!(
                        backup_id = %id,
                        backup_path = %entry.backup_path.display(),
                        root = %store.root_dir.display(),
                        "rollback skipped: backup path escapes the backup store root"
                    );
                    all_restored = false;
                    continue;
                }
                // Atomic restore (stage + rename), mirroring restore_backup, so a
                // crash/power-loss mid-rollback cannot leave a truncated DLL that
                // crashes the game on next launch.
                match crate::commands::backup::atomic_replace(
                    &entry.backup_path,
                    &entry.original_path,
                ) {
                    Ok(()) => {
                        if let Err(e) = store.mark_restored(id, chrono::Utc::now()) {
                            tracing::warn!(backup_id = %id, error = %e, "rollback restored file but failed to mark backup restored");
                        }
                    }
                    Err(_) => all_restored = false,
                }
            }
            Err(_) => all_restored = false,
        }
    }
    all_restored
}

/// A backup may only be restored from inside the backup store root. The DB-stored
/// `backup_path` is otherwise an unvalidated copy SOURCE, so a tampered row could
/// point the restore at an arbitrary file. `None`/non-`starts_with` paths are
/// rejected.
fn backup_path_under_root(backup_path: &std::path::Path, root: &std::path::Path) -> bool {
    crate::paths::PathGuard::assert_under_root(backup_path, root).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn coherent_set_has_no_error() {
        let zip = "https://github.com/NVIDIA-RTX/Streamline/releases/download/v2.11.1/sdk.zip";
        assert!(coherence_error(&urls(&[zip, zip, zip])).is_none());
        assert!(coherence_error(&urls(&[zip])).is_none());
        assert!(coherence_error(&[]).is_none());
    }

    #[test]
    fn mixed_version_set_is_rejected() {
        let v211 = "https://github.com/NVIDIA-RTX/Streamline/releases/download/v2.11.1/sdk.zip";
        let v210 = "https://github.com/NVIDIA-RTX/Streamline/releases/download/v2.10.3/sdk.zip";
        let reason = coherence_error(&urls(&[v211, v211, v210])).unwrap();
        assert!(reason.contains("version-locked"));
    }

    fn amd_gpu(fsr4_capable: bool) -> GpuInfo {
        GpuInfo {
            vendor: crate::system_info::GpuVendor::Amd,
            pci_vendor_id: 0x1002,
            pci_device_id: if fsr4_capable { 0x7550 } else { 0x744C },
            model: if fsr4_capable {
                "AMD Radeon RX 9070 XT".into()
            } else {
                "AMD Radeon RX 7900 XTX".into()
            },
            driver_version: "Unknown".into(),
            vram_bytes: 0,
            recommended_runtimes: vec![],
            is_dch: true,
            identifiable: true,
            fsr4_capable,
        }
    }

    #[test]
    fn fsr4_guard_blocks_4x_members_without_rdna4() {
        let gpus = [amd_gpu(false)];
        let reason = fsr4_guard(&gpus, "fsr_upscaler", "4.1.0.0").unwrap();
        assert!(reason.contains("RDNA4"));
        assert!(fsr4_guard(&gpus, "fsr_fg", "4.0.0.0").is_some());
        assert!(fsr4_guard(&[], "fsr_upscaler", "4.1.0.0").is_some());
    }

    #[test]
    fn fsr4_guard_allows_rdna4_and_non_4x_members() {
        let rdna4 = [amd_gpu(true)];
        assert!(fsr4_guard(&rdna4, "fsr_upscaler", "4.1.0.0").is_none());
        let older = [amd_gpu(false)];
        assert!(fsr4_guard(&older, "fsr_upscaler", "3.1.4.0").is_none());
        assert!(fsr4_guard(&older, "fsr_loader", "2.2.0.0").is_none());
        assert!(fsr4_guard(&older, "xess_sr", "3.0.1.0").is_none());
    }

    #[test]
    fn backup_path_inside_root_is_restorable() {
        let root = std::path::Path::new("/data/DLSSync/Backups");
        let inside = std::path::Path::new("/data/DLSSync/Backups/Cyberpunk/2026/sl.dlss_g.dll");
        assert!(backup_path_under_root(inside, root));
    }

    #[test]
    fn backup_path_outside_root_is_rejected() {
        let root = std::path::Path::new("/data/DLSSync/Backups");
        assert!(!backup_path_under_root(
            std::path::Path::new("/etc/passwd"),
            root
        ));
        assert!(!backup_path_under_root(
            std::path::Path::new("/data/DLSSync/Other/sl.dlss_g.dll"),
            root
        ));
        assert!(!backup_path_under_root(
            std::path::Path::new("/data/DLSSync/BackupsEvil/x.dll"),
            root
        ));
    }
}
