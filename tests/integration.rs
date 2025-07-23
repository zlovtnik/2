// Integration tests for the whole app
use axum::http::StatusCode;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use std::env;

async fn app() -> axum::Router {
    use server as _; // Ensure the crate is linked
    use axum::{Router, routing::{get, post}};
    use server::api;
    use server::config;
    use sqlx::PgPool;
    use tower_http::trace::TraceLayer;

    let _config = config::load();
    let stateful_app = Router::new()
        .route("/health/live", get(api::health::live))
        .route("/health/ready", get(api::health::ready))
        .route("/api/v1/auth/register", post(api::auth::register))
        .route("/api/v1/auth/login", post(api::auth::login))
        .route("/api/v1/auth/refresh", post(api::auth::refresh));
    let db_url = std::env::var("APP_DATABASE_URL").unwrap();
    let pool = PgPool::connect_lazy(&db_url).unwrap();
    let stateful_app = stateful_app.with_state(pool);
    
    // Create a stateless router by merging the stateful one
    Router::new()
        .merge(stateful_app)
        .layer(TraceLayer::new_for_http())
}

#[tokio::test]
async fn test_health_and_auth_endpoints() {
    // Set up environment variables for test DB and JWT secret
    env::set_var("APP_DATABASE_URL", "postgres://user:pass@localhost/postgres");
    env::set_var("JWT_SECRET", "your-super-secret-jwt-key-here");

    // Start the app in the background on a random port
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let app = app().await.into_make_service();

    // Spawn the server
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    sleep(Duration::from_millis(100)).await; // Give the server a moment to start

    let client = Client::new();

    // Test /health/live
    let res = client.get(format!("http://{}/health/live", local_addr)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Test /api/v1/auth/register (should succeed)
    let payload = serde_json::json!({
        "email": "teste@teste.com",
        "password": "teste123",
        "full_name": "Integration Test"
    });
    let res = client.post(format!("http://{}/api/v1/auth/register", local_addr))
        .json(&payload)
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
} 