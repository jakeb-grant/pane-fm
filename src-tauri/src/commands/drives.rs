use crate::fs_ops::DriveEntry;
use std::io::BufRead;
use std::path::Path;

#[tauri::command]
pub fn list_drives() -> Vec<DriveEntry> {
    let mut drives = Vec::new();

    let Ok(file) = std::fs::File::open("/proc/mounts") else {
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
        .map_while(Result::ok)
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

fn is_removable(device: &str) -> bool {
    let dev_name = Path::new(device)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Strip partition number to get the base device (sda1 -> sda)
    let base = dev_name.trim_end_matches(|c: char| c.is_ascii_digit());

    let removable_path = format!("/sys/block/{base}/removable");
    std::fs::read_to_string(&removable_path)
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}
