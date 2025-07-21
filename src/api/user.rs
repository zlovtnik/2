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
    // Direct SQLx for insert (trait object not needed for this demo)
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
pub async fn update_user(State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_name): Json<String>) -> impl IntoResponse {
    let crud = PgCrud::new(pool, "users");
    match crud.read(id).await {
        Ok(Some(existing)) => {
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