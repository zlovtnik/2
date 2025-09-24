use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;
use crate::core::user::User;
use crate::infrastructure::database::{Crud, PgCrud, UpdatableCrud};
use sqlx::{PgPool, FromRow};
use axum::http::StatusCode;
use crate::middleware::auth::AuthenticatedUser;
use crate::api::auth::ErrorResponse;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserInfoWithStats {
    pub user_id: Uuid,
    pub email: String,
    pub full_name: String,
    pub preferences: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub refresh_token_count: i64,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserWithRequesterId {
    pub user: User,
    pub requester_id: Uuid,
}

fn user_crud_box(pool: PgPool) -> Box<dyn Crud<User, Uuid> + Send + Sync> {
    Box::new(PgCrud::new(pool, "users"))
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = User,
    responses(
        (status = 201, description = "Kitchen staff member created successfully - Rate limit: 20 req/min with 3 burst allowance", body = User),
        (status = 409, description = "Kitchen staff member with email already exists", body = ErrorResponse),
        (status = 500, description = "Database error during staff creation", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_user(State(pool): State<PgPool>, Json(user): Json<User>) -> impl IntoResponse {
    info!(user_id = %user.id, email = %user.email, "Creating new user");
    debug!(user_id = %user.id, full_name = %user.full_name, "User creation details");
    
    // Direct SQLx for insert (as before)
    let query = "INSERT INTO users (id, email, password_hash, full_name, preferences, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *";
    debug!("Executing user insert query");
    
    match sqlx::query_as::<_, User>(query)
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(&user.preferences)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&pool)
        .await
    {
        Ok(inserted) => {
            info!(user_id = %inserted.id, "User created successfully");
            (StatusCode::CREATED, Json(inserted)).into_response()
        },
        Err(e) => {
            error!(user_id = %user.id, error = %e, "Failed to create user");
            if e.to_string().contains("duplicate key") {
                warn!(email = %user.email, "Attempted to create user with duplicate email");
                // NOTE: coderabbitai warning — status mapping may be incorrect.
                // The project's central ErrorResponse->HTTP status mapping can cause
                // domain errors like "User already exists" or "User not found" to
                // be returned with a 500 status. Consider returning an explicit
                // HTTP status here (preferred), e.g.:
                // (StatusCode::CONFLICT, Json(ErrorResponse::new("User already exists", Some("Email address is already registered".to_string())))).into_response()
                // Or adjust the central mapping so named errors map to appropriate
                // status codes. Similar locations that may need review: 111-116,
                // 150-155, 188-193, 245-248, 292-297, 303-307.
                (StatusCode::CONFLICT, Json(ErrorResponse::new("User already exists", Some("Email address is already registered".to_string())))).into_response()
            } else {
                ErrorResponse::new("Database error", Some(e.to_string())).into_response()
            }
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff member ID to retrieve")
    ),
    responses(
        (status = 200, description = "Kitchen staff member found - Rate limit: 50 req/min with 10 burst allowance", body = UserWithRequesterId),
        (status = 404, description = "Kitchen staff member not found", body = ErrorResponse),
        (status = 401, description = "Kitchen authentication required"),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user(State(pool): State<PgPool>, Path(id): Path<Uuid>, AuthenticatedUser(user_id): AuthenticatedUser) -> impl IntoResponse {
    info!(requested_user_id = %id, authenticated_user_id = %user_id, "Getting user");
    debug!("Creating user CRUD instance");
    
    let crud = user_crud_box(pool);
    debug!(user_id = %id, "Executing user read query");
    
    // For demo: return the authenticated user_id and the requested user
    match crud.read(id).await {
        Ok(Some(user)) => {
            info!(user_id = %id, authenticated_user_id = %user_id, "User retrieved successfully");
            debug!(user_email = "[redacted]", "User details retrieved");
            let resp = UserWithRequesterId { user, requester_id: user_id };
            (StatusCode::OK, Json(resp)).into_response()
        },
        Ok(None) => {
            warn!(user_id = %id, authenticated_user_id = %user_id, "User not found");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %id, authenticated_user_id = %user_id, error = %e, "Failed to retrieve user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
        },
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff member ID to remove")
    ),
    responses(
        (status = 204, description = "Kitchen staff member removed successfully - Rate limit: 10 req/min with 2 burst allowance"),
        (status = 404, description = "Kitchen staff member not found", body = ErrorResponse),
        (status = 500, description = "Database error during staff removal", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_user(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    info!(user_id = %id, authenticated_user_id = %user_id, "Deleting user");

    // Authorization: allow if requester is the same user. Admin role checks
    // are not implemented yet; extend this block if role claims are added.
    if user_id != id {
        warn!(requested_id = %id, authenticated_user_id = %user_id, "Unauthorized delete attempt - users may only delete their own account");
        return (StatusCode::FORBIDDEN, ErrorResponse::new("Forbidden", Some("You are not allowed to delete this user".to_string()))).into_response();
    }
    debug!("Creating user CRUD instance for deletion");
    
    let crud = user_crud_box(pool);
    debug!(user_id = %id, "Executing user delete query");
    
    match crud.delete(id).await {
        Ok(affected) if affected > 0 => {
            info!(user_id = %id, affected_rows = affected, "User deleted successfully");
            (StatusCode::NO_CONTENT, "").into_response()
        },
        Ok(affected) => {
            warn!(user_id = %id, affected_rows = affected, "User not found for deletion");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %id, error = %e, "Failed to delete user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/user/profile",
    responses(
        (status = 200, description = "Current kitchen staff member profile - Rate limit: 60 req/min with 15 burst allowance", body = User),
        (status = 404, description = "Kitchen staff member profile not found", body = ErrorResponse),
        (status = 401, description = "Kitchen authentication required"),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_current_user(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>) -> impl IntoResponse {
    info!(user_id = %user_id, "Getting current user profile");
    debug!("Creating user CRUD instance for current user");
    
    let crud = user_crud_box(pool);
    debug!(user_id = %user_id, "Executing current user read query");
    
    match crud.read(user_id).await {
        Ok(Some(user)) => {
            info!(user_id = %user_id, "Current user profile retrieved successfully");
            debug!(user_email = "[redacted]", full_name = "[redacted]", "Current user details");
            (StatusCode::OK, Json(user)).into_response()
        },
        Ok(None) => {
            warn!(user_id = %user_id, "Current user not found in database");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %user_id, error = %e, "Failed to retrieve current user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/user/stats",
    responses(
        (status = 200, description = "Current kitchen staff member statistics and performance metrics - Rate limit: 30 req/min with 5 burst allowance", body = UserInfoWithStats),
        (status = 404, description = "Kitchen staff member not found", body = ErrorResponse),
        (status = 401, description = "Kitchen authentication required"),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_current_user_stats(
    AuthenticatedUser(user_id): AuthenticatedUser, 
    State(pool): State<PgPool>
) -> impl IntoResponse {
    info!(user_id = %user_id, "Getting current user stats via PostgreSQL procedure");
    debug!("Calling get_user_info_with_stats procedure");
    
    // Call the PostgreSQL procedure with the authenticated user's ID
    let query = "SELECT * FROM get_user_info_with_stats($1)";
    
    match sqlx::query_as::<_, UserInfoWithStats>(query)
        .bind(user_id)
        .fetch_one(&pool)
        .await
    {
        Ok(user_stats) => {
            info!(
                user_id = %user_id, 
                email = %user_stats.email,
                refresh_token_count = user_stats.refresh_token_count,
                "User stats retrieved successfully via procedure"
            );
            debug!(
                full_name = %user_stats.full_name,
                created_at = %user_stats.created_at,
                last_login = ?user_stats.last_login,
                "Detailed user stats from procedure"
            );
            (StatusCode::OK, Json(user_stats)).into_response()
        },
        Err(e) => {
            error!(user_id = %user_id, error = %e, "Failed to retrieve user stats via procedure");
            if e.to_string().contains("not found") {
                warn!(user_id = %user_id, "User not found in procedure call");
                ErrorResponse::new("User not found", None).into_response()
            } else {
                ErrorResponse::new("Database error", Some(e.to_string())).into_response()
            }
        },
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff member ID to update")
    ),
    request_body = String,
    responses(
        (status = 200, description = "Kitchen staff member updated successfully - Rate limit: 20 req/min with 3 burst allowance", body = User),
        (status = 404, description = "Kitchen staff member not found", body = ErrorResponse),
        (status = 500, description = "Database error during staff update", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_user(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_name): Json<String>) -> impl IntoResponse {
    info!(user_id = %id, authenticated_user_id = %user_id, new_name = %new_name, "Updating user");

    // Authorization: only allow users to update their own profile. If role
    // claims (e.g., admin) are added to AuthenticatedUser, update this logic
    // to allow admins to update other users.
    if user_id != id {
        warn!(requested_id = %id, authenticated_user_id = %user_id, "Unauthorized update attempt - users may only update their own account");
        return (StatusCode::FORBIDDEN, ErrorResponse::new("Forbidden", Some("You are not allowed to update this user".to_string()))).into_response();
    }
    debug!("Creating user CRUD instance for update");
    
    let crud: PgCrud<User> = PgCrud::new(pool, "users");
    debug!(user_id = %id, "Checking if user exists before update");
    
    match crud.read(id).await {
        Ok(Some(existing)) => {
            info!(user_id = %id, old_name = %existing.full_name, new_name = %new_name, "User found, proceeding with update");
            let update_fn = |mut u: User| {
                u.full_name = new_name.clone();
                u
            };
            debug!(user_id = %id, "Executing user update");
            match UpdatableCrud::update(&crud, id, update_fn).await {
                Ok(Some(updated)) => {
                    info!(user_id = %id, updated_name = %updated.full_name, "User updated successfully");
                    (StatusCode::OK, Json(updated)).into_response()
                },
                Ok(None) => {
                    warn!(user_id = %id, "User not found during update operation");
                    (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
                },
                Err(e) => {
                    error!(user_id = %id, error = %e, "Failed to update user");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
                },
            }
        }
        Ok(None) => {
            warn!(user_id = %id, "User not found for update");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %id, error = %e, "Failed to check user existence before update");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
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

    // Dummy pool for demonstration (not a real DB connection)
    fn dummy_pool() -> PgPool {
        // This will panic if actually used, but allows us to test endpoint wiring
        PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap()
    }

    fn app() -> Router {
        Router::new()
            .route("/users", post(create_user))
            // Add more routes as needed
            .with_state(dummy_pool())
    }

    #[tokio::test]
    async fn test_create_user_returns_500_on_db_error() {
        let user = json!({
            "id": Uuid::new_v4(),
            "email": "test@example.com",
            "password_hash": "hash",
            "full_name": "Test User",
            "preferences": null,
            "created_at": Utc::now(),
            "updated_at": Utc::now()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/users")
            .header("content-type", "application/json")
            .body(Body::from(user.to_string()))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        // Since the dummy pool is not connected, this should return 500
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
} 