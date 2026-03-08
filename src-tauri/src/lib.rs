mod commands;
mod fs_ops;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_directory,
            commands::get_home_dir,
            commands::create_directory,
            commands::create_file,
            commands::rename_entry,
            commands::delete_entry,
            commands::copy_entry,
            commands::move_entry,
            commands::list_drives,
            commands::path_exists,
            commands::list_trash,
            commands::restore_trash,
            commands::empty_trash,
            commands::open_default,
            commands::list_apps_for_mime,
            commands::open_with_app,
            commands::compress,
            commands::cancel_operation,
            commands::extract,
            commands::get_properties,
            commands::get_dir_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
