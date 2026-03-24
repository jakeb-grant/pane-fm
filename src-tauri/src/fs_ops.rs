use crate::error::AppError;
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

pub fn read_directory(path: &Path) -> Result<Vec<FileEntry>, AppError> {
    let entries = fs::read_dir(path).map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // DirEntry::file_type uses d_type from readdir (no stat syscall).
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        // Skip device files, sockets, FIFOs — stat/lstat can block on these.
        if !ft.is_file() && !ft.is_dir() && !ft.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let path_buf = entry.path();
        let is_symlink = ft.is_symlink();

        // For symlinks, lstat can block on special targets (e.g. /dev/stderr
        // -> /proc/self/fd/2). Build a minimal entry without stat.
        // For regular files/dirs, symlink_metadata (lstat) is safe.
        if is_symlink {
            // Resolve symlink target metadata for is_dir/size (stat is safe,
            // only file reads block on special targets like /proc/self/fd/*).
            let target_meta = fs::metadata(&path_buf).ok();
            let target_is_dir = target_meta.as_ref().is_some_and(|m| m.is_dir());
            let size = target_meta.as_ref().map_or(0, |m| m.len());

            let mime_type = if target_is_dir {
                "inode/directory".to_string()
            } else {
                // Extension-only mime guess — never open the file,
                // as the target may be a device/pipe that blocks on read.
                mime_guess::from_path(&path_buf)
                    .first()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string())
            };

            files.push(FileEntry {
                name: name.clone(),
                path: path_buf.to_string_lossy().to_string(),
                is_dir: target_is_dir,
                is_symlink: true,
                size,
                modified: String::new(),
                mime_type,
                permissions: 0,
                hidden: name.starts_with('.'),
                children_count: None,
            });
            continue;
        }

        let metadata = match fs::symlink_metadata(&path_buf) {
            Ok(m) => m,
            Err(_) => continue,
        };

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

        files.push(FileEntry {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            is_symlink: false,
            size: metadata.len(),
            modified,
            mime_type,
            permissions,
            hidden,
            children_count: None,
        });
    }

    Ok(files)
}

/// Count children for a batch of directory paths.
/// Returns a map of path -> count. Paths that fail are omitted.
pub fn get_children_counts(paths: &[String]) -> std::collections::HashMap<String, u64> {
    paths
        .iter()
        .filter_map(|p| {
            let count = fs::read_dir(p).ok()?.count() as u64;
            Some((p.clone(), count))
        })
        .collect()
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

pub fn create_directory(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|e| AppError::io_with_path(e, path.display().to_string()))
}

pub fn create_file(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::io_with_path(e, parent.display().to_string()))?;
    }
    fs::File::create(path).map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
    Ok(())
}

pub fn rename_entry(from: &Path, to: &Path) -> Result<(), AppError> {
    if from != to && to.exists() {
        return Err(AppError::AlreadyExists {
            path: to.display().to_string(),
        });
    }
    fs::rename(from, to).map_err(|e| AppError::io_with_path(e, from.display().to_string()))
}

pub fn delete_entry(path: &Path) -> Result<(), AppError> {
    trash::delete(path).map_err(|e| AppError::Trash {
        message: format!("Failed to move to trash: {e}"),
    })
}

pub fn permanent_delete(path: &Path) -> Result<(), AppError> {
    let meta =
        fs::symlink_metadata(path).map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|e| AppError::io_with_path(e, path.display().to_string()))
}

/// Returns a unique destination path by appending " (N)" if `to` already exists.
/// For files, the suffix is inserted before the extension: `foo (2).txt`.
/// For directories (or extensionless files): `foo (2)`.
pub fn unique_dest_path(to: &Path) -> std::path::PathBuf {
    if !to.exists() {
        return to.to_path_buf();
    }
    let stem = to
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = to.extension().map(|e| e.to_string_lossy().to_string());
    let parent = to.parent().unwrap_or(Path::new("/"));

    for n in 2..u32::MAX {
        let name = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = parent.join(&name);
        if !candidate.exists() {
            return candidate;
        }
    }
    to.to_path_buf()
}

