// Integration tests for the whole app
use axum::http::StatusCode;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use std::env;

#[tokio::test]
async fn test_health_and_auth_endpoints() {
    // Set up environment variables for test DB and JWT secret
    env::set_var("APP_DATABASE_URL", "postgres://user:pass@localhost/test_db");
    env::set_var("APP_AUTH__JWT_SECRET", "testsecretkeytestsecretkeytestsecr");

    // Start the app in the background on a random port
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    // Import the app setup from main.rs (refactor main.rs if needed to expose the app as a function)
    let app = {
        use rust_jwt_backend as _; // Ensure the crate is linked
        use axum::{Router, routing::{get, post}};
        use rust_jwt_backend::api;
        use rust_jwt_backend::config;
        use rust_jwt_backend::core;
        use rust_jwt_backend::infrastructure;
        use rust_jwt_backend::middleware;
        use sqlx::PgPool;
        use tower_http::trace::TraceLayer;

        let config = config::load();
        let app = Router::new()
            .route("/health/live", get(api::health::live))
            .route("/health/ready", get(api::health::ready))
            .route("/api/v1/auth/register", post(api::auth::register))
            .route("/api/v1/auth/login", post(api::auth::login))
            .route("/api/v1/auth/refresh", post(api::auth::refresh))
            .layer(TraceLayer::new_for_http());
        let db_url = std::env::var("APP_DATABASE_URL").unwrap();
        let pool = PgPool::connect_lazy(&db_url).unwrap();
        app.with_state(pool)
    };

    // Spawn the server
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });
    sleep(Duration::from_millis(100)).await; // Give the server a moment to start

    let client = Client::new();

    // Test /health/live
    let res = client.get(format!("http://{}/health/live", local_addr)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Test /api/v1/auth/register (should succeed)
    let payload = serde_json::json!({
        "email": "integration@example.com",
        "password": "password123",
        "full_name": "Integration Test"
    });
    let res = client.post(format!("http://{}/api/v1/auth/register", local_addr))
        .json(&payload)
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
} 