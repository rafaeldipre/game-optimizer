use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not running as administrator. This application requires elevation.")]
    NotElevated,

    #[error("Executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Windows API error: {0}")]
    WindowsApi(String),

    #[error("Service error for '{service}': {message}")]
    ServiceError { service: String, message: String },

    #[error("Process error for '{process}': {message}")]
    ProcessError { process: String, message: String },

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Action blocked by security guard: {0}")]
    SecurityGuard(String),
}

pub type AppResult<T> = Result<T, AppError>;
