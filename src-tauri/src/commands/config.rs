use crate::config::{load_config, AppConfig};
use crate::error::AppError;

#[tauri::command]
pub fn get_config() -> Result<AppConfig, AppError> {
    Ok(load_config())
}
