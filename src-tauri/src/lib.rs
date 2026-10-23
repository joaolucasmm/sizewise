//! Thin Tauri layer: translates commands and events between the UI and the
//! `scanner` and `diagnostics` crates. No scanning or diagnosis logic lives here.

mod commands;
mod diagnosis_commands;
mod diagnosis_dto;
mod dto;
mod state;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .manage(state::SamplerState::default())
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
            diagnosis_commands::get_overview,
            diagnosis_commands::list_startup_items,
            diagnosis_commands::get_startup_durations,
            diagnosis_commands::start_process_sampling,
            diagnosis_commands::stop_process_sampling,
            diagnosis_commands::end_process,
            diagnosis_commands::list_installed_programs,
            diagnosis_commands::measure_program,
            diagnosis_commands::start_uninstall,
            diagnosis_commands::list_physical_disks,
            diagnosis_commands::list_volumes,
            diagnosis_commands::reveal_path,
            diagnosis_commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sizewise");
}
