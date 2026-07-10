use crate::error::{AppError, AppResult};
use crate::state::AppState;
use dlssync_contracts::{JournalFilter, OperationRecord};
use tauri::State;

#[tauri::command]
pub async fn journal_list(
    state: State<'_, AppState>,
    filter: Option<JournalFilter>,
) -> AppResult<Vec<OperationRecord>> {
    let guard = state.journal.read();
    let journal = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("operation journal is unavailable".into()))?;
    Ok(journal.list(&filter.unwrap_or_default())?)
}

#[tauri::command]
pub async fn journal_export(
    state: State<'_, AppState>,
    filter: Option<JournalFilter>,
) -> AppResult<String> {
    let guard = state.journal.read();
    let journal = guard
        .as_ref()
        .ok_or_else(|| AppError::Other("operation journal is unavailable".into()))?;
    Ok(journal.export_redacted_json(&filter.unwrap_or_default())?)
}
