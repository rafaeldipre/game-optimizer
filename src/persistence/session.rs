use crate::core::errors::{AppError, AppResult};
use crate::core::model::SessionState;
use std::path::PathBuf;

const SESSION_FILE: &str = "session.json";
const SESSION_TMP: &str = "session.json.tmp";

pub fn load_session() -> Option<SessionState> {
    let path = PathBuf::from(SESSION_FILE);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_session(state: &SessionState) -> AppResult<()> {
    let content = serde_json::to_string_pretty(state).map_err(AppError::Json)?;
    // Atomic write: write to .tmp then rename
    std::fs::write(SESSION_TMP, &content).map_err(AppError::Io)?;
    std::fs::rename(SESSION_TMP, SESSION_FILE).map_err(AppError::Io)?;
    Ok(())
}

pub fn clear_session() -> AppResult<()> {
    let path = PathBuf::from(SESSION_FILE);
    if path.exists() {
        std::fs::remove_file(&path).map_err(AppError::Io)?;
    }
    Ok(())
}
