use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::events;
use crate::spells::Spell;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Booting,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    pub status: AppStatus,
    #[serde(rename = "noOfSpells")]
    pub no_of_spells: usize,
}

#[derive(Debug)]
struct AppInner {
    status: AppStatus,
    spells: HashMap<String, Spell>,
}

pub struct AppState {
    inner: Mutex<AppInner>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AppInner {
                status: AppStatus::Booting,
                spells: HashMap::new(),
            }),
        }
    }

    pub fn set_status(&self, status: AppStatus) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = status;
        }
    }

    pub fn set_ready_with_spells(&self, spells: HashMap<String, Spell>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = AppStatus::Ready;
            inner.spells = spells;
        }
    }

    pub fn set_error(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = AppStatus::Error;
            inner.spells.clear();
        }
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (status, no_of_spells) = if let Ok(inner) = self.inner.lock() {
            (inner.status, inner.spells.len())
        } else {
            (AppStatus::Error, 0)
        };

        StateSnapshot {
            status,
            no_of_spells,
        }
    }

    pub fn emit_snapshot(&self, app: &AppHandle) -> Result<(), tauri::Error> {
        events::emit_state_snapshot(app, self.snapshot())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
