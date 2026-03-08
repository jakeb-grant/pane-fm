use crate::fs_ops::{self, DriveEntry, FileEntry};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

static CANCEL_OPERATION: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
struct ProgressPayload {
    processed: u64,
    total: u64,
}

/// A Write wrapper that tracks bytes written, emits progress events,
/// and checks for cancellation on every write call.
struct ProgressWriter<W: std::io::Write> {
    inner: W,
    processed: u64,
    total: u64,
    app: AppHandle,
}

impl<W: std::io::Write> std::io::Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if CANCEL_OPERATION.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Cancelled"));
        }
        let n = self.inner.write(buf)?;
        self.processed += n as u64;
        let _ = self.app.emit("compress-progress", ProgressPayload {
            processed: self.processed,
            total: self.total,
        });
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<W: std::io::Write + std::io::Seek> std::io::Seek for ProgressWriter<W> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// A Read wrapper that tracks bytes read, emits progress events,
/// and checks for cancellation on every read call.
struct ProgressReader<R: std::io::Read> {
    inner: R,
    processed: u64,
    total: u64,
    app: AppHandle,
}

impl<R: std::io::Read> std::io::Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if CANCEL_OPERATION.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Cancelled"));
        }
        let n = self.inner.read(buf)?;
        self.processed += n as u64;
        let _ = self.app.emit("compress-progress", ProgressPayload {
            processed: self.processed,
            total: self.total,
        });
        Ok(n)
    }
}

impl<R: std::io::Read + std::io::Seek> std::io::Seek for ProgressReader<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

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
pub fn list_drives() -> Vec<DriveEntry> {
    fs_ops::list_drives()
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    PathBuf::from(path).is_dir()
}

#[tauri::command]
pub fn list_trash() -> Result<Vec<FileEntry>, String> {
    fs_ops::list_trash()
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
    let decoded = urlencoding(original_path);
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
pub fn open_default(path: String, app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| format!("Failed to open: {e}"))
}

#[derive(Debug, Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub desktop_id: String,
    pub icon: String,
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

#[tauri::command]
pub async fn compress(paths: Vec<String>, dest: String, app: AppHandle) -> Result<(), String> {
    CANCEL_OPERATION.store(false, Ordering::Relaxed);
    tokio::task::spawn_blocking(move || compress_sync(&paths, &dest, &app))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub fn cancel_operation() {
    CANCEL_OPERATION.store(true, Ordering::Relaxed);
}

fn compress_sync(paths: &[String], dest: &str, app: &AppHandle) -> Result<(), String> {
    let dest_path = PathBuf::from(dest);
    let name = dest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // Pre-scan total bytes for progress
    let total: u64 = paths.iter().map(|p| {
        let path = PathBuf::from(p);
        if path.is_dir() {
            dir_size_and_count(&path).map(|(s, _)| s).unwrap_or(0)
        } else {
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        }
    }).sum();

    let result = if name.ends_with(".zip") {
        compress_zip(paths, dest, total, app)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        compress_tar(paths, dest, "gz", total, app)
    } else if name.ends_with(".tar.xz") {
        compress_tar(paths, dest, "xz", total, app)
    } else if name.ends_with(".tar.zst") {
        compress_tar(paths, dest, "zst", total, app)
    } else if name.ends_with(".tar.bz2") {
        compress_tar(paths, dest, "bz2", total, app)
    } else {
        Err(format!("Unsupported archive format: {name}"))
    };

    // Clean up partial file on cancel
    if result.is_err() && CANCEL_OPERATION.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(dest);
        return Err("Cancelled".to_string());
    }

    result
}

fn compress_zip(paths: &[String], dest: &str, total: u64, app: &AppHandle) -> Result<(), String> {
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(dest)
        .map_err(|e| format!("Failed to create archive: {e}"))?;
    let pw = ProgressWriter { inner: file, processed: 0, total, app: app.clone() };
    let mut zip = zip::ZipWriter::new(pw);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for p in paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            add_dir_to_zip(&mut zip, &path, &path, options)?;
        } else {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            zip.start_file(name, options)
                .map_err(|e| format!("Zip error: {e}"))?;
            let mut f = std::fs::File::open(&path)
                .map_err(|e| format!("Failed to read {p}: {e}"))?;
            std::io::copy(&mut f, &mut zip)
                .map_err(|e| format!("Zip write error: {e}"))?;
        }
    }

    zip.finish().map_err(|e| format!("Zip finish error: {e}"))?;
    Ok(())
}

fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    root: &PathBuf,
    dir: &PathBuf,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    let base = root.parent().unwrap_or(root);

    for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {e}"))? {
        let entry = entry.map_err(|e| format!("{e}"))?;
        let meta = entry.metadata().map_err(|e| format!("{e}"))?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        let name = rel.to_string_lossy().to_string();

        if meta.is_dir() {
            zip.add_directory(&name, options)
                .map_err(|e| format!("Zip error: {e}"))?;
            add_dir_to_zip(zip, root, &path, options)?;
        } else {
            zip.start_file(&name, options)
                .map_err(|e| format!("Zip error: {e}"))?;
            let mut f = std::fs::File::open(&path)
                .map_err(|e| format!("Failed to read file: {e}"))?;
            std::io::copy(&mut f, zip)
                .map_err(|e| format!("Zip write error: {e}"))?;
        }
    }
    Ok(())
}

