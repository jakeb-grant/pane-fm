mod commands;
mod config;
mod error;
mod fs_ops;
pub mod progress;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .manage(commands::theme::ThemeWatcher(std::sync::Mutex::new(None)))
        .manage(commands::watcher::DirWatcher(std::sync::Mutex::new(None)))
        .manage(commands::config::ConfigWatcher(std::sync::Mutex::new(None)))
        .setup(|_app| {
            config::install_default_config();
            commands::theme::install_default_themes();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::file_ops::list_directory,
            commands::file_ops::get_drag_icon,
            commands::file_ops::get_home_dir,
            commands::file_ops::create_directory,
            commands::file_ops::create_file,
            commands::file_ops::rename_entry,
            commands::file_ops::delete_entry,
            commands::file_ops::permanent_delete,
            commands::file_ops::copy_entry,
            commands::file_ops::move_entry,
            commands::file_ops::create_symlink,
            commands::file_ops::chmod_entry,
            commands::file_ops::read_file_preview,
            commands::file_ops::read_pdf_preview,
            commands::file_ops::paste_entries,
            commands::file_ops::delete_entries_permanently,
            commands::file_ops::path_exists,
            commands::file_ops::get_children_counts,
            commands::file_ops::get_properties,
            commands::file_ops::get_dir_stats,
            commands::drives::list_drives,
            commands::trash::list_trash,
            commands::trash::restore_trash,
            commands::trash::empty_trash,
            commands::apps::open_default,
            commands::apps::list_apps_for_mime,
            commands::apps::open_with_app,
            commands::apps::open_terminal,
            commands::apps::run_custom_action,
            commands::archive::compress,
            commands::archive::cancel_operation,
            commands::archive::extract,
            commands::config::get_config,
            commands::config::watch_config,
            commands::theme::load_theme_css,
            commands::theme::watch_theme,
            commands::search::search_files,
            commands::search::cancel_search,
            commands::watcher::watch_directory,
            commands::watcher::unwatch_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
