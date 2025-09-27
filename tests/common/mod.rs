use std::env;
use sqlx::PgPool;
use axum::Router;

/// Shared test helper to create the application router for integration tests.
///
/// This sets up environment defaults (e.g., `APP_DATABASE_URL` from
/// `TEST_DATABASE_URL` if present) and returns an `axum::Router` wired with
/// a `PgPool`.
pub async fn create_test_app() -> Router {
    use server::app;

    // Set up test environment: prefer TEST_DATABASE_URL if provided, otherwise
    // fall back to a local test database URL. Tests/CI can override with
    // TEST_DATABASE_URL.
    env::set_var(
        "APP_DATABASE_URL",
        env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://user:pass@localhost/postgres_test".to_string()
        }),
    );

    // Use a deterministic test JWT secret for tests
    env::set_var("JWT_SECRET", "test-jwt-secret-not-for-production");

    let db_url = env::var("APP_DATABASE_URL").expect("APP_DATABASE_URL must be set");
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to DB eagerly");

    app(pool)
}
