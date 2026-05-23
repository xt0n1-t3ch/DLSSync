use crate::constants::{GITHUB_LATEST_RELEASE_URL, PORTABLE_MARKER_FILENAME};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMode {
    pub portable: bool,
    pub release_url: String,
}

fn detect_portable() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(PORTABLE_MARKER_FILENAME)))
        .map(|m| m.exists())
        .unwrap_or(false)
}

#[tauri::command]
pub fn runtime_mode() -> RuntimeMode {
    RuntimeMode {
        portable: detect_portable(),
        release_url: GITHUB_LATEST_RELEASE_URL.to_string(),
    }
}
