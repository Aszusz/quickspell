use tauri::{AppHandle, State};

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
