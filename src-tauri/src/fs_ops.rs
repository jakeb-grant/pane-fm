use crate::error::AppError;
use crate::progress;
use chrono::{DateTime, Local};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

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

// --- Progress-aware batch operations ---

fn copy_file_with_progress(
    from: &Path,
    to: &Path,
    processed: &mut u64,
    total: u64,
    app: &AppHandle,
) -> Result<(), AppError> {
    let mut src = fs::File::open(from)
        .map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;
    let mut dst = fs::File::create(to)
        .map_err(|e| AppError::io_with_path(e, to.display().to_string()))?;
    let mut buf = [0u8; 65536];
    loop {
        progress::check_cancelled_err()?;
        let n = src
            .read(&mut buf)
            .map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut dst, &buf[..n])
            .map_err(|e| AppError::io_with_path(e, to.display().to_string()))?;
        *processed += n as u64;
        progress::emit(app, *processed, total);
    }
    Ok(())
}

fn copy_dir_recursive_with_progress(
    from: &Path,
    to: &Path,
    processed: &mut u64,
    total: u64,
    app: &AppHandle,
) -> Result<(), AppError> {
    fs::create_dir_all(to).map_err(|e| AppError::io_with_path(e, to.display().to_string()))?;
    let entries =
        fs::read_dir(from).map_err(|e| AppError::io_with_path(e, from.display().to_string()))?;
    for entry in entries {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive_with_progress(&entry.path(), &dest, processed, total, app)?;
        } else {
            copy_file_with_progress(&entry.path(), &dest, processed, total, app)?;
        }
    }
    Ok(())
}

