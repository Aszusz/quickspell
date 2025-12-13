use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tauri::AppHandle;

use crate::api::events;
use crate::api::types::{
    AppInner, AppState, AppStatus, Frame, Spell, StateSnapshot, STARTING_SPELL_ID,
};

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::RwLock::new(AppInner {
                status: AppStatus::NotStarted,
                spells: HashMap::new(),
                stack: Vec::new(),
            })),
        }
    }

    pub fn begin_loading_with_spells(&self, spells: HashMap<String, Spell>) -> Result<(), String> {
        let mut inner = self.inner.write().map_err(|_| "state lock poisoned")?;

        if inner.status != AppStatus::NotStarted {
            return Err("already started".to_string());
        }

        inner.status = AppStatus::Booting;
        inner.spells = spells;
        inner.status = AppStatus::Loading;
        inner.stack = vec![Frame {
            spell_id: STARTING_SPELL_ID.to_string(),
            query: String::new(),
            all_items: Vec::new(),
            filtered_items: Vec::new(),
        }];
        Ok(())
    }

    pub fn finish_loading_with_items(&self, resources_dir: &Path) -> Result<(), String> {
        let items = self.load_items_for_current_frame(resources_dir)?;

        if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                frame.all_items = items.clone();
                frame.filtered_items = items;
            }
            inner.status = AppStatus::Ready;
            Ok(())
        } else {
            Err("state lock poisoned".to_string())
        }
    }

    pub fn set_error(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.status = AppStatus::Error;
            inner.spells.clear();
            inner.stack.clear();
        }
    }

    pub fn set_ready(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.status = AppStatus::Ready;
        }
    }

    pub fn append_items(&self, new_items: Vec<String>) {
        if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                frame.all_items.extend(new_items.clone());
                frame.filtered_items.extend(new_items);
            }
        }
    }

    pub fn get_current_spell(&self) -> Option<Spell> {
        let inner = self.inner.read().ok()?;
        let frame = inner.stack.last()?;
        inner.spells.get(&frame.spell_id).cloned()
    }

    pub fn set_query(&self, query: String) {
        if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                frame.query = query;
            }
        }
    }

    pub fn filter_items(&self) -> bool {
        let start = Instant::now();

        let (all_items, query, config) = {
            let inner = match self.inner.read() {
                Ok(i) => i,
                Err(_) => return false,
            };
            let frame = match inner.stack.last() {
                Some(f) => f,
                None => return false,
            };
            let cfg = inner
                .spells
                .get(&frame.spell_id)
                .and_then(|s| s.search.clone());
            (frame.all_items.clone(), frame.query.clone(), cfg)
        };

        let item_count = all_items.len();
        let filtered: Vec<String> = if query.is_empty() {
            all_items
        } else if let Some(cfg) = config {
            crate::core::search::filter_items(&all_items, &query, &cfg)
                .into_iter()
                .cloned()
                .collect()
        } else {
            all_items
        };
        let result_count = filtered.len();

        let applied = if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                if frame.query == query {
                    frame.filtered_items = filtered;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        println!(
            "[filter] query={:?} items={} results={} applied={} time={:?}",
            query,
            item_count,
            result_count,
            applied,
            start.elapsed()
        );

        applied
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (status, no_of_spells, spell_names, top_items, total_items) =
            if let Ok(inner) = self.inner.read() {
                let (top, total) = inner
                    .stack
                    .last()
                    .map(|f| {
                        (
                            f.filtered_items.iter().take(20).cloned().collect(),
                            f.filtered_items.len(),
                        )
                    })
                    .unwrap_or((Vec::new(), 0));

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
                    top,
                    total,
                )
            } else {
                (AppStatus::Error, 0, Vec::new(), Vec::new(), 0)
            };

        StateSnapshot {
            status,
            no_of_spells,
            spell_names,
            top_items,
            total_items,
        }
    }

    pub fn emit_snapshot(&self, app: &AppHandle) -> Result<(), tauri::Error> {
        events::emit_state_snapshot(app, self.snapshot())
    }

    fn load_items_for_current_frame(&self, resources_dir: &Path) -> Result<Vec<String>, String> {
        let (provider_cmd, frame_id) = {
            let inner = self.inner.read().map_err(|_| "state lock poisoned")?;
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

    pub fn stream_items_for_current_frame(
        &self,
        resources_dir: &Path,
        app: &AppHandle,
    ) -> Result<(), String> {
        let (provider_cmd, frame_id) = {
            let inner = self.inner.read().map_err(|_| "state lock poisoned")?;
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

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&provider_cmd)
            .current_dir(resources_dir)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn provider for {frame_id}: {e}"))?;

        let stdout = child.stdout.take().ok_or("no stdout handle")?;
        let reader = BufReader::new(stdout);

        let mut batch = Vec::new();
        let mut last_emit = Instant::now();
        let throttle = Duration::from_millis(500);

        for line in reader.lines().map_while(Result::ok) {
            batch.push(line);
            if last_emit.elapsed() >= throttle {
                self.append_items(std::mem::take(&mut batch));
                let _ = self.emit_snapshot(app);
                last_emit = Instant::now();
            }
        }

        if !batch.is_empty() {
            self.append_items(batch);
        }

        self.set_ready();
        let _ = self.emit_snapshot(app);
        let _ = child.wait();
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
