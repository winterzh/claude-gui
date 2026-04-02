mod config;
mod launcher;
mod pty;

use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Mutex::new(pty::PtyState::new())) as pty::SharedPtyState)
        .invoke_handler(tauri::generate_handler![
            config::save_config,
            config::load_config,
            config::save_working_dir,
            config::save_model_pref,
            config::save_profiles,
            config::test_connection,
            launcher::launch_claude_code,
            pty::spawn_claude,
            pty::pty_write,
            pty::pty_resize,
            pty::kill_claude,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
