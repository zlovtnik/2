use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::post,
    Router,
};
use serde_json::json;
use tower::ServiceExt;

use crate::api::auth::{register, login};
use crate::middleware::validation::validate_json_middleware;
use sqlx::PgPool;
use std::time::Duration;
use sqlx::postgres::PgPoolOptions;

// Create a test database connection pool
async fn test_pool() -> PgPool {
    let database_url = std::env::var("APP_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
        
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Failed to create test database pool")
}

async fn test_app() -> Router {
    let pool = test_pool().await;
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .layer(middleware::from_fn(validate_json_middleware))
        .with_state(pool)
}

#[tokio::test]
async fn test_validation_middleware_valid_registration() {
    std::env::set_var("APP_AUTH__JWT_SECRET", "test_secret_key_for_testing_jwt");
    let app = test_app().await;
    
    let payload = json!({
        "email": "test@example.com",
        "password": "StrongPassword123!",
        "full_name": "Test User"
    });
    
    let request = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Should either succeed (200) or fail due to duplicate email (400), 
    // but not fail due to validation (422)
    assert!(
        response.status() == StatusCode::OK || 
        response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_validation_middleware_invalid_email() {
    let app = test_app().await;
    
    let payload = json!({
        "email": "invalid-email",
        "password": "StrongPassword123!",
        "full_name": "Test User"
    });
    
    let request = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_weak_password() {
    let app = test_app().await;
    
    let payload = json!({
        "email": "test@example.com",
        "password": "weak",
        "full_name": "Test User"
    });
    
    let request = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_invalid_json() {
    let app = test_app().await;
    
    let request = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"email": invalid}"#))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_missing_fields() {
    let app = test_app().await;
    
    let payload = json!({
        "email": "test@example.com"
        // Missing password and full_name
    });
    
    let request = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_validation_middleware_login_validation() {
    let app = test_app().await;
    
    // Test invalid email
    let payload = json!({
        "email": "invalid-email",
        "password": "password123"
    });
    
    let request = Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
