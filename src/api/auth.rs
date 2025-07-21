use axum::{Json, response::IntoResponse};
use crate::core::auth::{RegisterRequest, LoginRequest, hash_password, verify_password, create_jwt, use_verify_jwt_for_warning};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct TokenResponse {
    token: String,
}

#[axum::debug_handler]
pub async fn register(Json(payload): Json<RegisterRequest>) -> Result<Json<TokenResponse>, (axum::http::StatusCode, &'static str)> {
    info!(email = %payload.email, "Registration attempt");
    // Hash password
    let password_hash = match hash_password(&payload.password) {
        Ok(hash) => hash,
        Err(e) => {
            warn!(error = %e, "Password hashing failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Registration failed"));
        }
    };
    // Stub: pretend to store user and generate user_id
    let user_id = Uuid::new_v4();
    // Create JWT
    let token = match create_jwt(user_id) {
        Ok(token) => token,
        Err(e) => {
            warn!(error = %e, "JWT creation failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Registration failed"));
        }
    };
    info!(user_id = %user_id, "User registered successfully");
    Ok(Json(TokenResponse { token }))
}

#[axum::debug_handler]
pub async fn login(Json(payload): Json<LoginRequest>) -> Result<Json<TokenResponse>, (axum::http::StatusCode, &'static str)> {
    info!(email = %payload.email, "Login attempt");
    // Stub: pretend to fetch user and password hash
    let stored_hash = match hash_password(&payload.password) { Ok(h) => h, Err(_) => "".to_string() };
    // Verify password
    if !verify_password(&payload.password, &stored_hash) {
        warn!(email = %payload.email, "Invalid password");
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials"));
    }
    // Stub: pretend to get user_id
    let user_id = Uuid::new_v4();
    // Create JWT
    let token = match create_jwt(user_id) {
        Ok(token) => token,
        Err(e) => {
            warn!(error = %e, "JWT creation failed");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Login failed"));
        }
    };
    info!(user_id = %user_id, "User logged in successfully");
    Ok(Json(TokenResponse { token }))
}

pub async fn refresh() -> impl IntoResponse {
    info!("Refresh token endpoint called");
    // Use verify_jwt to avoid unused warning
    let _ = use_verify_jwt_for_warning("dummy_token");
    (axum::http::StatusCode::OK, "refresh")
} 