use crate::error::AppError;
use crate::fs_ops::{self, FileEntry};
use chrono::{DateTime, Local};
use std::path::PathBuf;

#[tauri::command]
pub fn list_trash() -> Result<Vec<FileEntry>, AppError> {
    let trash_files = dirs::data_dir()
        .ok_or(AppError::Trash {
            message: "Could not find data directory".to_string(),
        })?
        .join("Trash/files");

    let trash_info = dirs::data_dir()
        .ok_or(AppError::Trash {
            message: "Could not find data directory".to_string(),
        })?
        .join("Trash/info");

    if !trash_files.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&trash_files)
        .map_err(|e| AppError::io_with_path(e, trash_files.display().to_string()))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path_buf = entry.path();

        // Try to get the original deletion date from .trashinfo
        let info_file = trash_info.join(format!("{name}.trashinfo"));
        let modified = if let Ok(info_content) = std::fs::read_to_string(&info_file) {
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
            fs_ops::guess_mime(&path_buf)
        };

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let permissions = 0u32;

        let children_count = if metadata.is_dir() {
            std::fs::read_dir(&path_buf).ok().map(|d| d.count() as u64)
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

    Ok(files)
}

#[tauri::command]
pub fn restore_trash(name: String) -> Result<(), AppError> {
    let data_dir = dirs::data_dir().ok_or(AppError::Trash {
        message: "Could not find data directory".to_string(),
    })?;
    let trash_file = data_dir.join("Trash/files").join(&name);
    let info_file = data_dir.join("Trash/info").join(format!("{name}.trashinfo"));

    // Read original path from .trashinfo
    let info = std::fs::read_to_string(&info_file)
        .map_err(|e| AppError::io_with_path(e, info_file.display().to_string()))?;

    let original_path = info
        .lines()
        .find(|l| l.starts_with("Path="))
        .map(|l| l.trim_start_matches("Path="))
        .ok_or(AppError::Trash {
            message: "Could not find original path in trash info".to_string(),
        })?;

    // URL-decode the path
    let decoded = percent_decode(original_path);
    let dest = PathBuf::from(&decoded);

    if dest.exists() {
        return Err(AppError::Trash {
            message: format!("Destination already exists: {decoded}"),
        });
    }

    std::fs::rename(&trash_file, &dest)
        .map_err(|e| AppError::io_with_path(e, trash_file.display().to_string()))?;

    // Remove the .trashinfo file
    let _ = std::fs::remove_file(&info_file);

    Ok(())
}

#[tauri::command]
pub fn empty_trash() -> Result<(), AppError> {
    let data_dir = dirs::data_dir().ok_or(AppError::Trash {
        message: "Could not find data directory".to_string(),
    })?;
    let trash_files = data_dir.join("Trash/files");
    let trash_info = data_dir.join("Trash/info");

    if trash_files.exists() {
        for entry in std::fs::read_dir(&trash_files)
            .map_err(|e| AppError::io_with_path(e, trash_files.display().to_string()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)
                    .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
            } else {
                std::fs::remove_file(&path)
                    .map_err(|e| AppError::io_with_path(e, path.display().to_string()))?;
            }
        }
    }

    if trash_info.exists() {
        for entry in std::fs::read_dir(&trash_info)
            .map_err(|e| AppError::io_with_path(e, trash_info.display().to_string()))?
        {
            let entry = entry?;
            let _ = std::fs::remove_file(entry.path());
        }
    }

    Ok(())
}

pub(crate) fn percent_decode(s: &str) -> String {
    let mut raw: Vec<u8> = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &s[i + 1..i + 3],
                16,
            ) {
                raw.push(byte);
                i += 3;
                continue;
            }
        }
        raw.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(raw).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_plain_string() {
        assert_eq!(percent_decode("hello"), "hello");
    }

    #[test]
    fn percent_decode_spaces() {
        assert_eq!(percent_decode("/home/user/my%20file.txt"), "/home/user/my file.txt");
    }

    #[test]
    fn percent_decode_special_characters() {
        // %23 = #, %26 = &, %3D = =
        assert_eq!(percent_decode("file%23name"), "file#name");
        assert_eq!(percent_decode("a%26b%3Dc"), "a&b=c");
    }

    #[test]
    fn percent_decode_unicode_path() {
        // %C3%A9 = é (UTF-8 bytes 0xC3 0xA9)
        assert_eq!(percent_decode("/home/user/caf%C3%A9"), "/home/user/café");
    }

    #[test]
    fn percent_decode_multibyte_unicode() {
        // %E4%B8%AD = 中 (CJK character, 3-byte UTF-8)
        assert_eq!(percent_decode("%E4%B8%AD"), "中");
        // Mixed: ASCII + multi-byte
        assert_eq!(percent_decode("a%C3%BC%C3%9Fb"), "aüßb");
    }

    #[test]
    fn percent_decode_trailing_percent() {
        // % at end without two hex chars should pass through
        assert_eq!(percent_decode("abc%"), "abc%");
        assert_eq!(percent_decode("abc%2"), "abc%2");
    }

    #[test]
    fn percent_decode_invalid_hex() {
        // %ZZ is not valid hex, should pass through
        assert_eq!(percent_decode("abc%ZZdef"), "abc%ZZdef");
    }
}
