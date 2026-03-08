use chrono::{DateTime, Local};
use serde::Serialize;
use std::fs;
use std::io::BufRead;
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

    files.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(files)
}

pub fn guess_mime_pub(path: &Path) -> String {
    guess_mime(path)
}

fn guess_mime(path: &Path) -> String {
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

#[derive(Debug, Serialize, Clone)]
pub struct DriveEntry {
    pub name: String,
    pub path: String,
    pub fstype: String,
    pub removable: bool,
}

pub fn list_drives() -> Vec<DriveEntry> {
    let mut drives = Vec::new();

    let Ok(file) = fs::File::open("/proc/mounts") else {
        return drives;
    };

    // Only show mounts under user-facing paths
    let user_mount_prefixes = ["/media/", "/mnt/", "/run/media/"];

    // Real on-disk/network filesystem types worth showing
    let real_fs = [
        "ext2", "ext3", "ext4", "btrfs", "xfs", "zfs", "f2fs", "bcachefs",
        "ntfs", "ntfs3", "vfat", "exfat", "fuseblk",
        "nfs", "nfs4", "cifs", "smb3",
        "fuse.sshfs", "fuse.rclone", "fuse.mergerfs",
    ];

    // First pass: find the root device so we can skip its subvolumes
    let mut root_device = String::new();
    let lines: Vec<String> = std::io::BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .collect();

    for line in &lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == "/" {
            root_device = parts[0].to_string();
            break;
        }
    }

    for line in &lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let device = parts[0];
        let mount = parts[1];
        let fstype = parts[2];

        // Skip root and boot partitions
        if mount == "/" || mount.starts_with("/boot") || mount.starts_with("/efi") {
            continue;
        }

        // Skip subvolumes/partitions on the same device as root
        if !root_device.is_empty() && device == root_device {
            continue;
        }

        // Include if it's under a user mount path, or if it's a real FS on a block device
        let is_user_mount = user_mount_prefixes.iter().any(|p| mount.starts_with(p));
        let is_real_fs = real_fs.contains(&fstype) && device.starts_with("/dev/");

        if !is_user_mount && !is_real_fs {
            continue;
        }

        let name = Path::new(mount)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| mount.to_string());

        let removable = is_removable(device);

        drives.push(DriveEntry {
            name,
            path: mount.to_string(),
            fstype: fstype.to_string(),
            removable,
        });
    }

    drives
}

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
        fs::read_dir(&trash_files).map_err(|e| format!("Failed to read trash: {e}"))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let metadata = entry.metadata().map_err(|e| format!("Failed to read metadata: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path_buf = entry.path();

        // Try to get the original deletion date from .trashinfo
        let info_file = trash_info.join(format!("{name}.trashinfo"));
        let modified = if let Ok(info_content) = fs::read_to_string(&info_file) {
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
            guess_mime(&path_buf)
        };

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let permissions = 0u32;

        let children_count = if metadata.is_dir() {
            fs::read_dir(&path_buf).ok().map(|d| d.count() as u64)
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

    files.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(files)
}

fn is_removable(device: &str) -> bool {
    let dev_name = Path::new(device)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Strip partition number to get the base device (sda1 -> sda)
    let base = dev_name.trim_end_matches(|c: char| c.is_ascii_digit());

    let removable_path = format!("/sys/block/{base}/removable");
    fs::read_to_string(&removable_path)
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}
