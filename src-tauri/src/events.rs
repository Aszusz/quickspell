use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
pub struct CounterChanged {
    pub count: i32,
}

impl CounterChanged {
    const NAME: &str = "counter-changed";

    pub fn emit(self, app: &AppHandle) {
        let _ = app.emit(Self::NAME, self);
    }
}
