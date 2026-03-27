use crate::error::AppError;
use crate::fs_ops::DriveEntry;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LsblkDevice {
    name: String,
    label: Option<String>,
    fstype: Option<String>,
    mountpoint: Option<String>,
    size: Option<String>,
    rm: Option<bool>,
    #[serde(rename = "type")]
    devtype: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

const SKIP_FS: &[&str] = &["swap", "crypto_LUKS"];

const REAL_FS: &[&str] = &[
    "ext2", "ext3", "ext4", "btrfs", "xfs", "zfs", "f2fs", "bcachefs", "ntfs", "ntfs3", "vfat",
    "exfat", "fuseblk", "nfs", "nfs4", "cifs", "smb3", "fuse.sshfs", "fuse.rclone",
    "fuse.mergerfs",
];

#[tauri::command]
pub fn list_drives() -> Vec<DriveEntry> {
    let Ok(output) = std::process::Command::new("lsblk")
        .args(["-Jpo", "NAME,LABEL,FSTYPE,MOUNTPOINT,SIZE,RM,TYPE"])
        .output()
    else {
        return Vec::new();
    };

    let Ok(parsed) = serde_json::from_slice::<LsblkOutput>(&output.stdout) else {
        return Vec::new();
    };

    // Find root device to skip its other partitions
    let root_device = find_root_device(&parsed.blockdevices);

    let mut drives = Vec::new();
    collect_drives(&parsed.blockdevices, &root_device, &mut drives);
    drives
}

fn find_root_device(devices: &[LsblkDevice]) -> String {
    for dev in devices {
        if let Some(children) = &dev.children {
            for child in children {
                if child.mountpoint.as_deref() == Some("/") {
                    return dev.name.clone();
                }
                // Check nested (e.g. LUKS -> btrfs mounted at /)
                if let Some(grandchildren) = &child.children {
                    for gc in grandchildren {
                        if gc.mountpoint.as_deref() == Some("/") {
                            return dev.name.clone();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

fn collect_drives(devices: &[LsblkDevice], root_device: &str, out: &mut Vec<DriveEntry>) {
    for dev in devices {
        // Skip the root disk entirely
        if !root_device.is_empty() && dev.name == root_device {
            continue;
        }

        if let Some(children) = &dev.children {
            for child in children {
                if let Some(entry) = partition_to_drive(child, dev) {
                    out.push(entry);
                }
                // Handle nested devices (LVM, LUKS containers on external drives)
                if let Some(grandchildren) = &child.children {
                    for gc in grandchildren {
                        if let Some(entry) = partition_to_drive(gc, dev) {
                            out.push(entry);
                        }
                    }
                }
            }
        }
    }
}

const SYSTEM_MOUNTS: &[&str] = &[
    "/", "/boot", "/efi", "/home", "/tmp", "/var", "/usr", "/opt", "/srv", "/root",
];

fn is_system_mount(mountpoint: &str) -> bool {
    SYSTEM_MOUNTS.iter().any(|&prefix| {
        mountpoint == prefix || (prefix != "/" && mountpoint.starts_with(&format!("{prefix}/")))
    })
}

fn partition_to_drive(part: &LsblkDevice, parent: &LsblkDevice) -> Option<DriveEntry> {
    let fstype = part.fstype.as_deref()?;

    if SKIP_FS.contains(&fstype) {
        return None;
    }

    // Skip system mounts (root, home, var, boot, etc.)
    if let Some(mp) = &part.mountpoint {
        if is_system_mount(mp) {
            return None;
        }
    }

    if !REAL_FS.contains(&fstype) {
        return None;
    }

    let mounted = part.mountpoint.is_some();
    let path = part.mountpoint.clone().unwrap_or_default();

    let name = part
        .label
        .clone()
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| {
            Path::new(&part.name)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| part.name.clone())
        });

    let removable = parent.rm.unwrap_or(false);

    Some(DriveEntry {
        name,
        path,
        device: part.name.clone(),
        fstype: fstype.to_string(),
        removable,
        mounted,
        size: part.size.clone().unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn mount_drive(device: String) -> Result<String, AppError> {
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("udisksctl")
            .args(["mount", "-b", &device, "--no-interaction"])
            .output()
    })
    .await
    .map_err(|e| AppError::Desktop {
        message: format!("Task join error: {e}"),
    })?
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::Desktop {
                message: "Mounting requires udisks2 (pacman -S udisks2)".to_string(),
            }
        } else {
            AppError::Desktop {
                message: format!("Failed to run udisksctl: {e}"),
            }
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Desktop {
            message: format!("Mount failed: {stderr}"),
        });
    }

    // Parse mount point from output: "Mounted /dev/sda1 at /run/media/jacob/Elements."
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mount_point = stdout
        .split(" at ")
        .nth(1)
        .map(|s| s.trim().trim_end_matches('.').to_string())
        .unwrap_or_default();

    Ok(mount_point)
}
