use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
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
    pub top_items: Vec<String>,
}

#[derive(Debug)]
struct AppInner {
    status: AppStatus,
    spells: HashMap<String, Spell>,
    stack: Vec<Frame>,
    all_items: Vec<String>,
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
                all_items: Vec::new(),
            }),
        }
    }

    pub fn set_status(&self, status: AppStatus) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = status;
        }
    }

    pub fn set_ready_with_spells(
        &self,
        spells: HashMap<String, Spell>,
        resources_dir: &Path,
    ) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().map_err(|_| "state lock poisoned")?;
            inner.status = AppStatus::Loading;
            inner.spells = spells;
            inner.stack = vec![Frame {
                spell_id: "quickspell".to_string(),
                query: String::new(),
            }];
            inner.all_items.clear();
        }

        let items = self.load_items_for_current_frame(resources_dir)?;

        if let Ok(mut inner) = self.inner.lock() {
            inner.all_items = items;
            inner.status = AppStatus::Ready;
        }

        Ok(())
    }

    pub fn set_error(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = AppStatus::Error;
            inner.spells.clear();
            inner.stack.clear();
            inner.all_items.clear();
        }
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (status, no_of_spells, spell_names, top_items) = if let Ok(inner) = self.inner.lock() {
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
                inner.all_items.iter().take(20).cloned().collect(),
            )
        } else {
            (AppStatus::Error, 0, Vec::new(), Vec::new())
        };

        StateSnapshot {
            status,
            no_of_spells,
            spell_names,
            top_items,
        }
    }

    pub fn emit_snapshot(&self, app: &AppHandle) -> Result<(), tauri::Error> {
        events::emit_state_snapshot(app, self.snapshot())
    }

    fn load_items_for_current_frame(&self, resources_dir: &Path) -> Result<Vec<String>, String> {
        let (provider_cmd, frame_id) = {
            let inner = self.inner.lock().map_err(|_| "state lock poisoned")?;
            let frame = inner
                .stack
                .last()
                .ok_or_else(|| "no active frame on stack".to_string())?;
            let spell = inner
                .spells
                .get(&frame.spell_id)
                .ok_or_else(|| format!("spell not found for frame {}", frame.spell_id))?;
            (spell.provider.clone(), frame.spell_id.clone())
        };

        let output = Command::new("sh")
            .arg("-c")
            .arg(provider_cmd)
            .current_dir(resources_dir)
            .output()
            .map_err(|err| format!("failed to launch provider for {frame_id}: {err}"))?;

        if !output.status.success() {
            return Err(format!(
                "provider for {frame_id} exited with status {}",
                output.status
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|line| line.to_string()).collect())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
