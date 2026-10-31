#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod application;
mod commands;
mod desktop;
mod domain;
mod infrastructure;
mod settings;
use tauri::Manager;
fn main() {
    let mut context = tauri::generate_context!();
    if cfg!(debug_assertions) {
        context.config_mut().identifier.push_str(".dev");
        for window in &mut context.config_mut().app.windows {
            window.title.push_str("（开发）");
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            let journal =
                infrastructure::storage::FileJournal::new().map_err(std::io::Error::other)?;
            app.manage(commands::AppState(std::sync::Mutex::new(
                application::Service {
                    network: infrastructure::network::WindowsNetwork,
                    journal,
                },
            )));
            let preferences = settings::Preferences::load().map_err(std::io::Error::other)?;
            app.manage(settings::PreferencesState(std::sync::Mutex::new(
                preferences,
            )));
            desktop::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::enable,
            commands::restore,
            commands::probe_connectivity,
            settings::get_preferences,
            settings::save_preferences,
            settings::open_project_link,
            settings::check_update
        ])
        .run(context)
        .expect("无法启动 SplitRoute");
}
