use tauri::State;

use crate::state::{AppState, StateSnapshot};

#[tauri::command]
pub fn get_state_snapshot(state: State<AppState>) -> StateSnapshot {
    state.snapshot()
}
