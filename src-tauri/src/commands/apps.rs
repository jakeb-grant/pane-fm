use serde::Serialize;
use std::path::PathBuf;
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub desktop_id: String,
    pub icon: String,
}

#[tauri::command]
pub fn open_default(path: String, app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| format!("Failed to open: {e}"))
}

#[tauri::command]
pub fn list_apps_for_mime(mime_type: String) -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let data_dirs = get_xdg_data_dirs();

    // Read mimeinfo.cache from each applications directory
    for dir in &data_dirs {
        let cache_path = dir.join("applications/mimeinfo.cache");
        let Ok(content) = std::fs::read_to_string(&cache_path) else {
            continue;
        };

        for line in content.lines() {
            if let Some(rest) = line.strip_prefix(&format!("{mime_type}=")) {
                for desktop_id in rest.split(';').filter(|s| !s.is_empty()) {
                    if seen.contains(desktop_id) {
                        continue;
                    }
                    seen.insert(desktop_id.to_string());

                    if let Some(entry) = parse_desktop_file(&data_dirs, desktop_id) {
                        apps.push(entry);
                    }
                }
            }
        }
    }

    apps
}

#[tauri::command]
pub fn open_with_app(path: String, desktop_id: String) -> Result<(), String> {
    let data_dirs = get_xdg_data_dirs();

    // Find and parse the .desktop file
    let mut exec_line = None;
    for dir in &data_dirs {
        let desktop_path = dir.join("applications").join(&desktop_id);
        if let Ok(content) = std::fs::read_to_string(&desktop_path) {
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("Exec=") {
                    exec_line = Some(val.to_string());
                    break;
                }
            }
            if exec_line.is_some() {
                break;
            }
        }
    }

    let exec = exec_line.ok_or_else(|| format!("Could not find Exec in {desktop_id}"))?;

    // Replace field codes with the file path
    let cmd = exec
        .replace("%f", &path)
        .replace("%F", &path)
        .replace("%u", &path)
        .replace("%U", &path)
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "");

    // If no field code was present, append the path
    let cmd = if !exec.contains("%f")
        && !exec.contains("%F")
        && !exec.contains("%u")
        && !exec.contains("%U")
    {
        format!("{cmd} {path}")
    } else {
        cmd
    };

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty Exec line".to_string());
    }

    std::process::Command::new(parts[0])
        .args(&parts[1..])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch: {e}"))?;

    Ok(())
}

fn get_xdg_data_dirs() -> Vec<PathBuf> {
    let mut dirs_list = Vec::new();

    if let Some(data_home) = dirs::data_dir() {
        dirs_list.push(data_home);
    }

    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in data_dirs.split(':') {
        dirs_list.push(PathBuf::from(dir));
    }

    dirs_list
}

fn parse_desktop_file(data_dirs: &[PathBuf], desktop_id: &str) -> Option<AppEntry> {
    for dir in data_dirs {
        let path = dir.join("applications").join(desktop_id);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let mut name = String::new();
            let mut icon = String::new();
            let mut no_display = false;

            for line in content.lines() {
                if let Some(val) = line.strip_prefix("Name=") {
                    if name.is_empty() {
                        name = val.to_string();
                    }
                } else if let Some(val) = line.strip_prefix("Icon=") {
                    if icon.is_empty() {
                        icon = val.to_string();
                    }
                } else if line == "NoDisplay=true" {
                    no_display = true;
                }
            }

            if no_display || name.is_empty() {
                continue;
            }

            return Some(AppEntry {
                name,
                desktop_id: desktop_id.to_string(),
                icon,
            });
        }
    }
    None
}
