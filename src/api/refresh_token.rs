use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;
use crate::core::refresh_token::RefreshToken;
use crate::infrastructure::database::{Crud, PgCrud, UpdatableCrud};
use sqlx::{PgPool, Row};
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
        (status = 403, description = "Forbidden — cannot create token for another user", body = ErrorResponse),
        (status = 409, description = "Session token with ID already exists", body = ErrorResponse),
        (status = 500, description = "Database error during token creation", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_refresh_token(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>, Json(token): Json<RefreshToken>) -> impl IntoResponse {
    info!(token_id = %token.id, user_id = %token.user_id, auth_user_id = %user_id, "Creating new refresh token");
    // Ensure authenticated user is creating a token for themselves
    if token.user_id != user_id {
        warn!(token_id = %token.id, token_user_id = %token.user_id, auth_user_id = %user_id, "Authenticated user attempted to create a token for a different user");
        return (StatusCode::FORBIDDEN, ErrorResponse::new("Forbidden", Some("Cannot create token for another user".to_string()))).into_response();
    }
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
        (status = 403, description = "Forbidden — not the owner", body = ErrorResponse),
        (status = 404, description = "Session token not found or expired", body = ErrorResponse),
        (status = 500, description = "Database error during token retrieval", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_refresh_token(AuthenticatedUser(auth_user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    info!(token_id = %id, auth_user_id = %auth_user_id, "Getting refresh token");
    debug!("Creating refresh token CRUD instance");

    let crud = refresh_token_crud_box(pool);
    debug!(token_id = %id, "Executing refresh token read query");

    match crud.read(id).await {
        Ok(Some(token)) => {
            // Ensure the authenticated user owns this token
            if token.user_id != auth_user_id {
                warn!(token_id = %id, owner_id = %token.user_id, auth_user_id = %auth_user_id, "Authenticated user is not the owner of the refresh token");
                return (StatusCode::FORBIDDEN, ErrorResponse::new("Forbidden", Some("You are not the owner of this token".to_string()))).into_response();
            }

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
        (status = 403, description = "Forbidden — not the owner", body = ErrorResponse),
        (status = 404, description = "Session token not found", body = ErrorResponse),
        (status = 500, description = "Database error during token revocation", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_refresh_token(
    AuthenticatedUser(auth_user_id): AuthenticatedUser,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>
) -> impl IntoResponse {
    info!(token_id = %id, auth_user_id = %auth_user_id, "Deleting refresh token with ownership check and atomic delete");

    // Start a transaction for atomicity
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!(token_id = %id, auth_user_id = %auth_user_id, error = %e, "Failed to start transaction for refresh token delete");
            return (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response();
        }
    };

    // Lock the token row for update to prevent TOCTOU
    let select_sql = "SELECT user_id FROM refresh_tokens WHERE id = $1 FOR UPDATE";
    match sqlx::query(select_sql)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(row)) => {
            let owner_id: Uuid = match row.try_get("user_id") {
                Ok(id) => id,
                Err(e) => {
                    error!(error = %e, "Failed to get user_id from row");
                    let _ = tx.rollback().await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some("Failed to process token ownership".to_string()))).into_response();
                }
            };
            if owner_id != auth_user_id {
                warn!(token_id = %id, owner_id = %owner_id, auth_user_id = %auth_user_id, "Authenticated user is not the owner of the refresh token");
                // Rollback transaction
                let _ = tx.rollback().await;
                return (StatusCode::FORBIDDEN, ErrorResponse::new("Forbidden", Some("You are not the owner of this token".to_string()))).into_response();
            }
            // Owner matches, proceed to delete
            let delete_sql = "DELETE FROM refresh_tokens WHERE id = $1";
            match sqlx::query(delete_sql)
                .bind(id)
                .execute(&mut *tx)
                .await
            {
                Ok(res) if res.rows_affected() > 0 => {
                    let _ = tx.commit().await;
                    info!(token_id = %id, auth_user_id = %auth_user_id, affected_rows = res.rows_affected(), "Refresh token deleted successfully");
                    (StatusCode::NO_CONTENT, "").into_response()
                }
                Ok(_) => {
                    let _ = tx.rollback().await;
                    warn!(token_id = %id, auth_user_id = %auth_user_id, "Token disappeared before delete");
                    (StatusCode::NOT_FOUND, ErrorResponse::new("Token not found", None)).into_response()
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    error!(token_id = %id, auth_user_id = %auth_user_id, error = %e, "Failed to delete refresh token");
                    (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
                }
            }
        }
        Ok(None) => {
            let _ = tx.rollback().await;
            warn!(token_id = %id, auth_user_id = %auth_user_id, "Token not found for delete");
            (StatusCode::NOT_FOUND, ErrorResponse::new("Token not found", None)).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            error!(token_id = %id, auth_user_id = %auth_user_id, error = %e, "Failed to select refresh token for delete");
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorResponse::new("Database error", Some(e.to_string()))).into_response()
        }
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
        (status = 403, description = "Forbidden — not the owner", body = ErrorResponse),
        (status = 404, description = "Session token not found", body = ErrorResponse),
        (status = 500, description = "Database error during token update", body = ErrorResponse)
    ),
    tag = "Session & Token Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_refresh_token(AuthenticatedUser(auth_user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_token): Json<String>) -> impl IntoResponse {
    info!(token_id = %id, auth_user_id = %auth_user_id, "Updating refresh token");
    debug!("Creating refresh token CRUD instance for update");

    let crud: PgCrud<RefreshToken> = PgCrud::new(pool, "refresh_tokens");
    debug!(token_id = %id, "Checking if refresh token exists before update");

    // Fetch existing token so we can verify ownership before applying changes
    match crud.read(id).await {
        Ok(Some(existing)) => {
            info!(token_id = %id, user_id = %existing.user_id, "Refresh token found, verifying ownership");
            if existing.user_id != auth_user_id {
                warn!(token_id = %id, owner_id = %existing.user_id, auth_user_id = %auth_user_id, "Authenticated user is not the owner of the refresh token");
                return (StatusCode::FORBIDDEN, ErrorResponse::new("Forbidden", Some("You are not the owner of this token".to_string()))).into_response();
            }

            // Owner matches; proceed to update
            let update_fn = |mut t: RefreshToken| {
                t.token = new_token.clone();
                t
            };
            debug!(token_id = %id, "Executing refresh token update");
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
    use axum::{
        body::Body, 
        http::{Request, StatusCode}, 
        Router, 
        routing::post
    };
    use tower::ServiceExt; // for `oneshot`
    use serde_json::json;
    use chrono::Utc;
    use uuid::Uuid;
    use sqlx::PgPool;

    fn dummy_pool() -> PgPool {
        PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap()
    }

    fn app_with_auth() -> Router {
        use crate::middleware::auth::AuthenticatedUser;
        
        let app = Router::new()
            .route("/refresh_tokens", post(create_refresh_token))
            .with_state(dummy_pool());
            
        let layer = ServiceBuilder::new()
            .layer(Extension(AuthenticatedUser(Uuid::nil())));
            
        app.layer(layer)
    }

    #[tokio::test]
    async fn test_create_refresh_token_returns_500_on_db_error() {
        let token = json!({
            "id": Uuid::new_v4(),
            "user_id": Uuid::nil(),
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
        let res = app_with_auth().oneshot(req).await.unwrap();
        // Since the dummy pool is not connected, this should return 500
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
} 