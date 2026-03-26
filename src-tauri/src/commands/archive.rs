use crate::error::AppError;
use crate::progress;
use std::path::PathBuf;
use tauri::AppHandle;

use super::file_ops::dir_size_and_count;

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
        progress::check_cancelled()?;
        let n = self.inner.write(buf)?;
        self.processed += n as u64;
        progress::emit(&self.app, self.processed, self.total);
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
        progress::check_cancelled()?;
        let n = self.inner.read(buf)?;
        self.processed += n as u64;
        progress::emit(&self.app, self.processed, self.total);
        Ok(n)
    }
}

impl<R: std::io::Read + std::io::Seek> std::io::Seek for ProgressReader<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

#[tauri::command]
pub async fn compress(paths: Vec<String>, dest: String, app: AppHandle) -> Result<(), AppError> {
    progress::reset();
    tokio::task::spawn_blocking(move || compress_sync(&paths, &dest, &app))
        .await
        .map_err(|e| AppError::Archive {
            message: format!("Task failed: {e}"),
        })?
}

#[tauri::command]
pub fn cancel_operation() {
    progress::cancel();
}

fn compress_sync(paths: &[String], dest: &str, app: &AppHandle) -> Result<(), AppError> {
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
        Err(AppError::Archive {
            message: format!("Unsupported archive format: {name}"),
        })
    };

    // Clean up partial file on cancel
    if result.is_err() && progress::is_cancelled() {
        let _ = std::fs::remove_file(dest);
        return Err(AppError::Cancelled);
    }

    result
}

fn compress_zip(paths: &[String], dest: &str, total: u64, app: &AppHandle) -> Result<(), AppError> {
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(dest)
        .map_err(|e| AppError::io_with_path(e, dest.to_string()))?;
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
                .map_err(|e| AppError::Archive { message: e.to_string() })?;
            let mut f = std::fs::File::open(&path)
                .map_err(|e| AppError::io_with_path(e, p.clone()))?;
            std::io::copy(&mut f, &mut zip)
                .map_err(|e| AppError::io_with_path(e, p.clone()))?;
        }
    }

    zip.finish().map_err(|e| AppError::Archive { message: e.to_string() })?;
    Ok(())
}

fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    root: &PathBuf,
    dir: &PathBuf,
    options: zip::write::SimpleFileOptions,
) -> Result<(), AppError> {
    let base = root.parent().unwrap_or(root);

    for entry in std::fs::read_dir(dir).map_err(|e| AppError::io_with_path(e, dir.display().to_string()))? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        let name = rel.to_string_lossy().to_string();

        if meta.is_dir() {
            zip.add_directory(&name, options)
                .map_err(|e| AppError::Archive { message: e.to_string() })?;
            add_dir_to_zip(zip, root, &path, options)?;
        } else {
            zip.start_file(&name, options)
                .map_err(|e| AppError::Archive { message: e.to_string() })?;
            let mut f = std::fs::File::open(&path)
                .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
            std::io::copy(&mut f, zip)
                .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
        }
    }
    Ok(())
}

fn compress_tar(paths: &[String], dest: &str, compression: &str, total: u64, app: &AppHandle) -> Result<(), AppError> {
    let file = std::fs::File::create(dest)
        .map_err(|e| AppError::io_with_path(e, dest.to_string()))?;
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
                .map_err(|e| AppError::Archive { message: e.to_string() })?
                .auto_finish();
            write_tar(enc, paths)
        }
        "bz2" => {
            let enc = bzip2::write::BzEncoder::new(pw, bzip2::Compression::default());
            write_tar(enc, paths)
        }
        _ => Err(AppError::Archive {
            message: format!("Unsupported compression: {compression}"),
        }),
    }
}

fn write_tar<W: std::io::Write>(writer: W, paths: &[String]) -> Result<(), AppError> {
    let mut tar = tar::Builder::new(writer);
    tar.follow_symlinks(false);
    for p in paths {
        let path = PathBuf::from(p);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            add_dir_to_tar(&mut tar, &path, name)?;
        } else {
            let mut f = std::fs::File::open(&path)
                .map_err(|e| AppError::io_with_path(e, p.clone()))?;
            tar.append_file(name, &mut f)
                .map_err(|e| AppError::io_with_path(e, p.clone()))?;
        }
    }
    tar.finish().map_err(|e| AppError::Archive { message: e.to_string() })?;
    Ok(())
}

