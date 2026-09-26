use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Discord error: {0}")]
    Discord(String),
    #[error("external API error: {0}")]
    External(String),
    #[error("not authorized")]
    Forbidden,
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type AppResult<T> = Result<T, AppError>;