pub fn copy_entry(from: &Path, to: &Path) -> Result<(), AppError> {
    let dest = unique_dest_path(to);
    if from.is_dir() {
        copy_dir_recursive(from, &dest)
    } else {
        fs::copy(from, &dest)
            .map(|_| ())
            .map_err(|e| AppError::io_with_path(e, from.display().to_string()))
    }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::create_dir_all(to).map_err(|e| AppError::io_with_path(e, to.display().to_string()))?;

    let entries =
        fs::read_dir(from).map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;

    for entry in entries {
        let entry = entry?;
        let dest = to.join(entry.file_name());

        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest)
                .map_err(|e| AppError::io_with_path(e, entry.path().display().to_string()))?;
        }
    }

    Ok(())
}

pub fn move_entry(from: &Path, to: &Path) -> Result<(), AppError> {
    let dest = unique_dest_path(to);
    // Try rename first (same filesystem, instant)
    if fs::rename(from, &dest).is_ok() {
        return Ok(());
    }
    // Fall back to copy + delete (cross-filesystem)
    // copy_entry already calls unique_dest_path, but dest is already unique here
    if from.is_dir() {
        copy_dir_recursive(from, &dest)?;
    } else {
        fs::copy(from, &dest)
            .map(|_| ())
            .map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;
    }
    if from.is_dir() {
        fs::remove_dir_all(from)
            .map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;
    } else {
        fs::remove_file(from)
            .map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;
    }
    Ok(())
}

