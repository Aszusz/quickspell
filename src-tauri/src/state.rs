use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::events;
use crate::spells::Spell;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub spell_id: String,
    pub query: String,
}

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
    pub spell_names: Vec<String>,
}

#[derive(Debug)]
struct AppInner {
    status: AppStatus,
    spells: HashMap<String, Spell>,
    stack: Vec<Frame>,
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
                stack: Vec::new(),
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
            inner.stack = vec![Frame {
                spell_id: "quickspell".to_string(),
                query: String::new(),
            }];
        }
    }

    pub fn set_error(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = AppStatus::Error;
            inner.spells.clear();
            inner.stack.clear();
        }
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (status, no_of_spells, spell_names) = if let Ok(inner) = self.inner.lock() {
            (
                inner.status,
                inner.spells.len(),
                inner
                    .stack
                    .iter()
                    .map(|frame| {
                        inner
                            .spells
                            .get(&frame.spell_id)
                            .map(|spell| spell.name.clone())
                            .unwrap_or_else(|| frame.spell_id.clone())
                    })
                    .collect(),
            )
        } else {
            (AppStatus::Error, 0, Vec::new())
        };

        StateSnapshot {
            status,
            no_of_spells,
            spell_names,
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
