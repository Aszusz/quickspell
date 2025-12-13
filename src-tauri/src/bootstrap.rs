use std::path::PathBuf;

use tauri::{path::BaseDirectory, AppHandle, Manager};

/// Resolve the spells directory (preferring the local dev path) and its parent resources directory.
pub fn resolve_resource_dirs(app: &AppHandle) -> (PathBuf, PathBuf) {
    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/spells");

    let resource_dir = ["resources/spells", "spells"].iter().find_map(|relative| {
        app.path()
            .resolve(relative, BaseDirectory::Resource)
            .ok()
            .filter(|p| p.exists())
    });

    let spells_dir = dev_dir
        .exists()
        .then_some(dev_dir.clone())
        .or(resource_dir)
        .unwrap_or(dev_dir);
    let resources_dir = spells_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| spells_dir.clone());

    (spells_dir, resources_dir)
}
