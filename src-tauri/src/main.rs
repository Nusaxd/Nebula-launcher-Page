// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod download;
mod error;
mod install;
mod java;
mod launch;
mod models;
mod state;
mod versions;

use std::sync::Arc;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let state = state::AppState::new(root).map_err(|e| e.to_string())?;
            app.manage(Arc::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::update_settings,
            commands::test_java,
            commands::running_instances,
            commands::list_versions,
            commands::install_version,
            commands::uninstall_version,
            commands::save_instance,
            commands::duplicate_instance,
            commands::delete_instance,
            commands::select_instance,
            commands::open_instance_folder,
            commands::open_data_folder,
            commands::open_url,
            commands::add_offline_account,
            commands::login_elyby,
            commands::remove_account,
            commands::select_account,
            commands::launch_instance,
            commands::kill_instance,
            commands::get_logs,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nebula Launcher");
}
