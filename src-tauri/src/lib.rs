use tauri::Manager;

mod commands;
mod config;
mod error;
mod fs_ops;
pub mod progress;

/// Check localStorage SQLite files for corruption before WebKit opens them.
/// WebKit's localStorage uses SQLite with WAL mode. If the app is killed during
/// a write (e.g. USB disconnect, OOM kill), the WAL can become corrupted or
/// unreplayable, causing WebKit to hang on any localStorage access.
/// We detect this by checking if the WAL file is disproportionately large
/// relative to the main DB, and delete all three files to let WebKit start fresh.
fn repair_localstorage(app: &tauri::App) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let ls_dir = data_dir.join("localstorage");
    if !ls_dir.is_dir() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(&ls_dir) else {
        return;
    };

    // Find .localstorage files (the main SQLite DBs)
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".localstorage") || name.contains("-shm") || name.contains("-wal") {
            continue;
        }

        let wal_path = path.with_extension("localstorage-wal");
        let shm_path = path.with_extension("localstorage-shm");

        let db_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let wal_size = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

        // Heuristic: WAL > 50KB with a small DB means writes weren't checkpointed.
        // Normal operation keeps WAL near zero between checkpoints.
        if wal_size > 50_000 && wal_size > db_size * 5 {
            eprintln!(
                "pane-fm: localStorage corrupted (db={}B, wal={}B), resetting: {}",
                db_size,
                wal_size,
                name
            );
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(&wal_path);
            let _ = std::fs::remove_file(&shm_path);
        }
    }
}

/// Read the user's theme CSS and extract --bg-primary color.
/// Falls back to Catppuccin Mocha dark (#1e1e2e) if anything fails.
fn theme_bg_color() -> tauri::window::Color {
    let default = tauri::window::Color(30, 30, 46, 255);

    let cfg = config::load_config();
    let theme_name = match cfg.general.theme.as_deref() {
        Some(name) if !name.is_empty() => name,
        _ => return default,
    };

    let Some(path) = config::resolve_theme_path(theme_name) else {
        return default;
    };
    let Ok(css) = std::fs::read_to_string(path) else {
        return default;
    };

    parse_bg_primary(&css).unwrap_or(default)
}

fn parse_bg_opacity(css: &str) -> u8 {
    let Some(idx) = css.find("--bg-opacity:") else {
        return 255;
    };
    let rest = &css[idx..];
    // Extract the numeric value before '%'
    let Some(pct_pos) = rest.find('%') else {
        return 255;
    };
    let between = rest["--bg-opacity:".len()..pct_pos].trim();
    between
        .parse::<f64>()
        .map(|p| (p / 100.0 * 255.0).round() as u8)
        .unwrap_or(255)
}

fn parse_bg_primary(css: &str) -> Option<tauri::window::Color> {
    // Match --bg-primary: #rrggbb
    let idx = css.find("--bg-primary")?;
    let rest = &css[idx..];
    let hash = rest.find('#')?;
    let hex = &rest[hash + 1..];
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let a = parse_bg_opacity(css);
    if a < 255 {
        // Fully transparent window — let CSS color-mix handle the visual background
        Some(tauri::window::Color(0, 0, 0, 0))
    } else {
        Some(tauri::window::Color(r, g, b, 255))
    }
}

#[tauri::command]
fn show_window(window: tauri::WebviewWindow) {
    let _ = window.show();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .manage(commands::theme::ThemeWatcher(std::sync::Mutex::new(None)))
        .manage(commands::watcher::DirWatcher(std::sync::Mutex::new(None)))
        .manage(commands::config::ConfigWatcher(std::sync::Mutex::new(None)))
        .setup(|app| {
            config::install_default_config();
            commands::theme::install_default_themes();
            repair_localstorage(app);

            if let Some(window) = app.get_webview_window("main") {
                // Set window background to theme's --bg-primary to prevent white flash
                let bg = theme_bg_color();
                let _ = window.set_background_color(Some(bg));
                let _ = window.hide();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Work around a RefCell borrow conflict in tauri-runtime-wry on
            // Linux: the close event handler holds a mutable borrow on an
            // internal RefCell while GTK destroys signal handlers that need
            // the same RefCell, causing a panic.  We prevent the in-line
            // close, hide the window for immediate visual feedback, then
            // defer the real exit to the next main-loop iteration when the
            // borrow has been released.
            #[cfg(target_os = "linux")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let handle = window.app_handle().clone();
                gtk::glib::idle_add_once(move || {
                    handle.exit(0);
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::file_ops::list_directory,
            commands::file_ops::get_drag_icon,
            commands::file_ops::reset_drag_source,
            commands::file_ops::get_home_dir,
            commands::file_ops::create_directory,
            commands::file_ops::create_file,
            commands::file_ops::rename_entry,
            commands::file_ops::delete_entry,
            commands::file_ops::permanent_delete,
            commands::file_ops::create_symlink,
            commands::file_ops::chmod_entry,
            commands::file_ops::read_file_preview,
            commands::file_ops::read_pdf_preview,
            commands::file_ops::generate_thumbnail,
            commands::file_ops::paste_entries,
            commands::file_ops::delete_entries_permanently,
            commands::file_ops::path_exists,
            commands::file_ops::get_children_counts,
            commands::file_ops::get_properties,
            commands::file_ops::get_dir_stats,
            commands::drives::list_drives,
            commands::drives::mount_drive,
            commands::drives::unmount_drive,
            commands::trash::list_trash,
            commands::trash::restore_trash,
            commands::trash::empty_trash,
            commands::apps::open_default,
            commands::apps::open_with_editor,
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
            commands::file_ops::set_preview_path,
            commands::file_ops::stream_directory,
            commands::file_ops::cancel_stream_directory,
            commands::watcher::watch_directory,
            commands::watcher::unwatch_directory,
            show_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
