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
use crate::core::search;

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::RwLock::new(AppInner {
                status: AppStatus::NotStarted,
                spells: HashMap::new(),
                stack: Vec::new(),
                all_items: Vec::new(),
                filtered_items: Vec::new(),
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
        }];
        inner.all_items.clear();
        inner.filtered_items.clear();
        Ok(())
    }

    pub fn finish_loading_with_items(&self, resources_dir: &Path) -> Result<(), String> {
        let items = self.load_items_for_current_frame(resources_dir)?;

        if let Ok(mut inner) = self.inner.write() {
            inner.filtered_items = items.clone();
            inner.all_items = items;
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
            inner.all_items.clear();
            inner.filtered_items.clear();
        }
    }

    pub fn set_ready(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.status = AppStatus::Ready;
        }
    }

    pub fn append_items(&self, new_items: Vec<String>) {
        if let Ok(mut inner) = self.inner.write() {
            inner.filtered_items.extend(new_items.clone());
            inner.all_items.extend(new_items);
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

    pub fn filter_items(&self) {
        let start = Instant::now();

        // Read lock: copy data needed for filtering
        let (all_items, query, search_config) = {
            let inner = match self.inner.read() {
                Ok(inner) => inner,
                Err(_) => return,
            };
            let frame = match inner.stack.last() {
                Some(f) => f,
                None => return,
            };
            let config = inner
                .spells
                .get(&frame.spell_id)
                .and_then(|s| s.search.clone());
            (inner.all_items.clone(), frame.query.clone(), config)
        }; // read lock released
        let copy_time = start.elapsed();
        let item_count = all_items.len();

        // Filter without holding any lock
        let filter_start = Instant::now();
        let filtered = if query.is_empty() {
            all_items
        } else if let Some(config) = search_config {
            search::filter_items(&all_items, &query, &config)
                .into_iter()
                .cloned()
                .collect()
        } else {
            all_items
        };
        let filter_time = filter_start.elapsed();
        let result_count = filtered.len();

        // Write lock: store filtered results (brief)
        let write_start = Instant::now();
        if let Ok(mut inner) = self.inner.write() {
            inner.filtered_items = filtered;
        }
        let write_time = write_start.elapsed();

        println!(
            "[filter] query={:?} items={} results={} | copy={:?} filter={:?} write={:?} total={:?}",
            query,
            item_count,
            result_count,
            copy_time,
            filter_time,
            write_time,
            start.elapsed()
        );
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (status, no_of_spells, spell_names, top_items, total_items) =
            if let Ok(inner) = self.inner.read() {
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
                    inner.filtered_items.iter().take(20).cloned().collect(),
                    inner.filtered_items.len(),
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
