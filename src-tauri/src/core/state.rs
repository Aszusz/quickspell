use std::collections::HashMap;
use std::env;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tauri::{async_runtime, AppHandle};

use crate::api::events;
use crate::api::types::{
    Action, AppInner, AppState, AppStatus, Frame, Item, Spell, StateSnapshot, STARTING_SPELL_ID,
};
use crate::core::template;

pub enum EscapeResult {
    ClearedQuery,
    PoppedFrame,
    Noop,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::RwLock::new(AppInner {
                status: AppStatus::NotStarted,
                spells: HashMap::new(),
                stack: Vec::new(),
                next_frame_id: 0,
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
        inner.stack = vec![new_frame(&mut inner, STARTING_SPELL_ID.to_string())];
        Ok(())
    }

    pub fn finish_loading_with_items(&self, resources_dir: &Path) -> Result<(), String> {
        let Some((items, frame_uid)) = self.load_items_for_current_frame(resources_dir)? else {
            return Ok(());
        };

        if let Ok(mut inner) = self.inner.write() {
            if is_current_frame(&inner, frame_uid) {
                if let Some(frame) = inner.stack.last_mut() {
                    frame.all_items = items.clone();
                    frame.filtered_items = items;
                }
                inner.status = AppStatus::Ready;
            }
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

    pub fn get_current_spell(&self) -> Option<Spell> {
        let inner = self.inner.read().ok()?;
        let frame = inner.stack.last()?;
        inner.spells.get(&frame.spell_id).cloned()
    }

    pub fn set_query(&self, query: String) {
        if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                frame.query = query;
                frame.selected_idx = 0;
                frame.is_filtering = true;
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
        let mut filtered: Vec<Item> = if query.is_empty() {
            all_items
        } else if let Some(cfg) = config {
            crate::core::search::filter_items(&all_items, &query, &cfg)
                .into_iter()
                .cloned()
                .collect()
        } else {
            all_items
        };

        if filtered.len() > 100 {
            filtered.truncate(100);
        }

        let result_count = filtered.len();

        let applied = if let Ok(mut inner) = self.inner.write() {
            match inner.stack.last_mut() {
                Some(frame) if frame.query == query => {
                    frame.filtered_items = filtered;
                    clamp_selection(frame);
                    frame.is_filtering = false;
                    true
                }
                _ => false,
            }
        } else {
            false
        };

        if let Err(err) =
            log_filter_metrics(&query, item_count, result_count, applied, start.elapsed())
        {
            eprintln!("failed to write quickspell log: {err}");
        }

        applied
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (
            status,
            no_of_spells,
            spell_names,
            top_items,
            total_items,
            query,
            is_filtering,
            selected_idx,
            selected_item,
        ) = if let Ok(inner) = self.inner.read() {
            let (top, total, query, is_filtering, selected_idx, selected_item) = inner
                .stack
                .last()
                .map(|f| {
                    let clamped_idx = f.selected_idx.min(f.filtered_items.len().saturating_sub(1));
                    let selected = f.filtered_items.get(clamped_idx).cloned();
                    (
                        f.filtered_items.iter().take(100).cloned().collect(),
                        f.filtered_items.len(),
                        f.query.clone(),
                        f.is_filtering,
                        clamped_idx,
                        selected,
                    )
                })
                .unwrap_or((Vec::new(), 0, String::new(), false, 0, None));

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
                query,
                is_filtering,
                selected_idx,
                selected_item,
            )
        } else {
            (
                AppStatus::Error,
                0,
                Vec::new(),
                Vec::new(),
                0,
                String::new(),
                false,
                0,
                None,
            )
        };

        StateSnapshot {
            status,
            no_of_spells,
            spell_names,
            top_items,
            total_items,
            query,
            is_filtering,
            selected_index: selected_idx,
            selected_item,
        }
    }

    pub fn emit_snapshot(&self, app: &AppHandle) -> Result<(), tauri::Error> {
        events::emit_state_snapshot(app, self.snapshot())
    }

    pub fn set_selection_delta(&self, delta: isize) -> bool {
        if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                let len = frame.filtered_items.len();
                if len == 0 {
                    frame.selected_idx = 0;
                    return true;
                }

                let current = frame.selected_idx.min(len.saturating_sub(1));
                let next = (current as isize + delta).clamp(0, (len.saturating_sub(1)) as isize);
                frame.selected_idx = next as usize;
                return true;
            }
        }
        false
    }

    pub fn handle_escape(&self) -> EscapeResult {
        if let Ok(mut inner) = self.inner.write() {
            if let Some(frame) = inner.stack.last_mut() {
                if !frame.query.is_empty() {
                    frame.query.clear();
                    frame.selected_idx = 0;
                    frame.filtered_items = frame.all_items.clone();
                    frame.is_filtering = false;
                    return EscapeResult::ClearedQuery;
                }
            }

            if inner.stack.len() > 1 {
                inner.stack.pop();
                if let Some(frame) = inner.stack.last_mut() {
                    clamp_selection(frame);
                }
                inner.status = AppStatus::Ready;
                return EscapeResult::PoppedFrame;
            }
        }

        EscapeResult::Noop
    }

