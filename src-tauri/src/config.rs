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
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub show_hidden: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_ascending: Option<bool>,
    pub theme: Option<String>,
    pub terminal: Option<String>,
}

pub fn resolve_theme_path(theme: &str) -> Option<std::path::PathBuf> {
    let path = if theme.starts_with('/') {
        std::path::PathBuf::from(theme)
    } else if let Some(rest) = theme.strip_prefix("~/") {
        dirs::home_dir()?.join(rest)
    } else {
        // Bundled theme name — look in ~/.config/hyprfiles/themes/{theme}.css
        dirs::config_dir()?
            .join("hyprfiles")
            .join("themes")
            .join(format!("{theme}.css"))
    };
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub fn load_config() -> AppConfig {
    let Some(config_dir) = dirs::config_dir() else {
        return AppConfig::default();
    };
    let path = config_dir.join("hyprfiles").join("config.toml");
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
}