fn compress_tar(paths: &[String], dest: &str, compression: &str, total: u64, app: &AppHandle) -> Result<(), String> {
    let file = std::fs::File::create(dest)
        .map_err(|e| format!("Failed to create archive: {e}"))?;
    let pw = ProgressWriter { inner: file, processed: 0, total, app: app.clone() };

    match compression {
        "gz" => {
            let enc = flate2::write::GzEncoder::new(pw, flate2::Compression::default());
            write_tar(enc, paths)
        }
        "xz" => {
            let enc = xz2::write::XzEncoder::new(pw, 6);
            write_tar(enc, paths)
        }
        "zst" => {
            let enc = zstd::Encoder::new(pw, 3)
                .map_err(|e| format!("Zstd error: {e}"))?
                .auto_finish();
            write_tar(enc, paths)
        }
        "bz2" => {
            let enc = bzip2::write::BzEncoder::new(pw, bzip2::Compression::default());
            write_tar(enc, paths)
        }
        _ => Err(format!("Unsupported compression: {compression}")),
    }
}

fn write_tar<W: std::io::Write>(writer: W, paths: &[String]) -> Result<(), String> {
    let mut tar = tar::Builder::new(writer);
    tar.follow_symlinks(false);
    for p in paths {
        let path = PathBuf::from(p);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            add_dir_to_tar(&mut tar, &path, name)?;
        } else {
            let mut f = std::fs::File::open(&path)
                .map_err(|e| format!("Failed to read {p}: {e}"))?;
            tar.append_file(name, &mut f)
                .map_err(|e| format!("Tar error: {e}"))?;
        }
    }
    tar.finish().map_err(|e| format!("Tar finish error: {e}"))?;
    Ok(())
}

fn add_dir_to_tar<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    dir: &PathBuf,
    prefix: &str,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {e}"))? {
        let entry = entry.map_err(|e| format!("{e}"))?;
        let meta = entry.metadata().map_err(|e| format!("{e}"))?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        let entry_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let archive_name = format!("{prefix}/{entry_name}");

        if meta.is_dir() {
            tar.append_dir(&archive_name, &path)
                .map_err(|e| format!("Tar error: {e}"))?;
            add_dir_to_tar(tar, &path, &archive_name)?;
        } else {
            let mut f = std::fs::File::open(&path)
                .map_err(|e| format!("Failed to read file: {e}"))?;
            tar.append_file(&archive_name, &mut f)
                .map_err(|e| format!("Tar error: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn extract(archive: String, dest: String, app: AppHandle) -> Result<(), String> {
    CANCEL_OPERATION.store(false, Ordering::Relaxed);
    tokio::task::spawn_blocking(move || extract_sync(&archive, &dest, &app))
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

fn extract_sync(archive: &str, dest: &str, app: &AppHandle) -> Result<(), String> {
    let archive_path = PathBuf::from(archive);
    let name = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let total = std::fs::metadata(archive)
        .map(|m| m.len())
        .unwrap_or(0);

    let dest_path = PathBuf::from(dest);
    std::fs::create_dir_all(&dest_path)
        .map_err(|e| format!("Failed to create destination: {e}"))?;

    let file = std::fs::File::open(archive)
        .map_err(|e| format!("Failed to open archive: {e}"))?;
    let pr = ProgressReader { inner: file, processed: 0, total, app: app.clone() };

    let result = if name.ends_with(".zip") {
        extract_zip(pr, &dest_path)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let dec = flate2::read::GzDecoder::new(pr);
        unpack_tar(dec, &dest_path)
    } else if name.ends_with(".tar.xz") {
        let dec = xz2::read::XzDecoder::new(pr);
        unpack_tar(dec, &dest_path)
    } else if name.ends_with(".tar.zst") {
        let dec = zstd::Decoder::new(pr)
            .map_err(|e| format!("Zstd error: {e}"))?;
        unpack_tar(dec, &dest_path)
    } else if name.ends_with(".tar.bz2") {
        let dec = bzip2::read::BzDecoder::new(pr);
        unpack_tar(dec, &dest_path)
    } else if name.ends_with(".tar") {
        unpack_tar(pr, &dest_path)
    } else {
        Err(format!("Unsupported archive format: {name}"))
    };

    if result.is_err() && CANCEL_OPERATION.load(Ordering::Relaxed) {
        return Err("Cancelled".to_string());
    }

    result
}

fn extract_zip<R: std::io::Read + std::io::Seek>(reader: R, dest: &PathBuf) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Invalid zip: {e}"))?;
    zip.extract(dest)
        .map_err(|e| format!("Extract error: {e}"))?;
    Ok(())
}

fn unpack_tar<R: std::io::Read>(reader: R, dest: &PathBuf) -> Result<(), String> {
    let mut tar = tar::Archive::new(reader);
    tar.unpack(dest)
        .map_err(|e| format!("Tar extract error: {e}"))?;
    Ok(())
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
        fs_ops::guess_mime_pub(&path)
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

fn dir_size_and_count(path: &PathBuf) -> Result<(u64, u64), std::io::Error> {
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

fn urlencoding(s: &str) -> String {
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
