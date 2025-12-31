//! Device registration and status endpoints

use axum::{extract::State, Extension, Json};

use crate::error::Result;
use crate::models::{DeviceRegisterRequest, DeviceRegisterResponse, DeviceStatusResponse};
use crate::routes::auth::AuthenticatedDevice;
use crate::AppState;

/// POST /api/device/register
/// Creates a new device and returns the token
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<Option<DeviceRegisterRequest>>,
) -> Result<Json<DeviceRegisterResponse>> {
    let name = payload.and_then(|p| p.name);
    let device = state.db.create_device(name.as_deref()).await?;

    tracing::info!("Registered new device: {}", device.id);

    Ok(Json(DeviceRegisterResponse {
        device_id: device.id,
        token: device.token,
    }))
}

/// GET /api/device/status
/// Returns device status
pub async fn status(
    Extension(auth): Extension<AuthenticatedDevice>,
    State(state): State<AppState>,
) -> Result<Json<DeviceStatusResponse>> {
    let device = state
        .db
        .get_device_by_token(&auth.token)
        .await?
        .ok_or_else(|| crate::error::ApiError::NotFound("Device not found".to_string()))?;

    Ok(Json(DeviceStatusResponse {
        device_id: device.id,
        last_seen_at: device.last_seen_at,
    }))
}
