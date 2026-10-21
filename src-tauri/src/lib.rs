pub mod cli;
pub mod commands;
pub mod elevate;
pub mod helper;
pub mod icons;
pub mod model;
pub mod shortcut;
pub mod store;
pub mod task;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 便携优先：配置放在 exe 同目录，程序与数据可以一起复制迁移
            let fallback = app.path().app_data_dir()?;
            let (dir, portable) = store::resolve_dir(fallback.clone());
            std::fs::create_dir_all(&dir)?;

            let file = dir.join("apps.json");
            // 旧版本把配置放在用户数据目录，首次以便携模式启动时迁移过来
            let legacy = fallback.join("apps.json");
            if !file.exists() && legacy != file && legacy.exists() {
                let _ = std::fs::copy(&legacy, &file);
            }

            let state = store::StoreState::load(file, portable);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::is_elevated,
            commands::list_apps,
            commands::add_apps,
            commands::update_app,
            commands::remove_app,
            commands::launch_app,
            commands::ensure_tasks,
            commands::app_icon,
            commands::reveal_in_explorer,
            commands::relaunch_elevated,
            commands::store_info,
            commands::create_desktop_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
