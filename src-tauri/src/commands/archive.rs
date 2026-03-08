use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

use super::file_ops::dir_size_and_count;

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
