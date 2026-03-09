use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum AppError {
    Io {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    NotFound {
        path: String,
    },
    PermissionDenied {
        path: String,
    },
    Cancelled,
    Archive {
        message: String,
    },
    Desktop {
        message: String,
    },
    AlreadyExists {
        path: String,
    },
    Trash {
        message: String,
    },
    #[allow(dead_code)]
    Config {
        message: String,
    },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io { message, path } => {
                if let Some(p) = path {
                    write!(f, "{message} ({p})")
                } else {
                    write!(f, "{message}")
                }
            }
            AppError::NotFound { path } => write!(f, "Not found: {path}"),
            AppError::PermissionDenied { path } => write!(f, "Permission denied: {path}"),
            AppError::AlreadyExists { path } => write!(f, "Already exists: {path}"),
            AppError::Cancelled => write!(f, "Operation cancelled"),
            AppError::Archive { message } => write!(f, "Archive error: {message}"),
            AppError::Desktop { message } => write!(f, "Desktop error: {message}"),
            AppError::Trash { message } => write!(f, "Trash error: {message}"),
            AppError::Config { message } => write!(f, "Config error: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound {
                path: String::new(),
            },
            std::io::ErrorKind::PermissionDenied => AppError::PermissionDenied {
                path: String::new(),
            },
            std::io::ErrorKind::AlreadyExists => AppError::AlreadyExists {
                path: String::new(),
            },
            std::io::ErrorKind::Interrupted => AppError::Cancelled,
            _ => AppError::Io {
                message: e.to_string(),
                path: None,
            },
        }
    }
}

impl AppError {
    /// Create from an io::Error with path context, mapping ErrorKind to specific variants.
    pub fn io_with_path(e: std::io::Error, path: impl Into<String>) -> Self {
        let path = path.into();
        match e.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound { path },
            std::io::ErrorKind::PermissionDenied => AppError::PermissionDenied { path },
            std::io::ErrorKind::AlreadyExists => AppError::AlreadyExists { path },
            std::io::ErrorKind::Interrupted => AppError::Cancelled,
            _ => AppError::Io {
                message: e.to_string(),
                path: Some(path),
            },
        }
    }
}

