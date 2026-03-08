use chrono::{DateTime, Local};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: String,
    pub mime_type: String,
    pub permissions: u32,
    pub hidden: bool,
    pub children_count: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DriveEntry {
    pub name: String,
    pub path: String,
    pub fstype: String,
    pub removable: bool,
}

pub fn read_directory(path: &Path) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {e}"))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {e}"))?;

        let name = entry.file_name().to_string_lossy().to_string();
        let path_buf = entry.path();
        let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

        let modified = metadata
            .modified()
            .ok()
            .map(|t| {
                let dt: DateTime<Local> = t.into();
                dt.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_default();

        let mime_type = if metadata.is_dir() {
            "inode/directory".to_string()
        } else {
            guess_mime(&path_buf)
        };

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let permissions = 0u32;

        let hidden = name.starts_with('.');

        let children_count = if metadata.is_dir() {
            fs::read_dir(&path_buf).ok().map(|d| d.count() as u64)
        } else {
            None
        };

        files.push(FileEntry {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            is_symlink,
            size: metadata.len(),
            modified,
            mime_type,
            permissions,
            hidden,
            children_count,
        });
    }

    Ok(files)
}

pub fn guess_mime(path: &Path) -> String {
    // Try extension-based first (fast)
    let from_ext = mime_guess::from_path(path).first().map(|m| m.to_string());

    if let Some(mime) = from_ext {
        return mime;
    }

    // Fall back to magic bytes
    if let Ok(Some(kind)) = infer::get_from_path(path) {
        return kind.mime_type().to_string();
    }

    "application/octet-stream".to_string()
}

pub fn create_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {e}"))
}

pub fn create_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {e}"))?;
    }
    fs::File::create(path).map_err(|e| format!("Failed to create file: {e}"))?;
    Ok(())
}

pub fn rename_entry(from: &Path, to: &Path) -> Result<(), String> {
    fs::rename(from, to).map_err(|e| format!("Failed to rename: {e}"))
}

pub fn delete_entry(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("Failed to move to trash: {e}"))
}

pub fn copy_entry(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_dir() {
        copy_dir_recursive(from, to)
    } else {
        fs::copy(from, to)
            .map(|_| ())
            .map_err(|e| format!("Failed to copy file: {e}"))
    }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("Failed to create directory: {e}"))?;

    let entries = fs::read_dir(from).map_err(|e| format!("Failed to read directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let dest = to.join(entry.file_name());

        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest).map_err(|e| format!("Failed to copy file: {e}"))?;
        }
    }

    Ok(())
}

pub fn move_entry(from: &Path, to: &Path) -> Result<(), String> {
    // Try rename first (same filesystem, instant)
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // Fall back to copy + delete (cross-filesystem)
    copy_entry(from, to)?;
    if from.is_dir() {
        fs::remove_dir_all(from).map_err(|e| format!("Failed to remove source directory: {e}"))?;
    } else {
        fs::remove_file(from).map_err(|e| format!("Failed to remove source file: {e}"))?;
    }
    Ok(())
}
