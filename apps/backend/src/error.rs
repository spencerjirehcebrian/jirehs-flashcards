//! Error handling for the backend API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// API error types
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Error response body
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type) = match &self {
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Parse(_) => (StatusCode::BAD_REQUEST, "parse_error"),
            ApiError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
            ApiError::Migration(_) => (StatusCode::INTERNAL_SERVER_ERROR, "migration_error"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}

/// Result type alias for API operations
pub type Result<T> = std::result::Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unauthorized_status() {
        let error = ApiError::Unauthorized("invalid token".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_not_found_status() {
        let error = ApiError::NotFound("card 123".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_bad_request_status() {
        let error = ApiError::BadRequest("invalid input".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_parse_error_status() {
        let error = ApiError::Parse("invalid ID".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_internal_error_status() {
        let error = ApiError::Internal("unexpected error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_migration_error_status() {
        let error = ApiError::Migration("migration failed".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_display_unauthorized() {
        let error = ApiError::Unauthorized("invalid token".to_string());
        assert_eq!(error.to_string(), "Unauthorized: invalid token");
    }

    #[test]
    fn test_error_display_not_found() {
        let error = ApiError::NotFound("Card 123".to_string());
        assert_eq!(error.to_string(), "Not found: Card 123");
    }

    #[test]
    fn test_error_display_bad_request() {
        let error = ApiError::BadRequest("missing field".to_string());
        assert_eq!(error.to_string(), "Bad request: missing field");
    }

    #[test]
    fn test_error_display_parse() {
        let error = ApiError::Parse("invalid number".to_string());
        assert_eq!(error.to_string(), "Parse error: invalid number");
    }

    #[test]
    fn test_error_display_internal() {
        let error = ApiError::Internal("connection lost".to_string());
        assert_eq!(error.to_string(), "Internal error: connection lost");
    }
}
