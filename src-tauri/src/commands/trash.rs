use crate::fs_ops::{self, FileEntry};
use chrono::{DateTime, Local};
use std::path::PathBuf;

#[tauri::command]
pub fn list_trash() -> Result<Vec<FileEntry>, String> {
    let trash_files = dirs::data_dir()
        .ok_or("Could not find data directory")?
        .join("Trash/files");

    let trash_info = dirs::data_dir()
        .ok_or("Could not find data directory")?
        .join("Trash/info");

    if !trash_files.exists() {
        return Ok(Vec::new());
    }

    let entries =
        std::fs::read_dir(&trash_files).map_err(|e| format!("Failed to read trash: {e}"))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let metadata = entry.metadata().map_err(|e| format!("Failed to read metadata: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path_buf = entry.path();

        // Try to get the original deletion date from .trashinfo
        let info_file = trash_info.join(format!("{name}.trashinfo"));
        let modified = if let Ok(info_content) = std::fs::read_to_string(&info_file) {
            info_content
                .lines()
                .find(|l| l.starts_with("DeletionDate="))
                .map(|l| l.trim_start_matches("DeletionDate=").to_string())
                .unwrap_or_default()
        } else {
            metadata
                .modified()
                .ok()
                .map(|t| {
                    let dt: DateTime<Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M").to_string()
                })
                .unwrap_or_default()
        };

        let mime_type = if metadata.is_dir() {
            "inode/directory".to_string()
        } else {
            fs_ops::guess_mime(&path_buf)
        };

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let permissions = 0u32;

        let children_count = if metadata.is_dir() {
            std::fs::read_dir(&path_buf).ok().map(|d| d.count() as u64)
        } else {
            None
        };

        files.push(FileEntry {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            is_symlink: false,
            size: metadata.len(),
            modified,
            mime_type,
            permissions,
            hidden: false,
            children_count,
        });
    }

    Ok(files)
}

#[tauri::command]
pub fn restore_trash(name: String) -> Result<(), String> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    let trash_file = data_dir.join("Trash/files").join(&name);
    let info_file = data_dir.join("Trash/info").join(format!("{name}.trashinfo"));

    // Read original path from .trashinfo
    let info = std::fs::read_to_string(&info_file)
        .map_err(|e| format!("Failed to read trash info: {e}"))?;

    let original_path = info
        .lines()
        .find(|l| l.starts_with("Path="))
        .map(|l| l.trim_start_matches("Path="))
        .ok_or("Could not find original path in trash info")?;

    // URL-decode the path
    let decoded = percent_decode(original_path);
    let dest = PathBuf::from(&decoded);

    if dest.exists() {
        return Err(format!("Destination already exists: {decoded}"));
    }

    std::fs::rename(&trash_file, &dest)
        .map_err(|e| format!("Failed to restore: {e}"))?;

    // Remove the .trashinfo file
    let _ = std::fs::remove_file(&info_file);

    Ok(())
}

#[tauri::command]
pub fn empty_trash() -> Result<(), String> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    let trash_files = data_dir.join("Trash/files");
    let trash_info = data_dir.join("Trash/info");

    if trash_files.exists() {
        for entry in std::fs::read_dir(&trash_files)
            .map_err(|e| format!("Failed to read trash: {e}"))?
        {
            let entry = entry.map_err(|e| format!("{e}"))?;
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(|e| format!("{e}"))?;
            } else {
                std::fs::remove_file(&path).map_err(|e| format!("{e}"))?;
            }
        }
    }

    if trash_info.exists() {
        for entry in std::fs::read_dir(&trash_info)
            .map_err(|e| format!("Failed to read trash info: {e}"))?
        {
            let entry = entry.map_err(|e| format!("{e}"))?;
            let _ = std::fs::remove_file(entry.path());
        }
    }

    Ok(())
}

fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &s[i + 1..i + 3],
                16,
            ) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}
