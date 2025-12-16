use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use tauri::{async_runtime, path::BaseDirectory, AppHandle, Manager, State};

use crate::api::types::{AppState, Spell, SpellLoadError};

pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let (spells_dir, resources_dir) = resolve_resource_dirs(app);

    let spells =
        load_spells_from_dir(&spells_dir).map_err(|err| format!("failed to load spells: {err}"))?;

    let state: State<AppState> = app.state();
    if state.begin_loading_with_spells(spells).is_err() {
        return Ok(()); // already started
    }
    state
        .emit_snapshot(app)
        .map_err(|err| format!("failed to emit loading snapshot: {err}"))?;

    let app_handle = app.clone();
    async_runtime::spawn_blocking(move || {
        let state: State<AppState> = app_handle.state();

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

    Ok(())
}

pub fn resolve_resource_dirs(app: &AppHandle) -> (PathBuf, PathBuf) {
    let factory_resources_dir = resolve_factory_resources_dir(app);
    let user_resources_dir = match resolve_user_resources_dir(app) {
        Ok(dir) => {
            if let Err(err) = sync_default_resources(&factory_resources_dir, &dir) {
                eprintln!("failed to sync default resources: {err}");
            }
            dir
        }
        Err(err) => {
            eprintln!("failed to resolve user resources dir, falling back to factory resources: {err}");
            factory_resources_dir.clone()
        }
    };

    let spells_dir = user_resources_dir.join("spells");
    (spells_dir, user_resources_dir)
}

pub fn resolve_resources_dir(app: &AppHandle) -> PathBuf {
    let (_, resources_dir) = resolve_resource_dirs(app);
    resources_dir
}

fn resolve_factory_resources_dir(app: &AppHandle) -> PathBuf {
    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    if dev_dir.exists() {
        return dev_dir;
    }

    let resource_dir = ["resources/spells", "spells", "resources"]
        .iter()
        .find_map(|relative| {
            app.path()
                .resolve(relative, BaseDirectory::Resource)
                .ok()
                .filter(|p| p.exists())
        });

    resource_dir
        .map(|path| {
            if path.file_name().map(|name| name == "spells").unwrap_or(false) {
                path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
            } else {
                path
            }
        })
        .unwrap_or(dev_dir)
}

fn resolve_user_resources_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("failed to resolve app config dir: {err}"))?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("failed to create user resources dir {}: {err}", dir.display()))?;
    Ok(dir)
}

fn sync_default_resources(factory_dir: &Path, user_dir: &Path) -> std::io::Result<()> {
    for subdir in ["spells", "providers"] {
        let src = factory_dir.join(subdir);
        if !src.exists() {
            continue;
        }

        let dst = user_dir.join(subdir);
        fs::create_dir_all(&dst)?;

        for entry in fs::read_dir(&src)? {
            let entry = entry?;
            let src_path = entry.path();
            if !src_path.is_file() {
                continue;
            }

            let dst_path = dst.join(entry.file_name());
            if !dst_path.exists() {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    }

    Ok(())
}

fn load_spells_from_dir(dir: &Path) -> Result<HashMap<String, Spell>, SpellLoadError> {
    if !dir.exists() {
        return Err(SpellLoadError::ResourceNotFound(dir.to_path_buf()));
    }

    let mut spells = HashMap::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("yml") | Some("yaml") => {
                let content = fs::read_to_string(&path)?;
                let spell: Spell =
                    serde_yaml::from_str(&content).map_err(|error| SpellLoadError::Parse {
                        path: path.clone(),
                        error,
                    })?;
                spells.insert(spell.id.clone(), spell);
            }
            _ => continue,
        }
    }

    Ok(spells)
}

impl std::fmt::Display for SpellLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpellLoadError::ResourceNotFound(path) => {
                write!(f, "spells directory not found at {}", path.display())
            }
            SpellLoadError::Io(err) => write!(f, "io error while loading spells: {err}"),
            SpellLoadError::Parse { path, error } => {
                write!(f, "failed to parse {}: {error}", path.display())
            }
        }
    }
}

impl From<std::io::Error> for SpellLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_dev_spells() {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("resources/spells");
        let spells = load_spells_from_dir(&dir).expect("failed to load spells from dev resources");
        assert!(!spells.is_empty(), "expected at least one spell");
    }
}
