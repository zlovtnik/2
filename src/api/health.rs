//! Health check endpoints for monitoring system status and dependencies.
//!
//! This module provides health check endpoints that are essential for:
//! - Load balancer health checks
//! - Container orchestration (Docker, Kubernetes)
//! - Monitoring and alerting systems
//! - Service discovery and registration
//!
//! # Endpoints
//!
//! - `/health/live` - Liveness probe (application is running)
//! - `/health/ready` - Readiness probe (application can serve traffic)
//!
//! # Examples
//!
//! ## Liveness Check
//!
//! ```rust
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let response = client
//!     .get("http://localhost:3000/health/live")
//!     .send()
//!     .await?;
//!
//! assert_eq!(response.status(), 200);
//! assert_eq!(response.text().await?, "live");
//! # Ok(())
//! # }
//! ```
//!
//! ## Readiness Check
//!
//! ```rust
//! use reqwest::Client;
//! use serde_json::Value;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let response = client
//!     .get("http://localhost:3000/health/ready")
//!     .send()
//!     .await?;
//!
//! let health: Value = response.json().await?;
//! println!("Database status: {}", health["database"]);
//! # Ok(())
//! # }
//! ```

use axum::{Json, extract::State, response::IntoResponse};
use sqlx::PgPool;
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;

/// Health status response structure containing system and dependency status information.
///
/// This structure provides detailed health information for monitoring systems
/// and load balancers to make routing decisions.
///
/// # Fields
///
/// * `status` - Overall system status: "ok", "degraded", or "error"
/// * `database` - Database connection status: "ok" or "error"
/// * `error` - Optional error message when status is not "ok"
///
/// # Status Meanings
///
/// - **"ok"** - All systems operational, ready to serve traffic
/// - **"degraded"** - Some non-critical issues, can still serve traffic
/// - **"error"** - Critical issues, should not receive traffic
///
/// # Examples
///
/// ```rust
/// use kitchen_api::api::health::HealthStatus;
///
/// // Healthy system
/// let healthy = HealthStatus {
///     status: "ok",
///     database: "ok",
///     error: None,
/// };
///
/// // System with database issues
/// let degraded = HealthStatus {
///     status: "degraded",
///     database: "error",
///     error: Some("Connection timeout".to_string()),
/// };
/// ```
#[derive(Serialize, ToSchema)]
pub struct HealthStatus {
    pub status: &'static str,
    pub database: &'static str,
    pub error: Option<String>,
}

/// Liveness probe endpoint that indicates if the application is running.
///
/// This endpoint is used by container orchestrators (Docker, Kubernetes) and
/// load balancers to determine if the application process is alive and should
/// continue running. It performs minimal checks and should always return quickly.
///
/// # Returns
///
/// * `200 OK` with "live" text - Application is running
///
/// # Usage in Kubernetes
///
/// ```yaml
/// livenessProbe:
///   httpGet:
///     path: /health/live
///     port: 3000
///   initialDelaySeconds: 30
///   periodSeconds: 10
/// ```
///
/// # Usage in Docker Compose
///
/// ```yaml
/// healthcheck:
///   test: ["CMD", "curl", "-f", "http://localhost:3000/health/live"]
///   interval: 30s
///   timeout: 10s
///   retries: 3
/// ```
///
/// # Kitchen Management Context
///
/// This endpoint ensures that the kitchen management system remains available
/// during critical service periods. If this check fails, the container will
/// be restarted automatically.
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let response = client
///     .get("http://localhost:3000/health/live")
///     .send()
///     .await?;
///
/// assert_eq!(response.status(), 200);
/// assert_eq!(response.text().await?, "live");
/// println!("Application is alive and running");
/// # Ok(())
/// # }
/// ```
#[utoipa::path(
    get,
    path = "/health/live",
    responses(
        (status = 200, description = "Kitchen management system is alive and operational - Rate limit: 300 req/min with 50 burst allowance")
    ),
    tag = "System Health & Monitoring"
)]
pub async fn live() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "live")
}

/// Readiness probe endpoint that indicates if the application can serve traffic.
///
/// This endpoint performs comprehensive health checks of all critical dependencies
/// (database, external services) to determine if the application is ready to
/// handle requests. Unlike the liveness probe, this can temporarily fail during
/// startup or dependency issues.
///
/// # Arguments
///
/// * `pool` - Database connection pool for health verification
///
/// # Returns
///
/// * `200 OK` with health status JSON - All dependencies healthy
/// * `500 Internal Server Error` with error details - Dependencies unhealthy
///
/// # Health Checks Performed
///
/// 1. **Database Connectivity** - Executes `SELECT 1` query to verify connection
/// 2. **Connection Pool Status** - Ensures database pool is available
///
/// # Usage in Kubernetes
///
/// ```yaml
/// readinessProbe:
///   httpGet:
///     path: /health/ready
///     port: 3000
///   initialDelaySeconds: 5
///   periodSeconds: 5
/// ```
///
/// # Kitchen Management Context
///
/// This endpoint ensures that the kitchen management system can access all
/// required data (user accounts, orders, inventory) before accepting traffic.
/// Critical for maintaining data consistency during deployments.
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
/// use serde_json::Value;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let response = client
///     .get("http://localhost:3000/health/ready")
///     .send()
///     .await?;
///
/// let health: Value = response.json().await?;
/// 
/// match health["status"].as_str() {
///     Some("ok") => {
///         println!("System ready to serve traffic");
///         println!("Database status: {}", health["database"]);
///     }
///     Some("degraded") => {
///         println!("System degraded: {}", health["error"]);
///     }
///     _ => {
///         println!("System not ready");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Response Examples
///
/// ## Healthy System
/// ```json
/// {
///   "status": "ok",
///   "database": "ok",
///   "error": null
/// }
/// ```
///
/// ## Degraded System
/// ```json
/// {
///   "status": "degraded",
///   "database": "error",
///   "error": "connection refused"
/// }
/// ```
#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "Kitchen management system is ready to serve traffic - Rate limit: 300 req/min with 50 burst allowance", body = HealthStatus),
        (status = 500, description = "Kitchen management system is not ready - dependencies unavailable", body = HealthStatus)
    ),
    tag = "System Health & Monitoring"
)]
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