use std::env;
use std::sync::Once;
use sqlx::PgPool;
use axum::Router;

/// Shared test helper to create the application router for integration tests.
///
/// This sets up environment defaults (e.g., `APP_DATABASE_URL` from
/// `TEST_DATABASE_URL` if present) and returns an `axum::Router` wired with
/// a `PgPool`.
static INIT_JWT_SECRET: Once = Once::new();

pub fn test_database_url() -> String {
    env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://user:pass@localhost/postgres_test".to_string()
    })
}

pub async fn create_test_app(database_url: &str) -> Router {
    use server::app;

    // Use a deterministic test JWT secret for tests
    INIT_JWT_SECRET.call_once(|| {
        if env::var("JWT_SECRET").is_err() {
            env::set_var("JWT_SECRET", "test-jwt-secret-not-for-production");
        }
    });

    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed to connect to DB eagerly");

    app(pool)
}
