use dlssync_application::product_config;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeMode {
    pub portable: bool,
    pub release_url: String,
}

fn detect_portable(marker_filename: &str) -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(marker_filename)))
        .map(|m| m.exists())
        .unwrap_or(false)
}

pub fn devtools_allowed() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
pub fn runtime_mode() -> RuntimeMode {
    let config = product_config().expect("embedded product.toml must be valid");
    RuntimeMode {
        portable: detect_portable(&config.distribution.portable.data_marker),
        release_url: config.links.releases_latest,
    }
}

#[tauri::command]
pub fn open_devtools(window: tauri::WebviewWindow) {
    if !devtools_allowed() {
        return;
    }
    // The `devtools` Tauri feature is no longer compiled into release builds, so
    // the open/close/is-open methods only exist under `debug_assertions` (Tauri
    // auto-enables devtools in debug). Gating the calls keeps release builds free
    // of the devtools surface while debug builds keep the inspector.
    #[cfg(debug_assertions)]
    {
        if window.is_devtools_open() {
            window.close_devtools();
        } else {
            window.open_devtools();
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = window;
}

#[cfg(test)]
mod tests {
    use super::devtools_allowed;

    #[test]
    fn devtools_gate_tracks_debug_assertions() {
        assert_eq!(devtools_allowed(), cfg!(debug_assertions));
    }
}
