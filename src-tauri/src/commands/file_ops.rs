use crate::error::AppError;
use crate::fs_ops::{self, FileEntry, FilePreview};
use crate::progress;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn list_directory(path: String, show_hidden: bool) -> Result<Vec<FileEntry>, AppError> {
    let path = PathBuf::from(&path);
    if !path.is_dir() {
        return Err(AppError::NotFound {
            path: path.display().to_string(),
        });
    }

    let entries = fs_ops::read_directory(&path)?;

    if show_hidden {
        Ok(entries)
    } else {
        Ok(entries.into_iter().filter(|e| !e.hidden).collect())
    }
}

#[tauri::command]
pub fn get_drag_icon(app: AppHandle) -> Result<String, AppError> {
    app.path()
        .resolve("icons/drag.png", tauri::path::BaseDirectory::Resource)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| AppError::Io {
            message: format!("Could not resolve drag icon: {e}"),
            path: None,
        })
}

#[tauri::command]
pub fn get_home_dir() -> Result<String, AppError> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| AppError::Io {
            message: "Could not determine home directory".to_string(),
            path: None,
        })
}

#[tauri::command]
pub fn create_directory(path: String) -> Result<(), AppError> {
    fs_ops::create_directory(&PathBuf::from(path))
}

#[tauri::command]
pub fn create_file(path: String) -> Result<(), AppError> {
    fs_ops::create_file(&PathBuf::from(path))
}

#[tauri::command]
pub fn rename_entry(from: String, to: String) -> Result<(), AppError> {
    fs_ops::rename_entry(&PathBuf::from(from), &PathBuf::from(to))
}

#[tauri::command]
pub fn delete_entry(path: String) -> Result<(), AppError> {
    fs_ops::delete_entry(&PathBuf::from(path))
}

#[tauri::command]
pub fn permanent_delete(path: String) -> Result<(), AppError> {
    fs_ops::permanent_delete(&PathBuf::from(path))
}

#[tauri::command]
pub fn copy_entry(from: String, to: String) -> Result<(), AppError> {
    fs_ops::copy_entry(&PathBuf::from(from), &PathBuf::from(to))
}

#[tauri::command]
pub fn move_entry(from: String, to: String) -> Result<(), AppError> {
    fs_ops::move_entry(&PathBuf::from(from), &PathBuf::from(to))
}

#[tauri::command]
pub fn create_symlink(target: String, link: String) -> Result<(), AppError> {
    fs_ops::create_symlink(&PathBuf::from(target), &PathBuf::from(link))
}

#[tauri::command]
pub async fn paste_entries(
    paths: Vec<String>,
    dest: String,
    mode: String,
    app: AppHandle,
) -> Result<(), AppError> {
    progress::reset();
    let sources: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let dest_dir = PathBuf::from(&dest);
    tokio::task::spawn_blocking(move || {
        if mode == "cut" {
            fs_ops::move_entries_with_progress(&sources, &dest_dir, &app)
        } else {
            fs_ops::copy_entries_with_progress(&sources, &dest_dir, &app)
        }
    })
    .await
    .map_err(|e| AppError::Io {
        message: format!("Task failed: {e}"),
        path: None,
    })?
}

#[tauri::command]
pub async fn delete_entries_permanently(
    paths: Vec<String>,
    app: AppHandle,
) -> Result<(), AppError> {
    progress::reset();
    let sources: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    tokio::task::spawn_blocking(move || {
        fs_ops::permanent_delete_with_progress(&sources, &app)
    })
    .await
    .map_err(|e| AppError::Io {
        message: format!("Task failed: {e}"),
        path: None,
    })?
}

#[tauri::command]
pub fn read_file_preview(path: String, max_bytes: usize) -> Result<FilePreview, AppError> {
    fs_ops::read_file_preview(&PathBuf::from(path), max_bytes)
}

