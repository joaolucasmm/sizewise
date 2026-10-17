//! Thin Tauri layer: translates commands and events between the UI and the
//! `scanner` crate. No scanning logic lives here.

mod commands;
mod dto;
mod state;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::system_locale,
            commands::list_drives,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_summary,
            commands::get_children,
            commands::get_top_files,
            commands::get_unreadable,
            commands::reveal_in_explorer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sizewise");
}