    fn load_items_for_current_frame(
        &self,
        resources_dir: &Path,
    ) -> Result<Option<(Vec<Item>, u64)>, String> {
        let (provider_cmd, frame_id, frame_uid) = {
            let inner = self.inner.read().map_err(|_| "state lock poisoned")?;
            let Some(frame) = inner.stack.last() else {
                return Ok(None);
            };
            let spell = inner
                .spells
                .get(&frame.spell_id)
                .ok_or_else(|| format!("spell not found for frame {}", frame.spell_id))?;
            (spell.provider.clone(), frame.spell_id.clone(), frame.id)
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
        Ok(Some((
            stdout
                .lines()
                .filter_map(|line| parse_item_line(line, &frame_id))
                .collect(),
            frame_uid,
        )))
    }

    pub fn stream_items_for_current_frame(
        &self,
        resources_dir: &Path,
        app: &AppHandle,
    ) -> Result<(), String> {
        let (provider_cmd, frame_id, frame_uid) = {
            let inner = self.inner.read().map_err(|_| "state lock poisoned")?;
            let Some(frame) = inner.stack.last() else {
                return Ok(());
            };
            let spell = inner
                .spells
                .get(&frame.spell_id)
                .ok_or_else(|| format!("spell not found for frame {}", frame.spell_id))?;
            (spell.provider.clone(), frame.spell_id.clone(), frame.id)
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

        let mut batch: Vec<Item> = Vec::new();
        let mut last_emit = Instant::now();
        let throttle = Duration::from_millis(500);

        for line in reader.lines().map_while(Result::ok) {
            if let Some(item) = parse_item_line(&line, &frame_id) {
                batch.push(item);
            }
            if last_emit.elapsed() >= throttle {
                if self.is_current_frame(frame_uid) {
                    self.append_items_for_frame(frame_uid, std::mem::take(&mut batch));
                    let _ = self.emit_snapshot(app);
                } else {
                    batch.clear();
                }
                last_emit = Instant::now();
            }
        }

        if !batch.is_empty() && self.is_current_frame(frame_uid) {
            self.append_items_for_frame(frame_uid, batch);
        }

        if self.is_current_frame(frame_uid) {
            self.set_ready();
            let _ = self.emit_snapshot(app);
        }
        let _ = child.wait();
        Ok(())
    }

    pub fn invoke_action(
        &self,
        label: &str,
        resources_dir: &Path,
        app: &AppHandle,
    ) -> Result<(), String> {
        let (frames, actions) = {
            let inner = self.inner.read().map_err(|_| "state lock poisoned")?;
            let frames = inner.stack.clone();
            let spell = inner
                .stack
                .last()
                .and_then(|frame| inner.spells.get(&frame.spell_id))
                .ok_or_else(|| "no active spell".to_string())?;
            (frames, spell.actions.clone())
        };

        for action in actions {
            let action_label = action_name(&action).unwrap_or("MAIN");
            if action_label != label {
                continue;
            }

            if !condition_passes(action_condition(&action), &frames)? {
                continue;
            }

            match action {
                Action::Spell { spell, .. } => {
                    let rendered_spell =
                        template::resolve_template(&spell, &frames).map_err(|e| match e {
                            template::TemplateError::Render(err) => err,
                        })?;

                    let target_spell_id = rendered_spell.trim();
                    if target_spell_id.is_empty() {
                        return Err("resolved spell id is empty".to_string());
                    }

                    {
                        let mut inner = self.inner.write().map_err(|_| "state lock poisoned")?;
                        if !inner.spells.contains_key(target_spell_id) {
                            return Err(format!("spell {target_spell_id} not found"));
                        }
                        let frame = new_frame(&mut inner, target_spell_id.to_string());
                        inner.stack.push(frame);
                        inner.status = AppStatus::Loading;
                    }

                    let _ = self.emit_snapshot(app);

                    let state = self.clone();
                    let resources_dir = resources_dir.to_path_buf();
                    let app_handle = app.clone();
                    async_runtime::spawn_blocking(move || {
                        let is_streaming = state
                            .get_current_spell()
                            .and_then(|s| s.is_streaming)
                            .unwrap_or(false);

                        let result = if is_streaming {
                            state.stream_items_for_current_frame(&resources_dir, &app_handle)
                        } else {
                            state.finish_loading_with_items(&resources_dir)
                        };

                        match result {
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

                    return Ok(());
                }
                Action::Cmd { cmd, .. } => {
                    let rendered_cmd =
                        template::resolve_template(&cmd, &frames).map_err(|e| match e {
                            template::TemplateError::Render(err) => err,
                        })?;

                    if rendered_cmd.trim().is_empty() {
                        return Err("resolved command is empty".to_string());
                    }

                    let argv = shell_words::split(&rendered_cmd)
                        .map_err(|err| format!("failed to parse action command: {err}"))?;

                    let (program, args) = argv
                        .split_first()
                        .ok_or_else(|| "resolved command is empty".to_string())?;

                    let status = std::process::Command::new(program)
                        .args(args)
                        .current_dir(resources_dir)
                        .status()
                        .map_err(|err| format!("failed to run action command: {err}"))?;

                    if status.success() {
                        return Ok(());
                    } else {
                        return Err(format!("action command exited with status {status}"));
                    }
                }
            }
        }

        Err(format!("no matching action for label {label}"))
    }

    fn is_current_frame(&self, frame_uid: u64) -> bool {
        if let Ok(inner) = self.inner.read() {
            is_current_frame(&inner, frame_uid)
        } else {
            false
        }
    }

    fn append_items_for_frame(&self, frame_uid: u64, new_items: Vec<Item>) {
        if let Ok(mut inner) = self.inner.write() {
            append_items_for_frame(&mut inner, frame_uid, new_items);
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

fn is_current_frame(inner: &AppInner, frame_uid: u64) -> bool {
    inner
        .stack
        .last()
        .map(|frame| frame.id == frame_uid)
        .unwrap_or(false)
}

fn append_items_for_frame(inner: &mut AppInner, frame_uid: u64, new_items: Vec<Item>) {
    if let Some(frame) = inner.stack.last_mut() {
        if frame.id == frame_uid {
            frame.all_items.extend(new_items.clone());
            frame.filtered_items.extend(new_items);
        }
    }
}

fn new_frame(inner: &mut AppInner, spell_id: String) -> Frame {
    let id = inner.next_frame_id;
    inner.next_frame_id = inner.next_frame_id.wrapping_add(1);
    Frame {
        id,
        spell_id,
        query: String::new(),
        all_items: Vec::new(),
        filtered_items: Vec::new(),
        is_filtering: false,
        selected_idx: 0,
    }
}

fn parse_item_line(line: &str, frame_id: &str) -> Option<Item> {
    if line.trim().is_empty() {
        return None;
    }

    match Item::from_line(line) {
        Some(item) => Some(item),
        None => {
            eprintln!("skipping malformed item for frame {frame_id}: {line}");
            None
        }
    }
}

fn log_filter_metrics(
    query: &str,
    items: usize,
    results: usize,
    applied: bool,
    elapsed: Duration,
) -> std::io::Result<()> {
    let log_path = resolve_log_path()?;
    if let Some(parent) = log_path.parent() {
        create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    writeln!(
        file,
        "[filter] query={query:?} items={items} results={results} applied={applied} time={elapsed:?}"
    )
}

fn resolve_log_path() -> std::io::Result<std::path::PathBuf> {
    let base = if cfg!(target_os = "macos") {
        env::var_os("HOME").map(std::path::PathBuf::from).map(|p| {
            p.join("Library")
                .join("Application Support")
                .join("QuickSpell")
        })
    } else if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .map(|p| p.join("QuickSpell"))
    } else {
        env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                env::var_os("HOME").map(|p| std::path::PathBuf::from(p).join(".local/share"))
            })
            .map(|p| p.join("quickspell"))
    };

    base.map(|p| p.join("quickspell.log"))
        .ok_or_else(|| std::io::Error::other("could not resolve log directory for quickspell"))
}

fn clamp_selection(frame: &mut Frame) {
    if frame.filtered_items.is_empty() {
        frame.selected_idx = 0;
    } else {
        frame.selected_idx = frame
            .selected_idx
            .min(frame.filtered_items.len().saturating_sub(1));
    }
}

fn action_name(action: &Action) -> Option<&str> {
    match action {
        Action::Cmd { name, .. } | Action::Spell { name, .. } => name.as_deref(),
    }
}

fn action_condition(action: &Action) -> Option<&str> {
    match action {
        Action::Cmd { condition, .. } | Action::Spell { condition, .. } => condition.as_deref(),
    }
}

fn condition_passes(condition: Option<&str>, frames: &[Frame]) -> Result<bool, String> {
    let Some(raw) = condition else {
        return Ok(true);
    };

    let rendered = template::resolve_template(raw, frames).map_err(|e| match e {
        template::TemplateError::Render(err) => err,
    })?;

    let text = rendered.trim();

    if text.is_empty() {
        return Ok(true);
    }

    if let Some((lhs, rhs)) = text.split_once("==") {
        return Ok(normalize_condition_value(lhs) == normalize_condition_value(rhs));
    }

    if let Some((lhs, rhs)) = text.split_once("!=") {
        return Ok(normalize_condition_value(lhs) != normalize_condition_value(rhs));
    }

    match text.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => Ok(!text.is_empty()),
    }
}

fn normalize_condition_value(value: &str) -> String {
    strip_matching_quotes(value.trim()).to_string()
}

fn strip_matching_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}