#[tauri::command]
pub async fn read_pdf_preview(path: String) -> Result<fs_ops::PdfPreview, AppError> {
    let path = PathBuf::from(path);
    tokio::task::spawn_blocking(move || fs_ops::render_pdf_preview(&path))
        .await
        .map_err(|e| AppError::Desktop {
            message: format!("Task join error: {e}"),
        })?
}

#[tauri::command]
pub async fn generate_thumbnail(
    path: String,
    max_dim: u32,
) -> Result<fs_ops::ImageThumbnail, AppError> {
    let path = PathBuf::from(path);
    tokio::task::spawn_blocking(move || fs_ops::generate_thumbnail(&path, max_dim))
        .await
        .map_err(|e| AppError::Desktop {
            message: format!("Task join error: {e}"),
        })?
}

#[tauri::command]
pub fn chmod_entry(path: String, mode: u32) -> Result<(), AppError> {
    fs_ops::chmod_entry(&PathBuf::from(path), mode)
}

#[tauri::command]
pub async fn get_children_counts(paths: Vec<String>) -> std::collections::HashMap<String, u64> {
    tokio::task::spawn_blocking(move || fs_ops::get_children_counts(&paths))
        .await
        .unwrap_or_default()
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
pub async fn get_dir_stats(path: String, app: AppHandle) -> Result<DirStats, AppError> {
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&path);
        let mut size = 0u64;
        let mut contents_count = 0u64;
        let mut since_emit = 0u64;
        dir_size_and_count_progressive(&path, &mut size, &mut contents_count, &mut since_emit, &app)
            .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
        // Final emit to ensure frontend gets the exact total
        let _ = app.emit("dir-stats-progress", DirStats { size, contents_count });
        Ok(DirStats { size, contents_count })
    })
    .await
    .map_err(|e| AppError::Io {
        message: format!("Task failed: {e}"),
        path: None,
    })?
}

#[tauri::command]
pub async fn get_properties(path: String) -> Result<FileProperties, AppError> {
    tokio::task::spawn_blocking(move || get_properties_sync(&path))
        .await
        .map_err(|e| AppError::Io {
            message: format!("Task failed: {e}"),
            path: None,
        })?
}

fn get_properties_sync(path: &str) -> Result<FileProperties, AppError> {
    let path = PathBuf::from(path);
    let metadata = std::fs::metadata(&path)
        .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;

    let symlink_meta = std::fs::symlink_metadata(&path).ok();
    let is_symlink = symlink_meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    let link_target = if is_symlink {
        std::fs::read_link(&path)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
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

    let size = if metadata.is_dir() { 0 } else { metadata.len() };

    Ok(FileProperties {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
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

fn dir_size_and_count_progressive(
    path: &PathBuf,
    size: &mut u64,
    count: &mut u64,
    since_emit: &mut u64,
    app: &AppHandle,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        *count += 1;
        *since_emit += 1;
        if meta.is_dir() {
            dir_size_and_count_progressive(&entry.path(), size, count, since_emit, app).unwrap_or(());
        } else {
            *size += meta.len();
        }
        if *since_emit >= 100 {
            *since_emit = 0;
            let _ = app.emit("dir-stats-progress", DirStats { size: *size, contents_count: *count });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn dir_size_and_count_accuracy() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "hello").unwrap(); // 5 bytes
        fs::write(tmp.path().join("b.txt"), "world!").unwrap(); // 6 bytes
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub/c.txt"), "nested").unwrap(); // 6 bytes

        let (size, count) = dir_size_and_count(&tmp.path().to_path_buf()).unwrap();
        assert_eq!(size, 17); // 5 + 6 + 6
        assert_eq!(count, 4); // a.txt, b.txt, sub/, sub/c.txt
    }

    #[test]
    fn dir_size_and_count_empty() {
        let tmp = TempDir::new().unwrap();
        let (size, count) = dir_size_and_count(&tmp.path().to_path_buf()).unwrap();
        assert_eq!(size, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn dir_size_and_count_nonexistent() {
        let result = dir_size_and_count(&PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }
}