fn dir_total_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    total += dir_total_bytes(&entry.path());
                } else {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn prescan_bytes<P: AsRef<Path>>(sources: &[P]) -> u64 {
    sources
        .iter()
        .map(|p| {
            let p = p.as_ref();
            if p.is_dir() {
                dir_total_bytes(p)
            } else {
                fs::metadata(p).map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

pub fn copy_entries_with_progress(
    sources: &[PathBuf],
    dest_dir: &Path,
    app: &AppHandle,
) -> Result<(), AppError> {
    let total = prescan_bytes(sources);
    let mut processed = 0u64;
    for src in sources {
        let name = src.file_name().unwrap_or_default();
        let dest = unique_dest_path(&dest_dir.join(name));
        if src.is_dir() {
            let result =
                copy_dir_recursive_with_progress(src, &dest, &mut processed, total, app);
            if result.is_err() && progress::is_cancelled() {
                let _ = fs::remove_dir_all(&dest);
                return Err(AppError::Cancelled);
            }
            result?;
        } else {
            let result = copy_file_with_progress(src, &dest, &mut processed, total, app);
            if result.is_err() && progress::is_cancelled() {
                let _ = fs::remove_file(&dest);
                return Err(AppError::Cancelled);
            }
            result?;
        }
    }
    Ok(())
}

pub fn move_entries_with_progress(
    sources: &[PathBuf],
    dest_dir: &Path,
    app: &AppHandle,
) -> Result<(), AppError> {
    // Try rename first for each source (instant on same filesystem)
    let mut needs_copy: Vec<&PathBuf> = Vec::new();
    for src in sources {
        let name = src.file_name().unwrap_or_default();
        let dest = unique_dest_path(&dest_dir.join(name));
        match fs::rename(src, &dest) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(18) => {
                // EXDEV: cross-device move, need copy+delete
                needs_copy.push(src);
            }
            Err(e) => {
                return Err(AppError::io_with_path(e, src.display().to_string()));
            }
        }
    }

    if needs_copy.is_empty() {
        return Ok(());
    }

    // Fall back to copy + delete for cross-device entries
    let total = prescan_bytes(&needs_copy);
    let mut processed = 0u64;
    for src in &needs_copy {
        let name = src.file_name().unwrap_or_default();
        let dest = unique_dest_path(&dest_dir.join(name));
        let copy_result = if src.is_dir() {
            copy_dir_recursive_with_progress(src, &dest, &mut processed, total, app)
        } else {
            copy_file_with_progress(src, &dest, &mut processed, total, app)
        };
        if copy_result.is_err() && progress::is_cancelled() {
            // Clean up partial destination on cancellation
            if dest.is_dir() {
                let _ = fs::remove_dir_all(&dest);
            } else {
                let _ = fs::remove_file(&dest);
            }
            return Err(AppError::Cancelled);
        }
        copy_result?;
        // Only delete source after successful copy
        permanent_delete(src)?;
    }
    Ok(())
}

fn count_items(path: &Path) -> u64 {
    if !path.is_dir() {
        return 1;
    }
    let mut count = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            count += count_items(&entry.path());
        }
    }
    count + 1 // +1 for the directory itself
}

fn delete_recursive_with_progress(
    path: &Path,
    processed: &mut u64,
    total: u64,
    app: &AppHandle,
) -> Result<(), AppError> {
    progress::check_cancelled_err()?;
    if path.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
        for entry in entries {
            let entry = entry?;
            delete_recursive_with_progress(&entry.path(), processed, total, app)?;
        }
        fs::remove_dir(path)
            .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
    } else {
        fs::remove_file(path)
            .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
    }
    *processed += 1;
    progress::emit(app, *processed, total);
    Ok(())
}

pub fn permanent_delete_with_progress(
    paths: &[PathBuf],
    app: &AppHandle,
) -> Result<(), AppError> {
    let total: u64 = paths.iter().map(|p| count_items(p)).sum();
    let mut processed = 0u64;
    for path in paths {
        delete_recursive_with_progress(path, &mut processed, total, app)?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
pub struct FilePreview {
    pub content: String,
    pub truncated: bool,
    pub bytes_read: usize,
    pub is_binary: bool,
}

pub fn read_file_preview(path: &Path, max_bytes: usize) -> Result<FilePreview, AppError> {
    let file =
        fs::File::open(path).map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let capacity = if file_size == 0 {
        max_bytes
    } else {
        (file_size as usize).min(max_bytes)
    };
    let mut buf = Vec::with_capacity(capacity);
    file.take(max_bytes as u64)
        .read_to_end(&mut buf)
        .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
    let truncated = file_size > max_bytes as u64;
    let is_binary = buf[..buf.len().min(512)].contains(&0);
    let content = if is_binary {
        String::new()
    } else {
        String::from_utf8_lossy(&buf).into_owned()
    };
    Ok(FilePreview {
        content,
        truncated,
        bytes_read: buf.len(),
        is_binary,
    })
}

#[derive(Debug, Serialize, Clone)]
pub struct PdfPreview {
    pub image_path: String,
    pub page_count: u32,
}

/// Returns a cache path under `temp_dir/hyprfiles-{subdir}/{hash}.{ext}` and whether it is fresh.
/// Fresh means the cached file exists and is newer than `source`.
fn cached_path(
    source: &Path,
    subdir: &str,
    ext: &str,
    extra_key: &str,
) -> Result<(PathBuf, bool), AppError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    source.display().to_string().hash(&mut hasher);
    extra_key.hash(&mut hasher);
    let hash = hasher.finish();

    let dir = std::env::temp_dir().join(format!("hyprfiles-{subdir}"));
    fs::create_dir_all(&dir)
        .map_err(|e| AppError::io_with_path(e, dir.display().to_string()))?;
    let out = dir.join(format!("{hash:x}.{ext}"));

    let fresh = out.exists()
        && fs::metadata(source)
            .and_then(|src| fs::metadata(&out).map(|dst| (src, dst)))
            .and_then(|(src, dst)| Ok(dst.modified()? >= src.modified()?))
            .unwrap_or(false);

    Ok((out, fresh))
}

pub fn render_pdf_preview(path: &Path) -> Result<PdfPreview, AppError> {
    let path_str = path.display().to_string();
    let (output_png, fresh) = cached_path(path, "pdf", "png", "")?;
    let output_prefix = output_png.with_extension("");

    if fresh {
        let page_count = get_pdf_page_count(path);
        return Ok(PdfPreview {
            image_path: output_png.display().to_string(),
            page_count,
        });
    }

    // Render first page to PNG
    let result = std::process::Command::new("pdftoppm")
        .args([
            "-png",
            "-f",
            "1",
            "-l",
            "1",
            "-singlefile",
            "-r",
            "200",
            &path_str,
            &output_prefix.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output();

    match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::Desktop {
                message: "PDF preview requires poppler-utils (pacman -S poppler)".to_string(),
            });
        }
        Err(e) => {
            return Err(AppError::io_with_path(e, path_str));
        }
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Desktop {
                message: format!("Failed to render PDF: {stderr}"),
            });
        }
        Ok(_) => {}
    }

    if !output_png.exists() {
        return Err(AppError::Desktop {
            message: "PDF rendering produced no output".to_string(),
        });
    }

    let page_count = get_pdf_page_count(path);

    Ok(PdfPreview {
        image_path: output_png.display().to_string(),
        page_count,
    })
}

