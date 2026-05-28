use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::system_info::GpuVendor;
use nvapi_drs::settings::RESETTABLE_IDS;
use nvapi_drs::{DlssOverrideConfig, DrsSetting, OverrideScope};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

const EXE_WALK_MAX_DEPTH: usize = 4;
const EXE_SKIP_TOKENS: &[&str] = &[
    "redist",
    "vcredist",
    "directx",
    "crashpad",
    "crashreport",
    "unins",
    "setup",
    "dotnet",
    "commonredist",
    "easyanticheat",
    "battleye",
    "launcher",
    "touchup",
];

#[derive(Debug, Clone, Serialize)]
pub struct DlssSettingValue {
    pub id: u32,
    pub value: Option<u32>,
}

#[cfg(windows)]
fn run_apply(scope: &OverrideScope, settings: &[DrsSetting]) -> Result<(), String> {
    nvapi_drs::ffi::apply_overrides(scope, settings)
}

#[cfg(windows)]
fn run_reset(scope: &OverrideScope, ids: &[u32]) -> Result<(), String> {
    nvapi_drs::ffi::reset_overrides(scope, ids)
}

#[cfg(windows)]
fn run_read(scope: &OverrideScope, ids: &[u32]) -> Result<Vec<(u32, Option<u32>)>, String> {
    nvapi_drs::ffi::read_overrides(scope, ids)
}

#[cfg(not(windows))]
fn run_apply(_scope: &OverrideScope, _settings: &[DrsSetting]) -> Result<(), String> {
    Err("DLSS overrides require Windows with an NVIDIA driver".to_string())
}

#[cfg(not(windows))]
fn run_reset(_scope: &OverrideScope, _ids: &[u32]) -> Result<(), String> {
    Err("DLSS overrides require Windows with an NVIDIA driver".to_string())
}

#[cfg(not(windows))]
fn run_read(_scope: &OverrideScope, _ids: &[u32]) -> Result<Vec<(u32, Option<u32>)>, String> {
    Err("DLSS overrides require Windows with an NVIDIA driver".to_string())
}

#[tauri::command]
pub async fn dlss_overrides_supported(state: State<'_, AppState>) -> AppResult<bool> {
    let info = crate::commands::drivers::ensure_system_info(&state).await?;
    Ok(info.gpus.iter().any(|gpu| gpu.vendor == GpuVendor::Nvidia))
}

#[tauri::command]
pub async fn apply_dlss_override(
    scope: OverrideScope,
    config: DlssOverrideConfig,
) -> AppResult<()> {
    let settings = config.to_drs_settings();
    tokio::task::spawn_blocking(move || run_apply(&scope, &settings))
        .await
        .map_err(|e| AppError::Other(format!("dlss apply task: {e}")))?
        .map_err(AppError::Other)
}

#[tauri::command]
pub async fn reset_dlss_override(scope: OverrideScope) -> AppResult<()> {
    tokio::task::spawn_blocking(move || run_reset(&scope, RESETTABLE_IDS))
        .await
        .map_err(|e| AppError::Other(format!("dlss reset task: {e}")))?
        .map_err(AppError::Other)
}

#[tauri::command]
pub async fn read_dlss_override(scope: OverrideScope) -> AppResult<Vec<DlssSettingValue>> {
    let ids = RESETTABLE_IDS.to_vec();
    let raw = tokio::task::spawn_blocking(move || run_read(&scope, &ids))
        .await
        .map_err(|e| AppError::Other(format!("dlss read task: {e}")))?
        .map_err(AppError::Other)?;
    Ok(raw
        .into_iter()
        .map(|(id, value)| DlssSettingValue { id, value })
        .collect())
}

fn collect_executables(dir: &Path, depth: usize, out: &mut Vec<(u64, PathBuf)>) {
    if depth > EXE_WALK_MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if EXE_SKIP_TOKENS.iter().any(|token| name.contains(token)) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let path = entry.path();
        if meta.is_dir() {
            collect_executables(&path, depth + 1, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            out.push((meta.len(), path));
        }
    }
}

#[tauri::command]
pub async fn find_game_executable(install_dir: String) -> AppResult<Option<String>> {
    let resolved = tokio::task::spawn_blocking(move || {
        let mut executables = Vec::new();
        collect_executables(Path::new(&install_dir), 0, &mut executables);
        executables
            .into_iter()
            .max_by_key(|(size, _)| *size)
            .map(|(_, path)| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| AppError::Other(format!("executable scan: {e}")))?;
    Ok(resolved)
}
