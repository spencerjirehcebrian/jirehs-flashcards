//! Authentication middleware

use axum::{
    body::Body,
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::AppState;

/// Authenticated device info stored in request extensions
#[derive(Clone, Debug)]
pub struct AuthenticatedDevice {
    pub device_id: Uuid,
    pub token: String,
}

/// Auth middleware - extracts device token from Authorization header
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response> {
    // Skip auth for register endpoint and health check
    let path = request.uri().path();
    if path == "/api/device/register" || path == "/health" {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("Invalid Authorization format".to_string()))?
        .to_string();

    // Look up device by token
    let device = state
        .db
        .get_device_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid device token".to_string()))?;

    // Update last_seen
    state.db.update_last_seen(device.id).await?;

    // Store authenticated device in request extensions
    request.extensions_mut().insert(AuthenticatedDevice {
        device_id: device.id,
        token,
    });

    Ok(next.run(request).await)
}
