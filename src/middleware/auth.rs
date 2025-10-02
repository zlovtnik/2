use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use crate::core::auth::verify_jwt;
use uuid::Uuid;
use async_trait::async_trait;
use tracing::{info, warn, error, debug};

/// Represents an authenticated user in the system.
/// 
/// This struct wraps a user ID and is used to represent an authenticated user
/// in the request handling pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthenticatedUser(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser   
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        debug!("Starting authentication middleware processing");
        
        let auth_header = parts.headers.get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));
            
        if let Some(token) = auth_header {
            debug!("Authorization header found, verifying JWT token");
            match verify_jwt(token) {
                Ok(user_id) => {
                    info!(user_id = %user_id, "Authentication successful");
                    Ok(AuthenticatedUser(user_id))
                },
                Err(e) => {
                    error!(error = %e, "Authentication failed - invalid or expired token");
                    Err((StatusCode::UNAUTHORIZED, "Invalid or expired token"))
                },
            }
        } else {
            warn!("Authentication failed - missing Authorization header");
            Err((StatusCode::UNAUTHORIZED, "Missing Authorization header"))
        }
    }
} 