fn get_pdf_page_count(path: &Path) -> u32 {
    let output = std::process::Command::new("pdfinfo")
        .arg(path.display().to_string())
        .output()
        .ok();
    output
        .and_then(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout
                .lines()
                .find(|l| l.starts_with("Pages:"))
                .and_then(|l| l.split_whitespace().last()?.parse().ok())
        })
        .unwrap_or(0)
}

#[derive(Debug, Serialize, Clone)]
pub struct ImageThumbnail {
    pub image_path: String,
    pub width: u32,
    pub height: u32,
}

pub fn generate_thumbnail(path: &Path, max_dim: u32) -> Result<ImageThumbnail, AppError> {
    let path_str = path.display().to_string();
    let (output_png, fresh) = cached_path(path, "thumbs", "png", &max_dim.to_string())?;

    if fresh {
        // Dimensions encoded in cached filename would be ideal, but parsing the
        // PNG header via image_dimensions is a single small read — acceptable.
        let (width, height) =
            image::image_dimensions(&output_png).map_err(|e| AppError::Desktop {
                message: format!("Failed to read cached thumbnail: {e}"),
            })?;
        return Ok(ImageThumbnail {
            image_path: output_png.display().to_string(),
            width,
            height,
        });
    }

    let img = image::open(path)
        .map_err(|e| AppError::io_with_path(std::io::Error::other(e), path_str))?;

    let (orig_w, orig_h) = (img.width(), img.height());
    let scale = f64::from(max_dim) / f64::from(orig_w.max(orig_h));
    let (new_w, new_h) = if scale >= 1.0 {
        (orig_w, orig_h)
    } else {
        (
            (f64::from(orig_w) * scale).round() as u32,
            (f64::from(orig_h) * scale).round() as u32,
        )
    };

    let thumb = img.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle);

    thumb.save(&output_png).map_err(|e| AppError::Desktop {
        message: format!("Failed to save thumbnail: {e}"),
    })?;

    Ok(ImageThumbnail {
        image_path: output_png.display().to_string(),
        width: thumb.width(),
        height: thumb.height(),
    })
}

pub fn chmod_entry(path: &Path, mode: u32) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, perms)
        .map_err(|e| AppError::io_with_path(e, path.display().to_string()))
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

    #[test]
    fn read_file_preview_basic() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "hello\nworld\n").unwrap();
        let preview = read_file_preview(&file, 65536).unwrap();
        assert_eq!(preview.content, "hello\nworld\n");
        assert!(!preview.truncated);
        assert!(!preview.is_binary);
        assert_eq!(preview.bytes_read, 12);
    }

    #[test]
    fn read_file_preview_truncated() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("big.txt");
        let content = "x".repeat(1000);
        fs::write(&file, &content).unwrap();
        let preview = read_file_preview(&file, 500).unwrap();
        assert_eq!(preview.content.len(), 500);
        assert!(preview.truncated);
        assert!(!preview.is_binary);
    }

    #[test]
    fn read_file_preview_binary() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("binary.bin");
        fs::write(&file, &[0u8, 1, 2, 3, 0xFF, 0xFE]).unwrap();
        let preview = read_file_preview(&file, 65536).unwrap();
        assert!(preview.is_binary);
        assert!(preview.content.is_empty());
    }

    #[test]
    fn chmod_entry_works() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "hello").unwrap();
        chmod_entry(&file, 0o644).unwrap();
        let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o644);
        chmod_entry(&file, 0o755).unwrap();
        let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o755);
    }
}
