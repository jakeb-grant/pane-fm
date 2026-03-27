use std::sync::Mutex;

use notify::{Event, EventKind, RecommendedWatcher, Watcher};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{load_config, AppConfig};
use crate::error::AppError;

pub struct ConfigWatcher(pub Mutex<Option<RecommendedWatcher>>);

#[tauri::command]
pub fn get_config() -> Result<AppConfig, AppError> {
    Ok(load_config())
}

#[tauri::command]
pub fn watch_config(app: AppHandle) -> Result<(), AppError> {
    let Some(config_dir) = dirs::config_dir() else {
        return Ok(());
    };
    let watch_dir = config_dir.join("pane-fm");
    if !watch_dir.is_dir() {
        return Ok(());
    }

    let config_path = watch_dir.join("config.toml");
    let emitter = app.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        let Ok(event) = res else { return };
        let dominated = event.paths.iter().any(|p| p == &config_path);
        if !dominated {
            return;
        }
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                let config = load_config();
                let _ = emitter.emit("config-changed", &config);
            }
            EventKind::Remove(_) => {
                let _ = emitter.emit("config-changed", &AppConfig::default());
            }
            _ => {}
        }
    })
    .map_err(|e| AppError::Io {
        message: format!("Failed to create config watcher: {e}"),
        path: None,
    })?;

    watcher
        .watch(&watch_dir, notify::RecursiveMode::NonRecursive)
        .map_err(|e| AppError::Io {
            message: format!("Failed to watch config: {e}"),
            path: Some(watch_dir.display().to_string()),
        })?;

    let state = app.state::<ConfigWatcher>();
    *state.0.lock().unwrap() = Some(watcher);
    Ok(())
}
