use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
}

impl From<String> for CoreError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for CoreError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}
