use crate::commands::apply::{
    apply_single_item, lookup_release, streamline_guard, ApplyOutcome, ApplyRequest, StateHandles,
};
use crate::error::AppResult;
use crate::state::AppState;
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
    if items.is_empty() {
        return Ok(StreamlineSetResult {
            success: true,
            applied: vec![],
            error: None,
            rolled_back: false,
        });
    }
    let handles = state.inner().clone_handles();

    let mut cdn_urls = Vec::with_capacity(items.len());
    for item in &items {
        cdn_urls.push(lookup_release(&handles, item).await?.cdn_url);
    }
    if let Some(reason) = coherence_error(&cdn_urls) {
        return Ok(set_failure(vec![], &reason, false));
    }

    for item in &items {
        let dll_path = PathBuf::from(&item.dll_path);
        if let Some(reason) = streamline_guard(&handles, &dll_path, &item.target_version) {
            return Ok(set_failure(vec![], &reason, false));
        }
    }

    let registry = state.apply_registry.clone();
    let mut applied: Vec<ApplyOutcome> = Vec::with_capacity(items.len());
    let mut applied_backup_ids: Vec<String> = Vec::new();
    for item in &items {
        let cancel = registry.register(&item.apply_id);
        let outcome = apply_single_item(&handle, &handles, item, cancel).await?;
        registry.release(&item.apply_id);
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
                .unwrap_or_else(|| "Streamline set member failed".to_string());
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
                if std::fs::copy(&entry.backup_path, &entry.original_path).is_err() {
                    all_restored = false;
                }
            }
            Err(_) => all_restored = false,
        }
    }
    all_restored
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
}
