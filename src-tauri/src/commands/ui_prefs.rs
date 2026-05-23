use crate::efficiency;
use crate::tray::TrayPrefs;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn set_close_to_tray(prefs: State<'_, TrayPrefs>, enable: bool) -> Result<(), String> {
    prefs.set_close_to_tray(enable);
    Ok(())
}

#[tauri::command]
pub fn get_close_to_tray(prefs: State<'_, TrayPrefs>) -> bool {
    prefs.close_to_tray()
}

#[tauri::command]
pub fn set_efficiency_mode(enable: bool) -> Result<(), String> {
    efficiency::apply(enable)
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.show().map_err(|e| e.to_string())?;
        w.unminimize().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}
