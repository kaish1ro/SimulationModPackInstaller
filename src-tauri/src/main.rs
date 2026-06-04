// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod installer;
mod java;
mod manifest;
mod profile;
mod icon_b64;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            installer::check_java,
            installer::pick_directory,
            installer::get_default_install_dir,
            installer::check_install_exists,
            installer::get_profiles_path,
            installer::save_settings,
            installer::launch_game,
            installer::install,
            installer::check_updates,
            installer::apply_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
