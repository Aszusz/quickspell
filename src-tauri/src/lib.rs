mod bootstrap;
mod commands;
mod events;
mod spells;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            bootstrap::initialize(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_state_snapshot,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
