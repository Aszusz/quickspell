use tauri::{AppHandle, Emitter};

use crate::api::types::StateSnapshot;

pub const STATE_SNAPSHOT_EVENT: &str = "state-snapshot";

pub fn emit_state_snapshot(app: &AppHandle, snapshot: StateSnapshot) -> Result<(), tauri::Error> {
    app.emit(STATE_SNAPSHOT_EVENT, snapshot)
}
