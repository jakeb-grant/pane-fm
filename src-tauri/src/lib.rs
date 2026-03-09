mod commands;
mod error;
mod fs_ops;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::file_ops::list_directory,
            commands::file_ops::get_home_dir,
            commands::file_ops::create_directory,
            commands::file_ops::create_file,
            commands::file_ops::rename_entry,
            commands::file_ops::delete_entry,
            commands::file_ops::permanent_delete,
            commands::file_ops::copy_entry,
            commands::file_ops::move_entry,
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
            commands::archive::compress,
            commands::archive::cancel_operation,
            commands::archive::extract,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
