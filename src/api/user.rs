use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;
use crate::core::user::User;
use crate::infrastructure::database::{Crud, PgCrud, UpdatableCrud};
use sqlx::PgPool;
use axum::http::StatusCode;
use crate::middleware::auth::AuthenticatedUser;

fn user_crud_box(pool: PgPool) -> Box<dyn Crud<User, Uuid> + Send + Sync> {
    Box::new(PgCrud::new(pool, "users"))
}

pub async fn create_user(State(pool): State<PgPool>, Json(user): Json<User>) -> impl IntoResponse {
    // Direct SQLx for insert (as before)
    let query = "INSERT INTO users (id, email, password_hash, full_name, preferences, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *";
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
        Ok(inserted) => (StatusCode::CREATED, Json(inserted)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

pub async fn get_user(State(pool): State<PgPool>, Path(id): Path<Uuid>, AuthenticatedUser(user_id): AuthenticatedUser) -> impl IntoResponse {
    let crud = user_crud_box(pool);
    // For demo: return the authenticated user_id and the requested user
    match crud.read(id).await {
        Ok(Some(user)) => (StatusCode::OK, Json((user, user_id))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

pub async fn delete_user(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let crud = user_crud_box(pool);
    match crud.delete(id).await {
        Ok(affected) if affected > 0 => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

// For demonstration, a simple update handler that updates the full_name
pub async fn get_current_user(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>) -> impl IntoResponse {
    let crud = user_crud_box(pool);
    match crud.read(user_id).await {
        Ok(Some(user)) => (StatusCode::OK, Json(user)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

pub async fn update_user(State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_name): Json<String>) -> impl IntoResponse {
    let crud = PgCrud::new(pool, "users");
    match crud.read(id).await {
        Ok(Some(_existing)) => {
            let update_fn = |mut u: User| {
                u.full_name = new_name.clone();
                u
            };
            match UpdatableCrud::update(&crud, id, update_fn).await {
                Ok(Some(updated)) => (StatusCode::OK, Json(updated)).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
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