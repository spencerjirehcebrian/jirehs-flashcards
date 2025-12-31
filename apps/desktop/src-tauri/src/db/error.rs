//! Database error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("card not found: {0}")]
    CardNotFound(i64),

    #[error("invalid data: {0}")]
    InvalidData(String),
}
