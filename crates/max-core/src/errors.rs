use thiserror::Error;

#[derive(Error, Debug)]
pub enum MaxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Model Provider error: {0}")]
    Provider(String),

    #[error("Tool execution error: {0}")]
    Tool(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, MaxError>;
