use crate::core::errors::{AppError, AppResult};
use crate::core::model::AppConfig;

const CONFIG_FILE: &str = "config.json";

pub fn load_config() -> AppConfig {
    let path = std::path::PathBuf::from(CONFIG_FILE);
    if !path.exists() {
        return AppConfig::default();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return AppConfig::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> AppResult<()> {
    let content = serde_json::to_string_pretty(config).map_err(AppError::Json)?;
    std::fs::write(CONFIG_FILE, content).map_err(AppError::Io)?;
    Ok(())
}
