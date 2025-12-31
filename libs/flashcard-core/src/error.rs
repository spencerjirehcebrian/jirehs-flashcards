//! Error types for flashcard-core.

use thiserror::Error;

/// Result type alias using ParseError.
pub type Result<T> = std::result::Result<T, ParseError>;

/// Errors that can occur during markdown parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing question at line {line}")]
    MissingQuestion { line: usize },

    #[error("missing answer at line {line}")]
    MissingAnswer { line: usize },

    #[error("invalid ID format at line {line}: {value}")]
    InvalidId { line: usize, value: String },

    #[error("duplicate ID {id} at line {line}")]
    DuplicateId { id: i64, line: usize },

    #[error("empty file")]
    EmptyFile,
}
