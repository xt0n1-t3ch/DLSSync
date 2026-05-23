use crate::error::AppResult;
use crate::paths::AppPaths;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LauncherOverrides {
    #[serde(default)]
    pub steam: Vec<String>,
    #[serde(default)]
    pub epic: Vec<String>,
    #[serde(default)]
    pub gog: Vec<String>,
    #[serde(default)]
    pub ubisoft: Vec<String>,
    #[serde(default)]
    pub ea_desktop: Vec<String>,
    #[serde(default)]
    pub xbox: Vec<String>,
    #[serde(default)]
    pub battlenet: Vec<String>,
    #[serde(default)]
    pub custom: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferences {
    pub update_dlss: bool,
    pub update_dlss_fg: bool,
    pub update_dlss_rr: bool,
    pub update_streamline: bool,
    pub update_reflex: bool,
    pub update_xess: bool,
    pub update_fsr: bool,
    pub update_direct_storage: bool,
    pub create_backups: bool,
    pub auto_apply_all_on_rescan: bool,
}

impl Default for UpdatePreferences {
    fn default() -> Self {
        Self {
            update_dlss: true,
            update_dlss_fg: true,
            update_dlss_rr: true,
            update_streamline: false,
            update_reflex: true,
            update_xess: true,
            update_fsr: true,
            update_direct_storage: true,
            create_backups: true,
            auto_apply_all_on_rescan: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    pub theme: String,
    pub sidebar_collapsed: bool,
    pub grid_density: String,
    pub sort_order: String,
    pub launcher_filter: String,
    pub status_filter: String,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            sidebar_collapsed: false,
            grid_density: "comfortable".into(),
            sort_order: "outdated_first".into(),
            launcher_filter: "all".into(),
            status_filter: "all".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SteamApiConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub steam_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SgdbConfig {
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowState {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub top: Option<f64>,
    pub left: Option<f64>,
    pub maximized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GamePreference {
    #[serde(default)]
    pub disabled_families: Vec<String>,
    #[serde(default)]
    pub pinned_versions: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    #[serde(default)]
    pub dlss_debug_overlay: bool,
    #[serde(default)]
    pub verbose_logs: bool,
    #[serde(default)]
    pub allow_unsigned_dlls: bool,
    #[serde(default = "default_true")]
    pub prefer_stable_channel: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            dlss_debug_overlay: false,
            verbose_logs: false,
            allow_unsigned_dlls: false,
            prefer_stable_channel: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub launcher_overrides: LauncherOverrides,
    #[serde(default)]
    pub update_prefs: UpdatePreferences,
    #[serde(default)]
    pub ui_prefs: UiPreferences,
    #[serde(default)]
    pub steam_api: SteamApiConfig,
    #[serde(default)]
    pub steamgriddb: SgdbConfig,
    #[serde(default)]
    pub window_state: WindowState,
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default)]
    pub ignored: Vec<String>,
    #[serde(default)]
    pub game_preferences: std::collections::HashMap<String, GamePreference>,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

fn settings_path_from(paths: &AppPaths) -> AppResult<PathBuf> {
    std::fs::create_dir_all(&paths.settings_dir)?;
    Ok(paths.settings_file.clone())
}

fn settings_path_from_state(state: &AppState) -> AppResult<PathBuf> {
    let guard = state.paths.read();
    let paths = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Other("app paths not initialized".into()))?;
    settings_path_from(paths)
}

pub fn load_initial(paths: &AppPaths) -> AppSettings {
    match settings_path_from(paths).and_then(|p| {
        if !p.exists() {
            return Ok(AppSettings::default());
        }
        let bytes = std::fs::read(&p)?;
        serde_json::from_slice::<AppSettings>(&bytes)
            .map_err(|e| crate::error::AppError::Other(format!("settings parse: {e}")))
    }) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "settings load failed, using defaults");
            AppSettings::default()
        }
    }
}

fn persist(state: &AppState, settings: &AppSettings) -> AppResult<()> {
    let path = settings_path_from_state(state)?;
    let body = serde_json::to_vec_pretty(settings)
        .map_err(|e| crate::error::AppError::Other(format!("settings serialize: {e}")))?;
    std::fs::write(path, body)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct AppPathsDto {
    pub root: String,
    pub backups_dir: String,
    pub cache_dir: String,
    pub logs_dir: String,
    pub settings_dir: String,
    pub backups_db: String,
    pub catalog_cache: String,
    pub settings_file: String,
}

impl From<&AppPaths> for AppPathsDto {
    fn from(p: &AppPaths) -> Self {
        Self {
            root: p.root.to_string_lossy().into_owned(),
            backups_dir: p.backups_dir.to_string_lossy().into_owned(),
            cache_dir: p.cache_dir.to_string_lossy().into_owned(),
            logs_dir: p.logs_dir.to_string_lossy().into_owned(),
            settings_dir: p.settings_dir.to_string_lossy().into_owned(),
            backups_db: p.backups_db.to_string_lossy().into_owned(),
            catalog_cache: p.catalog_cache.to_string_lossy().into_owned(),
            settings_file: p.settings_file.to_string_lossy().into_owned(),
        }
    }
}

#[tauri::command]
pub async fn get_app_paths(state: State<'_, AppState>) -> AppResult<AppPathsDto> {
    let guard = state.paths.read();
    let paths = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Other("app paths not initialized".into()))?;
    Ok(AppPathsDto::from(paths))
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> AppResult<()> {
    persist(&state, &settings)?;
    *state.settings.write() = settings;
    Ok(())
}

#[tauri::command]
pub async fn add_blacklist_entry(
    state: State<'_, AppState>,
    game_id: String,
) -> AppResult<Vec<String>> {
    let mut guard = state.settings.write();
    if !guard.blacklist.iter().any(|g| g == &game_id) {
        guard.blacklist.push(game_id);
    }
    persist(&state, &guard)?;
    Ok(guard.blacklist.clone())
}

#[tauri::command]
pub async fn remove_blacklist_entry(
    state: State<'_, AppState>,
    game_id: String,
) -> AppResult<Vec<String>> {
    let mut guard = state.settings.write();
    guard.blacklist.retain(|g| g != &game_id);
    persist(&state, &guard)?;
    Ok(guard.blacklist.clone())
}

#[tauri::command]
pub async fn save_window_state(
    state: State<'_, AppState>,
    window_state: WindowState,
) -> AppResult<()> {
    let mut guard = state.settings.write();
    guard.window_state = window_state;
    persist(&state, &guard)?;
    Ok(())
}
