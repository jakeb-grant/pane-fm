use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomAction {
    pub name: String,
    pub command: String,
    #[serde(default = "default_context")]
    pub context: String,
    #[serde(default)]
    pub mime: Option<String>,
    #[serde(default)]
    pub refresh: bool,
}

fn default_context() -> String {
    "any".to_string()
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub keybinds: HashMap<String, serde_json::Value>,
    pub chords: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub actions: Vec<CustomAction>,
    #[serde(default)]
    pub preview: PreviewConfig,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub show_hidden: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_ascending: Option<bool>,
    pub theme: Option<String>,
    pub light_icons: Option<bool>,
    pub editor: Option<String>,
    pub terminal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    /// JPEG quality for thumbnails (50–90)
    #[serde(default = "default_image_quality")]
    pub image_quality: u8,
    /// Reject images with either dimension exceeding this
    #[serde(default = "default_max_dimension")]
    pub max_dimension: u32,
    /// Reject images whose decoded RGBA would exceed this many MB
    #[serde(default = "default_max_alloc_mb")]
    pub max_alloc_mb: u64,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            image_quality: default_image_quality(),
            max_dimension: default_max_dimension(),
            max_alloc_mb: default_max_alloc_mb(),
        }
    }
}

fn default_image_quality() -> u8 {
    75
}
fn default_max_dimension() -> u32 {
    10_000
}
fn default_max_alloc_mb() -> u64 {
    512
}

pub fn resolve_theme_path(theme: &str) -> Option<std::path::PathBuf> {
    let path = if theme.starts_with('/') {
        std::path::PathBuf::from(theme)
    } else if let Some(rest) = theme.strip_prefix("~/") {
        dirs::home_dir()?.join(rest)
    } else {
        // Bundled theme name — look in ~/.config/pane-fm/themes/{theme}.css
        dirs::config_dir()?
            .join("pane-fm")
            .join("themes")
            .join(format!("{theme}.css"))
    };
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub fn install_default_config() {
    let Some(config_dir) = dirs::config_dir() else {
        return;
    };
    let dir = config_dir.join("pane-fm");
    let path = dir.join("config.toml");
    if path.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(path, include_str!("default-config.toml"));
}

pub fn load_config() -> AppConfig {
    let Some(config_dir) = dirs::config_dir() else {
        return AppConfig::default();
    };
    let path = config_dir.join("pane-fm").join("config.toml");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    match toml::from_str::<RawConfig>(&contents) {
        Ok(raw) => {
            let keybinds = raw
                .keybinds
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect();
            AppConfig {
                general: raw.general,
                keybinds,
                chords: raw.chords,
                actions: raw.actions,
                preview: raw.preview,
                warning: None,
            }
        }
        Err(e) => AppConfig {
            warning: Some(format!("Config parse error: {e}")),
            ..AppConfig::default()
        },
    }
}

fn toml_to_json(v: toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_to_json).collect())
        }
        _ => serde_json::Value::String(v.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    general: GeneralConfig,
    #[serde(default)]
    keybinds: HashMap<String, toml::Value>,
    #[serde(default)]
    chords: HashMap<String, Vec<String>>,
    #[serde(default)]
    actions: Vec<CustomAction>,
    #[serde(default)]
    preview: PreviewConfig,
}
