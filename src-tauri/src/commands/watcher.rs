use std::path::PathBuf;
use std::sync::Mutex;

use notify::{EventKind, RecommendedWatcher, Watcher};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;

pub struct DirWatcher(pub Mutex<Option<RecommendedWatcher>>);

#[tauri::command]
pub fn watch_directory(path: String, app: AppHandle) -> Result<(), AppError> {
    let watch_path = PathBuf::from(&path);
    if !watch_path.is_dir() {
        return Err(AppError::NotFound { path });
    }

    let emitter = app.clone();
    let watched = path.clone();
    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            let Ok(event) = res else { return };
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    let _ = emitter.emit("directory-changed", &watched);
                }
                _ => {}
            }
        })
        .map_err(|e| AppError::Io {
            message: format!("Failed to create directory watcher: {e}"),
            path: None,
        })?;

    watcher
        .watch(&watch_path, notify::RecursiveMode::NonRecursive)
        .map_err(|e| AppError::Io {
            message: format!("Failed to watch directory: {e}"),
            path: Some(path),
        })?;

    // Dropping the old watcher stops the old watch
    let state = app.state::<DirWatcher>();
    *state.0.lock().unwrap() = Some(watcher);

    Ok(())
}

#[tauri::command]
pub fn unwatch_directory(app: AppHandle) {
    let state = app.state::<DirWatcher>();
    *state.0.lock().unwrap() = None;
}
