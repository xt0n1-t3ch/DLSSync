use crate::error::AppResult;
use crate::state::AppState;
use crate::system_info::{self, SystemInfo};
use tauri::State;

#[tauri::command]
pub async fn get_system_info(state: State<'_, AppState>) -> AppResult<SystemInfo> {
    {
        let guard = state.system_info.read();
        if let Some(info) = guard.as_ref() {
            return Ok(info.clone());
        }
    }
    let collected = tokio::task::spawn_blocking(system_info::collect)
        .await
        .map_err(|e| crate::error::AppError::Other(format!("system_info collect: {e}")))?;
    {
        let mut guard = state.system_info.write();
        *guard = Some(collected.clone());
    }
    Ok(collected)
}
