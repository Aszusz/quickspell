use tauri::{AppHandle, State};

use crate::events::CounterChanged;
use crate::state::AppState;

#[tauri::command]
pub fn get_count(state: State<AppState>) -> i32 {
    state.counter.get()
}

#[tauri::command]
pub fn increment(state: State<AppState>, app: AppHandle) {
    CounterChanged {
        count: state.counter.increment(),
    }
    .emit(&app);
}

#[tauri::command]
pub fn decrement(state: State<AppState>, app: AppHandle) {
    CounterChanged {
        count: state.counter.decrement(),
    }
    .emit(&app);
}

#[tauri::command]
pub fn reset(state: State<AppState>, app: AppHandle) {
    CounterChanged {
        count: state.counter.reset(),
    }
    .emit(&app);
}
