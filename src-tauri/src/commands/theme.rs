use std::sync::Mutex;

use notify::{Event, EventKind, RecommendedWatcher, Watcher};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::resolve_theme_path;
use crate::error::AppError;

pub struct ThemeWatcher(pub Mutex<Option<RecommendedWatcher>>);

#[tauri::command]
pub fn load_theme_css(path: String) -> Result<String, AppError> {
    let resolved = resolve_theme_path(&path).ok_or_else(|| AppError::NotFound {
        path: path.clone(),
    })?;
    std::fs::read_to_string(&resolved).map_err(|e| AppError::io_with_path(e, resolved.display().to_string()))
}

#[tauri::command]
pub fn watch_theme(path: String, app: AppHandle) -> Result<(), AppError> {
    let resolved = resolve_theme_path(&path).ok_or_else(|| AppError::NotFound {
        path: path.clone(),
    })?;

    let watch_path = resolved.clone();
    let emitter = app.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        let Ok(event) = res else { return };
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                if let Ok(css) = std::fs::read_to_string(&resolved) {
                    let _ = emitter.emit("theme-changed", css);
                }
            }
            EventKind::Remove(_) => {
                let _ = emitter.emit("theme-changed", String::new());
            }
            _ => {}
        }
    })
    .map_err(|e| AppError::Io {
        message: format!("Failed to create file watcher: {e}"),
        path: None,
    })?;

    watcher
        .watch(&watch_path, notify::RecursiveMode::NonRecursive)
        .map_err(|e| AppError::Io {
            message: format!("Failed to watch theme file: {e}"),
            path: Some(watch_path.display().to_string()),
        })?;

    let state = app.state::<ThemeWatcher>();
    let mut guard = state.0.lock().unwrap();
    *guard = Some(watcher);

    Ok(())
}

pub fn install_default_themes() {
    let Some(config_dir) = dirs::config_dir() else {
        return;
    };
    let themes_dir = config_dir.join("hyprfiles").join("themes");
    if themes_dir.exists() {
        return;
    }
    if std::fs::create_dir_all(&themes_dir).is_err() {
        return;
    }

    let defaults: &[(&str, &str)] = &[
        ("catppuccin-mocha.css", include_str!("../../themes/catppuccin-mocha.css")),
        ("nord.css", include_str!("../../themes/nord.css")),
        ("dark-minimal.css", include_str!("../../themes/dark-minimal.css")),
    ];

    for (name, content) in defaults {
        let _ = std::fs::write(themes_dir.join(name), content);
    }
}
