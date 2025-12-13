use tauri::{async_runtime, AppHandle, Manager, State};

use crate::bootstrap;
use crate::state::{AppState, StateSnapshot};

#[tauri::command]
pub fn get_state_snapshot(state: State<AppState>) -> StateSnapshot {
    state.snapshot()
}

#[tauri::command]
pub async fn start_app(app: AppHandle) -> Result<(), String> {
    let (spells_dir, resources_dir) = bootstrap::resolve_resource_dirs(&app);

    let spells = crate::spells::load_spells_from_dir(&spells_dir)
        .map_err(|err| format!("failed to load spells: {err}"))?;

    let state: State<AppState> = app.state();
    state.begin_loading_with_spells(spells)?;
    state
        .emit_snapshot(&app)
        .map_err(|err| format!("failed to emit loading snapshot: {err}"))?;

    let app_handle = app.clone();
    async_runtime::spawn_blocking(move || {
        let state: State<AppState> = app_handle.state();
        match state.finish_loading_with_items(&resources_dir) {
            Ok(()) => {
                let _ = state.emit_snapshot(&app_handle);
            }
            Err(err) => {
                state.set_error();
                let _ = state.emit_snapshot(&app_handle);
                eprintln!("failed to load items: {err}");
            }
        }
    });

    Ok(())
}
