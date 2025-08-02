use axum::{Json, extract::State, response::IntoResponse};
use sqlx::PgPool;
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
    pub database: &'static str,
    pub error: Option<String>,
}

pub async fn live() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "live")
}

pub async fn ready(State(pool): State<PgPool>) -> impl IntoResponse {
    let (db_status, db_error) = match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&pool).await {
        Ok(_) => ("ok", None),
        Err(e) => {
            error!(error = %e, "Database health check failed");
            ("error", Some(e.to_string()))
        }
    };
    let status = if db_status == "ok" { "ok" } else { "degraded" };
    let health = HealthStatus {
        status,
        database: db_status,
        error: db_error,
    };
    let code = if db_status == "ok" { axum::http::StatusCode::OK } else { axum::http::StatusCode::INTERNAL_SERVER_ERROR };
    (code, Json(health))
} 