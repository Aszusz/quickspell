use std::path::PathBuf;

use tauri::{path::BaseDirectory, Manager};

use crate::state::{AppState, AppStatus};

pub fn initialize(app: &tauri::App) {
    let state: tauri::State<AppState> = app.state();
    state.set_status(AppStatus::Loading);

    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/spells");

    let resource_dir = ["resources/spells", "spells"].iter().find_map(|relative| {
        app.path()
            .resolve(relative, BaseDirectory::Resource)
            .ok()
            .filter(|p| p.exists())
    });

    // Prefer the local resources directory during dev if it exists; otherwise use the bundled path, falling back to dev path if neither exists.
    let spells_dir = dev_dir
        .exists()
        .then_some(dev_dir.clone())
        .or(resource_dir)
        .unwrap_or(dev_dir);
    let resources_dir = spells_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| spells_dir.clone());

    let result = crate::spells::load_spells_from_dir(&spells_dir)
        .map_err(|err| format!("failed to load spells: {err}"));

    match result {
        Ok(spells) => {
            if let Err(err) = state.set_ready_with_spells(spells, &resources_dir) {
                eprintln!("{err}");
                state.set_error();
            }
        }
        Err(err) => {
            eprintln!("{err}");
            state.set_error();
        }
    }

    if let Err(err) = state.emit_snapshot(app.handle()) {
        eprintln!("failed to emit state snapshot event: {err}");
    }
}