fn add_dir_to_tar<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    dir: &PathBuf,
    prefix: &str,
) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir).map_err(|e| AppError::io_with_path(e, dir.display().to_string()))? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let path = entry.path();
        let entry_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let archive_name = format!("{prefix}/{entry_name}");

        if meta.is_dir() {
            tar.append_dir(&archive_name, &path)
                .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
            add_dir_to_tar(tar, &path, &archive_name)?;
        } else {
            let mut f = std::fs::File::open(&path)
                .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
            tar.append_file(&archive_name, &mut f)
                .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn extract(archive: String, dest: String, app: AppHandle) -> Result<(), AppError> {
    progress::reset();
    tokio::task::spawn_blocking(move || extract_sync(&archive, &dest, &app))
        .await
        .map_err(|e| AppError::Archive {
            message: format!("Task failed: {e}"),
        })?
}

fn extract_sync(archive: &str, dest: &str, app: &AppHandle) -> Result<(), AppError> {
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
        .map_err(|e| AppError::io_with_path(e, dest.to_string()))?;

    let file = std::fs::File::open(archive)
        .map_err(|e| AppError::io_with_path(e, archive.to_string()))?;
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
            .map_err(|e| AppError::Archive { message: e.to_string() })?;
        unpack_tar(dec, &dest_path)
    } else if name.ends_with(".tar.bz2") {
        let dec = bzip2::read::BzDecoder::new(pr);
        unpack_tar(dec, &dest_path)
    } else if name.ends_with(".tar") {
        unpack_tar(pr, &dest_path)
    } else {
        Err(AppError::Archive {
            message: format!("Unsupported archive format: {name}"),
        })
    };

    if result.is_err() && progress::is_cancelled() {
        return Err(AppError::Cancelled);
    }

    result
}

fn extract_zip<R: std::io::Read + std::io::Seek>(reader: R, dest: &PathBuf) -> Result<(), AppError> {
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| AppError::Archive { message: format!("Invalid zip: {e}") })?;
    zip.extract(dest)
        .map_err(|e| AppError::Archive { message: format!("Extract error: {e}") })?;
    Ok(())
}

