use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// Windows file names cannot contain `"` or control characters. Rejecting them
/// keeps a webview-supplied path from breaking out of the `/select,"<path>"`
/// argument or smuggling shell metacharacters into Explorer.
fn reject_unsafe_path(path: &str) -> AppResult<()> {
    if path.contains('"') || path.chars().any(|c| c.is_control()) {
        return Err(AppError::Validation(format!(
            "refusing to open path with illegal characters: {path:?}"
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn open_path(path: String) -> AppResult<()> {
    reject_unsafe_path(&path)?;
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(AppError::Validation(format!(
            "open_path only opens directories: {path}"
        )));
    }
    spawn_explorer(&[p.as_os_str()])
}

#[tauri::command]
pub async fn reveal_path(path: String) -> AppResult<()> {
    reject_unsafe_path(&path)?;
    let p = PathBuf::from(&path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if p.exists() {
            return std::process::Command::new("explorer.exe")
                .raw_arg(select_arg(&p))
                .spawn()
                .map(|_| ())
                .map_err(|e| AppError::Other(format!("explorer reveal: {e}")));
        }
        if let Some(parent) = p.parent().filter(|d| d.exists()) {
            return spawn_explorer(&[parent.as_os_str()]);
        }
        Err(AppError::Other(format!(
            "snapshot no longer on disk: {path}"
        )))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::Other(
            "reveal_path is only implemented on Windows".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
fn select_arg(p: &std::path::Path) -> String {
    format!("/select,\"{}\"", p.display())
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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::select_arg;
    use std::path::Path;

    #[test]
    fn select_arg_quotes_only_the_path_not_the_flag() {
        let arg = select_arg(Path::new(
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Game\\nvngx_dlss.dll",
        ));
        assert!(arg.starts_with("/select,\""));
        assert!(arg.ends_with("nvngx_dlss.dll\""));
        assert!(!arg.starts_with("\""));
    }

    #[test]
    fn select_arg_handles_path_without_spaces() {
        let arg = select_arg(Path::new("D:\\b\\s.dll"));
        assert_eq!(arg, "/select,\"D:\\b\\s.dll\"");
    }
}
