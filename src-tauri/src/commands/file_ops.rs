use crate::fs_ops::{self, FileEntry};
use serde::Serialize;
use std::path::PathBuf;

#[tauri::command]
pub fn list_directory(path: String, show_hidden: bool) -> Result<Vec<FileEntry>, String> {
    let path = PathBuf::from(&path);
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }

    let entries = fs_ops::read_directory(&path)?;

    if show_hidden {
        Ok(entries)
    } else {
        Ok(entries.into_iter().filter(|e| !e.hidden).collect())
    }
}

#[tauri::command]
pub fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())
}

#[tauri::command]
pub fn create_directory(path: String) -> Result<(), String> {
    fs_ops::create_directory(&PathBuf::from(path))
}

#[tauri::command]
pub fn create_file(path: String) -> Result<(), String> {
    fs_ops::create_file(&PathBuf::from(path))
}

#[tauri::command]
pub fn rename_entry(from: String, to: String) -> Result<(), String> {
    fs_ops::rename_entry(&PathBuf::from(from), &PathBuf::from(to))
}

#[tauri::command]
pub fn delete_entry(path: String) -> Result<(), String> {
    fs_ops::delete_entry(&PathBuf::from(path))
}

#[tauri::command]
pub fn copy_entry(from: String, to: String) -> Result<(), String> {
    fs_ops::copy_entry(&PathBuf::from(from), &PathBuf::from(to))
}

#[tauri::command]
pub fn move_entry(from: String, to: String) -> Result<(), String> {
    fs_ops::move_entry(&PathBuf::from(from), &PathBuf::from(to))
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    PathBuf::from(path).exists()
}

#[derive(Clone, Serialize)]
pub struct DirStats {
    pub size: u64,
    pub contents_count: u64,
}

#[tauri::command]
pub async fn get_dir_stats(path: String) -> Result<DirStats, String> {
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&path);
        let (size, contents_count) = dir_size_and_count(&path)
            .map_err(|e| format!("Failed to scan directory: {e}"))?;
        Ok(DirStats { size, contents_count })
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub async fn get_properties(path: String) -> Result<FileProperties, String> {
    tokio::task::spawn_blocking(move || get_properties_sync(&path))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

fn get_properties_sync(path: &str) -> Result<FileProperties, String> {
    let path = PathBuf::from(path);
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to read metadata: {e}"))?;

    let symlink_meta = std::fs::symlink_metadata(&path).ok();
    let is_symlink = symlink_meta.as_ref().map(|m| m.file_type().is_symlink()).unwrap_or(false);

    let link_target = if is_symlink {
        std::fs::read_link(&path).ok().map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    let mime_type = if metadata.is_dir() {
        "inode/directory".to_string()
    } else {
        fs_ops::guess_mime(&path)
    };

    let created = metadata.created().ok().map(|t| {
        let dt: chrono::DateTime<chrono::Local> = t.into();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    });

    let modified = metadata.modified().ok().map(|t| {
        let dt: chrono::DateTime<chrono::Local> = t.into();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    });

    let accessed = metadata.accessed().ok().map(|t| {
        let dt: chrono::DateTime<chrono::Local> = t.into();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    });

    #[cfg(unix)]
    let (permissions, owner, group) = {
        use std::os::unix::fs::MetadataExt;
        let mode = metadata.mode();
        let uid = metadata.uid();
        let gid = metadata.gid();
        (
            format!("{:o}", mode & 0o7777),
            uid.to_string(),
            gid.to_string(),
        )
    };
    #[cfg(not(unix))]
    let (permissions, owner, group) = ("".to_string(), "".to_string(), "".to_string());

    let size = if metadata.is_dir() {
        0
    } else {
        metadata.len()
    };

    Ok(FileProperties {
        name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        path: path.to_string_lossy().to_string(),
        size,
        is_dir: metadata.is_dir(),
        is_symlink,
        link_target,
        mime_type,
        permissions,
        owner,
        group,
        created,
        modified,
        accessed,
        contents_count: None,
    })
}

#[derive(Debug, Serialize, Clone)]
pub struct FileProperties {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub link_target: Option<String>,
    pub mime_type: String,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub accessed: Option<String>,
    pub contents_count: Option<u64>,
}

pub fn dir_size_and_count(path: &PathBuf) -> Result<(u64, u64), std::io::Error> {
    let mut size = 0u64;
    let mut count = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        count += 1;
        if meta.is_dir() {
            let (s, c) = dir_size_and_count(&entry.path()).unwrap_or((0, 0));
            size += s;
            count += c;
        } else {
            size += meta.len();
        }
    }
    Ok((size, count))
}