fn unpack_tar<R: std::io::Read>(reader: R, dest: &PathBuf) -> Result<(), AppError> {
    let mut tar = tar::Archive::new(reader);
    tar.unpack(dest)
        .map_err(|e| AppError::io_with_path(e, dest.display().to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a directory with some test files
    fn create_test_dir(dir: &std::path::Path) {
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("hello.txt"), "Hello, world!").unwrap();
        fs::write(dir.join("sub/nested.txt"), "Nested content").unwrap();
    }

    /// Helper: verify extracted content matches what we created
    fn assert_test_dir_contents(dir: &std::path::Path, prefix: &str) {
        let hello = dir.join(prefix).join("hello.txt");
        assert!(hello.exists(), "hello.txt should exist");
        assert_eq!(fs::read_to_string(&hello).unwrap(), "Hello, world!");

        let nested = dir.join(prefix).join("sub/nested.txt");
        assert!(nested.exists(), "sub/nested.txt should exist");
        assert_eq!(fs::read_to_string(&nested).unwrap(), "Nested content");
    }

    #[test]
    fn zip_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        create_test_dir(&src);

        let archive = tmp.path().join("test.zip");

        // Test add_dir_to_zip directly (compress_zip needs AppHandle for ProgressWriter)
        {
            use zip::write::SimpleFileOptions;
            let file = fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            add_dir_to_zip(&mut zip, &src, &src, options).unwrap();
            zip.finish().unwrap();
        }

        // Extract
        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            extract_zip(file, &extract_dir).unwrap();
        }

        assert_test_dir_contents(&extract_dir, "source");
    }

    #[test]
    fn tar_gz_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        create_test_dir(&src);

        let archive = tmp.path().join("test.tar.gz");
        let paths = vec![src.to_string_lossy().to_string()];

        // Compress
        {
            let file = fs::File::create(&archive).unwrap();
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            write_tar(enc, &paths).unwrap();
        }

        // Extract
        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            let dec = flate2::read::GzDecoder::new(file);
            unpack_tar(dec, &extract_dir).unwrap();
        }

        assert_test_dir_contents(&extract_dir, "source");
    }

    #[test]
    fn tar_xz_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        create_test_dir(&src);

        let archive = tmp.path().join("test.tar.xz");
        let paths = vec![src.to_string_lossy().to_string()];

        {
            let file = fs::File::create(&archive).unwrap();
            let enc = xz2::write::XzEncoder::new(file, 6);
            write_tar(enc, &paths).unwrap();
        }

        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            let dec = xz2::read::XzDecoder::new(file);
            unpack_tar(dec, &extract_dir).unwrap();
        }

        assert_test_dir_contents(&extract_dir, "source");
    }

    #[test]
    fn tar_zst_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        create_test_dir(&src);

        let archive = tmp.path().join("test.tar.zst");
        let paths = vec![src.to_string_lossy().to_string()];

        {
            let file = fs::File::create(&archive).unwrap();
            let enc = zstd::Encoder::new(file, 3).unwrap().auto_finish();
            write_tar(enc, &paths).unwrap();
        }

        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            let dec = zstd::Decoder::new(file).unwrap();
            unpack_tar(dec, &extract_dir).unwrap();
        }

        assert_test_dir_contents(&extract_dir, "source");
    }

    #[test]
    fn tar_bz2_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        create_test_dir(&src);

        let archive = tmp.path().join("test.tar.bz2");
        let paths = vec![src.to_string_lossy().to_string()];

        {
            let file = fs::File::create(&archive).unwrap();
            let enc = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
            write_tar(enc, &paths).unwrap();
        }

        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            let dec = bzip2::read::BzDecoder::new(file);
            unpack_tar(dec, &extract_dir).unwrap();
        }

        assert_test_dir_contents(&extract_dir, "source");
    }

    #[test]
    fn compress_empty_directory() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("empty");
        fs::create_dir_all(&src).unwrap();

        let archive = tmp.path().join("empty.tar.gz");
        let paths = vec![src.to_string_lossy().to_string()];

        {
            let file = fs::File::create(&archive).unwrap();
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            write_tar(enc, &paths).unwrap();
        }

        // The archive should be created successfully even with an empty dir.
        // Note: write_tar uses add_dir_to_tar which only adds contents,
        // so the empty dir itself won't have entries but the archive is valid.
        assert!(archive.exists());
        assert!(fs::metadata(&archive).unwrap().len() > 0);

        // Extraction should succeed without errors
        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            let dec = flate2::read::GzDecoder::new(file);
            unpack_tar(dec, &extract_dir).unwrap();
        }
    }

    #[test]
    fn zip_skips_symlinks() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("real.txt"), "real file").unwrap();

        // Create a symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(src.join("real.txt"), src.join("link.txt")).unwrap();

        let archive = tmp.path().join("test.zip");
        {
            use zip::write::SimpleFileOptions;
            let file = fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            add_dir_to_zip(&mut zip, &src, &src, options).unwrap();
            zip.finish().unwrap();
        }

        // Extract and verify symlink was skipped
        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).unwrap();
        {
            let file = fs::File::open(&archive).unwrap();
            extract_zip(file, &extract_dir).unwrap();
        }

        assert!(extract_dir.join("source/real.txt").exists());
        #[cfg(unix)]
        assert!(!extract_dir.join("source/link.txt").exists());
    }

    #[test]
    fn extract_to_existing_directory() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("source");
        create_test_dir(&src);

        // Create archive
        let archive = tmp.path().join("test.tar.gz");
        let paths = vec![src.to_string_lossy().to_string()];
        {
            let file = fs::File::create(&archive).unwrap();
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            write_tar(enc, &paths).unwrap();
        }

        // Extract to a directory that already exists with some content
        let extract_dir = tmp.path().join("existing");
        fs::create_dir_all(&extract_dir).unwrap();
        fs::write(extract_dir.join("preexisting.txt"), "already here").unwrap();

        {
            let file = fs::File::open(&archive).unwrap();
            let dec = flate2::read::GzDecoder::new(file);
            unpack_tar(dec, &extract_dir).unwrap();
        }

        // Both old and new content should exist
        assert!(extract_dir.join("preexisting.txt").exists());
        assert_test_dir_contents(&extract_dir, "source");
    }
}
