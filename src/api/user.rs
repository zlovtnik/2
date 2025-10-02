#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicUser {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub preferences: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&User> for PublicUser {
    fn from(user: &User) -> Self {
        PublicUser {
            id: user.id,
            email: user.email.clone(),
            full_name: user.full_name.clone(),
            preferences: user.preferences.clone(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}
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
use crate::middleware::validation::InputSanitizer;

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

// The following code was misplaced and caused a syntax error. 
// If you want to document responses, add them inside the #[utoipa::path(...)] attribute for your handler function.
// If you want to handle user creation, implement a `create_user` handler function as shown below:

#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserPayload,
    responses(
        (status = 201, description = "Kitchen staff member created successfully - Rate limit: 20 req/min with 3 burst allowance", body = PublicUser),
        (status = 409, description = "User already exists", body = ErrorResponse),
        (status = 500, description = "Database error", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Management"
)]
pub async fn create_user(State(pool): State<PgPool>, Json(mut payload): Json<CreateUserPayload>) -> impl IntoResponse {
    payload.sanitize();
    let pool_closed = pool.is_closed();
    debug!(pool_closed, user_email = %payload.email, "create_user handler invoked but not implemented");
    error!("create_user handler called, but not implemented");
    (StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse::new("Not implemented", None))).into_response()
}

// Define the payload struct for user creation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUserPayload {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub preferences: Option<serde_json::Value>,
}

impl CreateUserPayload {
    fn sanitize(&mut self) {
        self.email = InputSanitizer::sanitize_email(&self.email);
        self.full_name = InputSanitizer::sanitize_text(&self.full_name);
    }
}


#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "Kitchen staff member ID to retrieve")
    ),
    responses(
        (status = 200, description = "Kitchen staff member found - Rate limit: 50 req/min with 10 burst allowance", body = PublicUser),
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
    info!(requested_user_id = %id.to_string(), authenticated_user_id = %user_id.to_string(), "Getting user");
    debug!("Creating user CRUD instance");
    
    let crud = PgCrud::new(pool, "users");
    debug!(user_id = %id, "Executing user read query");
    
    match crud.read(id).await {
        Ok(Some(user)) => {
            info!(user_id = %id.to_string(), authenticated_user_id = %user_id.to_string(), "User retrieved successfully");
            debug!(user_email = "[redacted]", "User details retrieved");
            let public_user = PublicUser::from(&user);
            (StatusCode::OK, Json(public_user)).into_response()
        },
        Ok(None) => {
            warn!(user_id = %id.to_string(), authenticated_user_id = %user_id.to_string(), "User not found");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %id.to_string(), authenticated_user_id = %user_id.to_string(), error = %e, "Failed to retrieve user");
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
        (status = 403, description = "Forbidden - cannot delete another user", body = ErrorResponse),
        (status = 404, description = "Kitchen staff member not found", body = ErrorResponse),
        (status = 500, description = "Database error during staff removal", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_user(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    info!(user_id = %id.to_string(), authenticated_user_id = %user_id.to_string(), "Deleting user");
    // Authorization: allow if requester is the same user. Admin role checks
    // are not implemented yet; extend this block if role claims are added.
    if user_id != id {
        warn!(requested_id = %id.to_string(), authenticated_user_id = %user_id.to_string(), "Unauthorized delete attempt - users may only delete their own account");
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Forbidden", Some("You are not allowed to delete this user".to_string())))).into_response();
    }
    debug!("Creating user CRUD instance for deletion");
    
    let crud: PgCrud<User> = PgCrud::new(pool, "users");
    debug!(user_id = %id.to_string(), "Executing user delete query");
    
    match crud.delete(id).await {
        Ok(affected) if affected > 0 => {
            info!(user_id = %id.to_string(), affected_rows = affected, "User deleted successfully");
            (StatusCode::NO_CONTENT, "").into_response()
        },
        Ok(affected) => {
            warn!(user_id = %id.to_string(), affected_rows = affected, "User not found for deletion");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %id.to_string(), error = %e, "Failed to delete user");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/user/profile",
    responses(
        (status = 200, description = "Current kitchen staff member profile - Rate limit: 60 req/min with 15 burst allowance", body = PublicUser),
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
    info!(user_id = %user_id.to_string(), "Getting current user profile");
    debug!("Creating user CRUD instance for current user");
    
    let crud = PgCrud::new(pool, "users");
    debug!(user_id = %user_id.to_string(), "Executing current user read query");
    
    match crud.read(user_id).await {
        Ok(Some(user)) => {
            info!(user_id = %user_id.to_string(), "Current user profile retrieved successfully");
            debug!(user_email = "[redacted]", full_name = "[redacted]", "Current user details");
            let public_user = PublicUser::from(&user);
            (StatusCode::OK, Json(public_user)).into_response()
        },
        Ok(None) => {
            warn!(user_id = %user_id.to_string(), "Current user not found in database");
            (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
        },
        Err(e) => {
            error!(user_id = %user_id.to_string(), error = %e, "Failed to retrieve current user");
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
                (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found", None))).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Database error", Some(e.to_string())))).into_response()
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
        (status = 200, description = "Kitchen staff member updated successfully - Rate limit: 20 req/min with 3 burst allowance", body = PublicUser),
        (status = 403, description = "Forbidden - user may only update their own account", body = ErrorResponse),
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
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Forbidden", Some("You are not allowed to update this user".to_string())))).into_response();
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
                    let public_user = PublicUser::from(&updated);
                    (StatusCode::OK, Json(public_user)).into_response()
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
    use axum::{body::Body, http::{Request, StatusCode}, Router, routing::post};
    use serde_json::json;
    use sqlx::PgPool;
    use tower::ServiceExt; // for `oneshot`

    // Dummy pool for demonstration (not a real DB connection)
    fn dummy_pool() -> PgPool {
        // This will panic if actually used, but allows us to test endpoint wiring
        PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap()
    }

    fn app() -> Router {
        use super::create_user;
        Router::new()
            .route("/users", post(create_user))
            // Add more routes as needed
            .with_state(dummy_pool())
    }

    #[tokio::test]
    async fn test_create_user_returns_501_not_implemented() {
        let user = json!({
            "email": "test@example.com",
            "password": "StrongPass123!",
            "full_name": "Test User",
            "preferences": null
        });
        let req = Request::builder()
            .method("POST")
            .uri("/users")
            .header("content-type", "application/json")
            .body(Body::from(user.to_string()))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
