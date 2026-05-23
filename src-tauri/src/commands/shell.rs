use crate::error::{AppError, AppResult};
use std::path::PathBuf;

#[tauri::command]
pub async fn open_path(path: String) -> AppResult<()> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(AppError::Other(format!("path does not exist: {path}")));
    }
    spawn_explorer(&[p.as_os_str()])
}

#[tauri::command]
pub async fn reveal_path(path: String) -> AppResult<()> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(AppError::Other(format!("path does not exist: {path}")));
    }
    let arg = format!("/select,{}", p.display());
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&arg)
            .spawn()
            .map(|_| ())
            .map_err(|e| AppError::Other(format!("explorer reveal: {e}")))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = arg;
        Err(AppError::Other(
            "reveal_path is only implemented on Windows".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
fn spawn_explorer(args: &[&std::ffi::OsStr]) -> AppResult<()> {
    std::process::Command::new("explorer.exe")
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Other(format!("explorer open: {e}")))
}

#[cfg(not(target_os = "windows"))]
fn spawn_explorer(_args: &[&std::ffi::OsStr]) -> AppResult<()> {
    Err(AppError::Other(
        "open_path is only implemented on Windows".to_string(),
    ))
}
