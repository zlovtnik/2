use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;
use crate::core::refresh_token::RefreshToken;
use crate::infrastructure::database::{Crud, PgCrud, UpdatableCrud};
use sqlx::PgPool;
use axum::http::StatusCode;

fn refresh_token_crud_box(pool: PgPool) -> Box<dyn Crud<RefreshToken, Uuid> + Send + Sync> {
    Box::new(PgCrud::new(pool, "refresh_tokens"))
}

pub async fn create_refresh_token(State(pool): State<PgPool>, Json(token): Json<RefreshToken>) -> impl IntoResponse {
    // Direct SQLx for insert (trait object not needed for this demo)
    let query = "INSERT INTO refresh_tokens (id, user_id, token, expires_at, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING *";
    match sqlx::query_as::<_, RefreshToken>(query)
        .bind(token.id)
        .bind(token.user_id)
        .bind(&token.token)
        .bind(token.expires_at)
        .bind(token.created_at)
        .fetch_one(&pool)
        .await
    {
        Ok(inserted) => (StatusCode::CREATED, Json(inserted)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

pub async fn get_refresh_token(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let crud = refresh_token_crud_box(pool);
    match crud.read(id).await {
        Ok(Some(token)) => (StatusCode::OK, Json(token)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

pub async fn delete_refresh_token(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let crud = refresh_token_crud_box(pool);
    match crud.delete(id).await {
        Ok(affected) if affected > 0 => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

// For demonstration, a simple update handler that updates the token string
pub async fn update_refresh_token(State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_token): Json<String>) -> impl IntoResponse {
    let crud = PgCrud::new(pool, "refresh_tokens");
    // For demonstration, fetch, update, and save using the UpdatableCrud trait
    // In a real implementation, you'd want to update only the necessary fields
    match crud.read(id).await {
        Ok(Some(existing)) => {
            let update_fn = |mut t: RefreshToken| {
                t.token = new_token.clone();
                t
            };
            // Call update via the trait (note: this is a stub, real SQL needed)
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