use tauri::{AppHandle, State};

use crate::api::events::emit_state_snapshot;
use crate::api::types::{AppState, StateSnapshot};
use crate::core::app;

#[tauri::command]
pub fn get_state_snapshot(state: State<AppState>) -> StateSnapshot {
    state.snapshot()
}

#[tauri::command]
pub async fn start_app(handle: AppHandle) -> Result<(), String> {
    app::initialize(&handle)
}

#[tauri::command]
pub fn set_query(query: String, handle: AppHandle, state: State<'_, AppState>) {
    state.set_query(query);
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        if state.filter_items() {
            let _ = emit_state_snapshot(&handle, state.snapshot());
        }
    });
}