pub fn create_symlink(target: &Path, link: &Path) -> Result<(), AppError> {
    let dest = unique_dest_path(link);
    std::os::unix::fs::symlink(target, &dest)
        .map_err(|e| AppError::io_with_path(e, dest.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn read_directory_basic() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "content").unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let entries = read_directory(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2);

        let file = entries.iter().find(|e| e.name == "file.txt").unwrap();
        assert!(!file.is_dir);
        assert_eq!(file.size, 7); // "content".len()

        let dir = entries.iter().find(|e| e.name == "subdir").unwrap();
        assert!(dir.is_dir);
    }

    #[test]
    fn read_directory_hidden_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible"), "").unwrap();

        let entries = read_directory(tmp.path()).unwrap();
        let hidden = entries.iter().find(|e| e.name == ".hidden").unwrap();
        assert!(hidden.hidden);

        let visible = entries.iter().find(|e| e.name == "visible").unwrap();
        assert!(!visible.hidden);
    }

    #[test]
    fn read_directory_symlinks() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("target.txt"), "data").unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                tmp.path().join("target.txt"),
                tmp.path().join("link.txt"),
            )
            .unwrap();

            let entries = read_directory(tmp.path()).unwrap();
            let link = entries.iter().find(|e| e.name == "link.txt").unwrap();
            assert!(link.is_symlink);
        }
    }

    #[test]
    fn read_directory_nonexistent() {
        let result = read_directory(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn read_directory_with_spaces_in_path() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("path with spaces");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("file.txt"), "hello").unwrap();

        let entries = read_directory(&dir).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file.txt");
    }

    #[test]
    fn read_directory_root() {
        // Reading / should succeed and return entries
        let entries = read_directory(Path::new("/")).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn create_and_delete_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("newdir");

        create_directory(&dir).unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn create_and_delete_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("newfile.txt");

        create_file(&file).unwrap();
        assert!(file.exists());
        assert!(file.is_file());
    }

    #[test]
    fn create_file_with_nested_parent() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("a/b/c/deep.txt");

        create_file(&file).unwrap();
        assert!(file.exists());
    }

    #[test]
    fn rename_entry_works() {
        let tmp = TempDir::new().unwrap();
        let from = tmp.path().join("old.txt");
        let to = tmp.path().join("new.txt");

        fs::write(&from, "data").unwrap();
        rename_entry(&from, &to).unwrap();

        assert!(!from.exists());
        assert!(to.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "data");
    }

    #[test]
    fn copy_entry_file() {
        let tmp = TempDir::new().unwrap();
        let from = tmp.path().join("src.txt");
        let to = tmp.path().join("dst.txt");

        fs::write(&from, "content").unwrap();
        copy_entry(&from, &to).unwrap();

        assert!(from.exists()); // source still exists
        assert!(to.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "content");
    }

    #[test]
    fn copy_entry_directory() {
        let tmp = TempDir::new().unwrap();
        let from = tmp.path().join("srcdir");
        let to = tmp.path().join("dstdir");

        fs::create_dir(&from).unwrap();
        fs::write(from.join("file.txt"), "hello").unwrap();
        fs::create_dir(from.join("sub")).unwrap();
        fs::write(from.join("sub/nested.txt"), "world").unwrap();

        copy_entry(&from, &to).unwrap();

        assert!(to.join("file.txt").exists());
        assert_eq!(fs::read_to_string(to.join("file.txt")).unwrap(), "hello");
        assert!(to.join("sub/nested.txt").exists());
        assert_eq!(
            fs::read_to_string(to.join("sub/nested.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn move_entry_file() {
        let tmp = TempDir::new().unwrap();
        let from = tmp.path().join("src.txt");
        let to = tmp.path().join("dst.txt");

        fs::write(&from, "content").unwrap();
        move_entry(&from, &to).unwrap();

        assert!(!from.exists()); // source removed
        assert!(to.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "content");
    }

    #[test]
    fn unique_dest_path_no_conflict() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        assert_eq!(unique_dest_path(&target), target);
    }

    #[test]
    fn unique_dest_path_with_conflict() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        fs::write(&target, "").unwrap();

        let result = unique_dest_path(&target);
        assert_eq!(result, tmp.path().join("file (2).txt"));
    }

    #[test]
    fn unique_dest_path_multiple_conflicts() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "").unwrap();
        fs::write(tmp.path().join("file (2).txt"), "").unwrap();
        fs::write(tmp.path().join("file (3).txt"), "").unwrap();

        let result = unique_dest_path(&tmp.path().join("file.txt"));
        assert_eq!(result, tmp.path().join("file (4).txt"));
    }

    #[test]
    fn unique_dest_path_no_extension() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("folder");
        fs::create_dir(&target).unwrap();

        let result = unique_dest_path(&target);
        assert_eq!(result, tmp.path().join("folder (2)"));
    }

    #[test]
    fn copy_entry_same_dir_auto_renames() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "original").unwrap();

        copy_entry(&file, &file).unwrap();

        assert!(file.exists());
        let copy = tmp.path().join("test (2).txt");
        assert!(copy.exists());
        assert_eq!(fs::read_to_string(&copy).unwrap(), "original");
    }

    #[test]
    fn guess_mime_known_extension() {
        let path = Path::new("test.png");
        assert_eq!(guess_mime(path), "image/png");
    }

    #[test]
    fn guess_mime_unknown_extension() {
        let path = Path::new("test.unknownext123");
        // Should fall back to octet-stream
        assert_eq!(guess_mime(path), "application/octet-stream");
    }

    #[test]
    fn create_symlink_basic() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("original.txt");
        fs::write(&target, "content").unwrap();
        let link = tmp.path().join("original.txt (link)");
        create_symlink(&target, &link).unwrap();
        assert!(link.is_symlink());
        assert_eq!(fs::read_to_string(&link).unwrap(), "content");
    }

    #[test]
    fn create_symlink_collision() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("notes");
        fs::create_dir(&target).unwrap();
        let link = tmp.path().join("notes (link)");
        fs::write(&link, "occupied").unwrap();
        create_symlink(&target, &link).unwrap();
        let expected = tmp.path().join("notes (link) (2)");
        assert!(expected.is_symlink());
    }
}
