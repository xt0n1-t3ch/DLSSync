use crate::error::{AppError, AppResult};

#[cfg(windows)]
#[tauri::command]
pub async fn set_dlss_debug_overlay(enabled: bool) -> AppResult<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _disp) = hkcu
        .create_subkey("SOFTWARE\\NVIDIA Corporation\\Global\\NGXCore")
        .map_err(|e| AppError::Other(format!("registry create: {e}")))?;
    if enabled {
        key.set_value("ShowDlssIndicator", &1024u32)
            .map_err(|e| AppError::Other(format!("registry write: {e}")))?;
    } else {
        let _ = key.delete_value("ShowDlssIndicator");
    }
    Ok(())
}

#[cfg(windows)]
#[tauri::command]
pub async fn get_dlss_debug_overlay() -> AppResult<bool> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey("SOFTWARE\\NVIDIA Corporation\\Global\\NGXCore") {
        Ok(key) => match key.get_value::<u32, _>("ShowDlssIndicator") {
            Ok(v) => Ok(v == 1024),
            Err(_) => Ok(false),
        },
        Err(_) => Ok(false),
    }
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn set_dlss_debug_overlay(_enabled: bool) -> AppResult<()> {
    Err(AppError::Other("DLSS overlay is Windows-only".into()))
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn get_dlss_debug_overlay() -> AppResult<bool> {
    Ok(false)
}
