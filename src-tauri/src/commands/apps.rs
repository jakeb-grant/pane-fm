use crate::error::AppError;
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
pub fn open_default(path: String, app: tauri::AppHandle) -> Result<(), AppError> {
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| AppError::Desktop {
            message: format!("Failed to open: {e}"),
        })
}

#[tauri::command]
pub fn open_with_editor(path: String, editor: Option<String>) -> Result<(), AppError> {
    let editor = editor
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .ok_or_else(|| AppError::Desktop {
            message: "No editor configured — set 'editor' in config.toml or $EDITOR".to_string(),
        })?;

    let parts = shlex::split(&editor).ok_or_else(|| AppError::Desktop {
        message: format!("Invalid editor command syntax: {editor}"),
    })?;
    let (cmd, args) = parts
        .split_first()
        .ok_or_else(|| AppError::Desktop {
            message: "$EDITOR is empty".to_string(),
        })?;

    std::process::Command::new(cmd)
        .args(args.iter())
        .arg(&path)
        .spawn()
        .map_err(|e| AppError::Desktop {
            message: format!("Failed to open editor: {e}"),
        })?;

    Ok(())
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
pub fn open_with_app(path: String, desktop_id: String) -> Result<(), AppError> {
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

    let exec = exec_line.ok_or_else(|| AppError::Desktop {
        message: format!("Could not find Exec in {desktop_id}"),
    })?;

    // Shell-escape the path so spaces/quotes don't break the command
    let escaped = shlex::try_quote(&path).unwrap_or(std::borrow::Cow::Borrowed(&path));

    // Replace field codes with the escaped file path
    let cmd = exec
        .replace("%f", &escaped)
        .replace("%F", &escaped)
        .replace("%u", &escaped)
        .replace("%U", &escaped)
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "");

    // If no field code was present, append the path
    let cmd = if !exec.contains("%f")
        && !exec.contains("%F")
        && !exec.contains("%u")
        && !exec.contains("%U")
    {
        format!("{cmd} {escaped}")
    } else {
        cmd
    };

    let parts = shlex::split(&cmd).ok_or_else(|| AppError::Desktop {
        message: format!("Invalid Exec syntax in {desktop_id}"),
    })?;
    if parts.is_empty() {
        return Err(AppError::Desktop {
            message: "Empty Exec line".to_string(),
        });
    }

    std::process::Command::new(&parts[0])
        .args(&parts[1..])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| AppError::Desktop {
            message: format!("Failed to launch {}: {e}", parts[0]),
        })?;

    Ok(())
}

#[tauri::command]
pub fn open_terminal(path: String, terminal: String) -> Result<(), AppError> {
    std::process::Command::new(&terminal)
        .current_dir(&path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| AppError::Desktop {
            message: format!("Failed to launch terminal '{terminal}': {e}"),
        })?;
    Ok(())
}

#[tauri::command]
pub async fn run_custom_action(command: String, cwd: String, wait: bool) -> Result<(), AppError> {
    let parts = shlex::split(&command).ok_or_else(|| AppError::Desktop {
        message: format!("Invalid command syntax: {command}"),
    })?;
    if parts.is_empty() {
        return Err(AppError::Desktop {
            message: "Empty command".to_string(),
        });
    }

    let mut child = std::process::Command::new(&parts[0])
        .args(&parts[1..])
        .current_dir(&cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| AppError::Desktop {
            message: format!("Failed to run '{}': {e}", parts[0]),
        })?;

    if wait {
        tokio::task::spawn_blocking(move || {
            child.wait().map_err(|e| AppError::Desktop {
                message: format!("Command failed: {e}"),
            })
        })
        .await
        .map_err(|e| AppError::Desktop {
            message: format!("Task join error: {e}"),
        })??;
    }

    Ok(())
}

pub(crate) fn get_xdg_data_dirs() -> Vec<PathBuf> {
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

pub(crate) fn parse_desktop_file(data_dirs: &[PathBuf], desktop_id: &str) -> Option<AppEntry> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a fake XDG data dir with an applications/ subdirectory
    fn setup_data_dir(tmp: &TempDir) -> PathBuf {
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("applications")).unwrap();
        dir
    }

    #[test]
    fn parse_valid_desktop_file() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_data_dir(&tmp);

        fs::write(
            dir.join("applications/firefox.desktop"),
            "[Desktop Entry]\nName=Firefox\nExec=firefox %u\nIcon=firefox\nType=Application\n",
        )
        .unwrap();

        let dirs = vec![dir];
        let entry = parse_desktop_file(&dirs, "firefox.desktop");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.icon, "firefox");
        assert_eq!(entry.desktop_id, "firefox.desktop");
    }

    #[test]
    fn parse_desktop_file_missing_name() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_data_dir(&tmp);

        // No Name= field
        fs::write(
            dir.join("applications/noname.desktop"),
            "[Desktop Entry]\nExec=something\nIcon=thing\n",
        )
        .unwrap();

        let dirs = vec![dir];
        assert!(parse_desktop_file(&dirs, "noname.desktop").is_none());
    }

    #[test]
    fn parse_desktop_file_missing_exec() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_data_dir(&tmp);

        // Has Name but no Exec — parse_desktop_file only reads Name/Icon/NoDisplay
        fs::write(
            dir.join("applications/noexec.desktop"),
            "[Desktop Entry]\nName=NoExec App\nIcon=noexec\n",
        )
        .unwrap();

        let dirs = vec![dir];
        // parse_desktop_file should still return Some (it doesn't check Exec)
        let entry = parse_desktop_file(&dirs, "noexec.desktop");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().name, "NoExec App");
    }

    #[test]
    fn parse_desktop_file_no_display() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_data_dir(&tmp);

        fs::write(
            dir.join("applications/hidden.desktop"),
            "[Desktop Entry]\nName=Hidden\nNoDisplay=true\nIcon=hidden\n",
        )
        .unwrap();

        let dirs = vec![dir];
        assert!(parse_desktop_file(&dirs, "hidden.desktop").is_none());
    }

    #[test]
    fn parse_desktop_file_localized_entries() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_data_dir(&tmp);

        // Name[en] should not override Name=
        fs::write(
            dir.join("applications/localized.desktop"),
            "[Desktop Entry]\nName=Original\nName[en]=English\nIcon=loc\n",
        )
        .unwrap();

        let dirs = vec![dir];
        let entry = parse_desktop_file(&dirs, "localized.desktop").unwrap();
        // Name[en]= doesn't match strip_prefix("Name="), so original name is kept
        assert_eq!(entry.name, "Original");
    }

    #[test]
    fn parse_desktop_file_not_found() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_data_dir(&tmp);

        let dirs = vec![dir];
        assert!(parse_desktop_file(&dirs, "nonexistent.desktop").is_none());
    }

    #[test]
    fn get_xdg_data_dirs_returns_valid_paths() {
        let dirs = get_xdg_data_dirs();
        // Should always return at least one directory (from XDG_DATA_DIRS fallback)
        assert!(!dirs.is_empty());
    }
}
