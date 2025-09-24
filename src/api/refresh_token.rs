use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;
use crate::core::refresh_token::RefreshToken;
use crate::infrastructure::database::{Crud, PgCrud, UpdatableCrud};
use sqlx::PgPool;
use axum::http::StatusCode;
use crate::api::auth::ErrorResponse;
use crate::middleware::auth::AuthenticatedUser;
use tracing::{info, warn, error, debug};

fn refresh_token_crud_box(pool: PgPool) -> Box<dyn Crud<RefreshToken, Uuid> + Send + Sync> {
    Box::new(PgCrud::new(pool, "refresh_tokens"))
}

#[utoipa::path(
    post,
    path = "/api/v1/refresh_tokens",
    request_body = RefreshToken,
    responses(
        (status = 201, description = "Kitchen staff session token created successfully - Rate limit: 30 req/min with 5 burst allowance", body = RefreshToken),
        (status = 409, description = "Session token with ID already exists", body = ErrorResponse),
        (status = 500, description = "Database error during token creation", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_refresh_token(AuthenticatedUser(_user_id): AuthenticatedUser, State(pool): State<PgPool>, Json(token): Json<RefreshToken>) -> impl IntoResponse {
    info!(token_id = %token.id, user_id = %token.user_id, "Creating new refresh token");
    debug!(token_id = %token.id, expires_at = %token.expires_at, "Refresh token creation details");
    
    // Direct SQLx for insert (trait object not needed for this demo)
    let query = "INSERT INTO refresh_tokens (id, user_id, token, expires_at, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING *";
    debug!("Executing refresh token insert query");
    
    match sqlx::query_as::<_, RefreshToken>(query)
        .bind(token.id)
        .bind(token.user_id)
        .bind(&token.token)
        .bind(token.expires_at)
        .bind(token.created_at)
        .fetch_one(&pool)
        .await
    {
        Ok(inserted) => {
            info!(token_id = %inserted.id, user_id = %inserted.user_id, "Refresh token created successfully");
            (StatusCode::CREATED, Json(inserted)).into_response()
        },
        Err(e) => {
            error!(token_id = %token.id, user_id = %token.user_id, error = %e, "Failed to create refresh token");
            if e.to_string().contains("duplicate key") {
                warn!(token_id = %token.id, "Attempted to create refresh token with duplicate ID");
                (StatusCode::CONFLICT, ErrorResponse::new("Token already exists", Some("Refresh token with this ID already exists".to_string()))).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
            }
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/refresh_tokens/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff session token ID to retrieve")
    ),
    responses(
        (status = 200, description = "Kitchen staff session token found - Rate limit: 60 req/min with 10 burst allowance", body = RefreshToken),
        (status = 404, description = "Session token not found or expired", body = ErrorResponse),
        (status = 500, description = "Database error during token retrieval", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_refresh_token(AuthenticatedUser(_user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    info!(token_id = %id, "Getting refresh token");
    debug!("Creating refresh token CRUD instance");
    
    let crud = refresh_token_crud_box(pool);
    debug!(token_id = %id, "Executing refresh token read query");
    
    match crud.read(id).await {
        Ok(Some(token)) => {
            info!(token_id = %id, user_id = %token.user_id, "Refresh token retrieved successfully");
            debug!(token_id = %id, expires_at = %token.expires_at, "Refresh token details retrieved");
            (StatusCode::OK, Json(token)).into_response()
        },
        Ok(None) => {
            warn!(token_id = %id, "Refresh token not found");
            (StatusCode::NOT_FOUND, ErrorResponse::new("Token not found", None)).into_response()
        },
        Err(e) => {
            error!(token_id = %id, error = %e, "Failed to retrieve refresh token");
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
        },
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/refresh_tokens/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff session token ID to revoke")
    ),
    responses(
        (status = 204, description = "Kitchen staff session token revoked successfully - Rate limit: 20 req/min with 3 burst allowance"),
        (status = 404, description = "Session token not found", body = ErrorResponse),
        (status = 500, description = "Database error during token revocation", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_refresh_token(AuthenticatedUser(_user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    info!(token_id = %id, "Deleting refresh token");
    debug!("Creating refresh token CRUD instance for deletion");
    
    let crud = refresh_token_crud_box(pool);
    debug!(token_id = %id, "Executing refresh token delete query");
    
    match crud.delete(id).await {
        Ok(affected) if affected > 0 => {
            info!(token_id = %id, affected_rows = affected, "Refresh token deleted successfully");
            (StatusCode::NO_CONTENT, "").into_response()
        },
        Ok(affected) => {
            warn!(token_id = %id, affected_rows = affected, "Refresh token not found for deletion");
            (StatusCode::NOT_FOUND, ErrorResponse::new("Token not found", None)).into_response()
        },
        Err(e) => {
            error!(token_id = %id, error = %e, "Failed to delete refresh token");
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
        },
    }
}

// For demonstration, a simple update handler that updates the token string
#[utoipa::path(
    put,
    path = "/api/v1/refresh_tokens/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff session token ID to update")
    ),
    request_body = String,
    responses(
        (status = 200, description = "Kitchen staff session token updated successfully - Rate limit: 30 req/min with 5 burst allowance", body = RefreshToken),
        (status = 404, description = "Session token not found", body = ErrorResponse),
        (status = 500, description = "Database error during token update", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_refresh_token(AuthenticatedUser(_user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_token): Json<String>) -> impl IntoResponse {
    info!(token_id = %id, "Updating refresh token");
    debug!("Creating refresh token CRUD instance for update");
    
    let crud: PgCrud<RefreshToken> = PgCrud::new(pool, "refresh_tokens");
    debug!(token_id = %id, "Checking if refresh token exists before update");
    
    // For demonstration, fetch, update, and save using the UpdatableCrud trait
    // In a real implementation, you'd want to update only the necessary fields
    match crud.read(id).await {
        Ok(Some(existing)) => {
            info!(token_id = %id, user_id = %existing.user_id, "Refresh token found, proceeding with update");
            let update_fn = |mut t: RefreshToken| {
                t.token = new_token.clone();
                t
            };
            debug!(token_id = %id, "Executing refresh token update");
            // Call update via the trait (note: this is a stub, real SQL needed)
            match UpdatableCrud::update(&crud, id, update_fn).await {
                Ok(Some(updated)) => {
                    info!(token_id = %id, user_id = %updated.user_id, "Refresh token updated successfully");
                    (StatusCode::OK, Json(updated)).into_response()
                },
                Ok(None) => {
                    warn!(token_id = %id, "Refresh token not found during update operation");
                    (StatusCode::NOT_FOUND, ErrorResponse::new("Token not found", None)).into_response()
                },
                Err(e) => {
                    error!(token_id = %id, error = %e, "Failed to update refresh token");
                    (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
                },
            }
        }
        Ok(None) => {
            warn!(token_id = %id, "Refresh token not found for update");
            (StatusCode::NOT_FOUND, ErrorResponse::new("Token not found", None)).into_response()
        },
        Err(e) => {
            error!(token_id = %id, error = %e, "Failed to check refresh token existence before update");
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}, Router, routing::post};
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot`
    use sqlx::PgPool;
    use chrono::Utc;
    use uuid::Uuid;

    fn dummy_pool() -> PgPool {
        PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap()
    }

    fn app() -> Router {
        Router::new()
            .route("/refresh_tokens", post(create_refresh_token))
            .with_state(dummy_pool())
    }

    #[tokio::test]
    async fn test_create_refresh_token_returns_500_on_db_error() {
        let token = json!({
            "id": Uuid::new_v4(),
            "user_id": Uuid::new_v4(),
            "token": "sometokenstring",
            "expires_at": Utc::now(),
            "created_at": Utc::now()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/refresh_tokens")
            .header("content-type", "application/json")
            .body(Body::from(token.to_string()))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        // Since the dummy pool is not connected, this should return 500
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
} 