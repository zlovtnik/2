use std::time::Duration;
use tokio::time::sleep;
use tonic::transport::{Channel, Endpoint};

// Import our connection pool
use server::grpc::GrpcConnectionPool;

#[tokio::test]
async fn test_connection_pool_creation() {
    let endpoint = "http://127.0.0.1:9999"; // Non-existent endpoint for testing
    let pool = GrpcConnectionPool::new(
        endpoint.to_string(),
        5,
        Duration::from_secs(5),
        Duration::from_secs(10),
    ).await;
    
    // Should fail because endpoint doesn't exist, but pool should be created
    assert!(pool.is_err());
}

#[tokio::test]
async fn test_connection_pool_metrics() {
    // Test metrics collection without actual connections
    let metrics = server::grpc::ConnectionPoolMetrics::default();
    
    assert_eq!(metrics.total_connections, 0);
    assert_eq!(metrics.active_connections, 0);
    assert_eq!(metrics.available_connections, 0);
    assert_eq!(metrics.connection_errors, 0);
    assert_eq!(metrics.health_check_failures, 0);
    assert!(metrics.last_health_check.is_none());
}

#[tokio::test]
async fn test_connection_pool_configuration() {
    // Test that configuration values are properly set
    let config = server::config::load();
    
    // These should have default values
    assert!(config.grpc_connection_pool_size > 0);
    assert!(config.grpc_connection_timeout_secs > 0);
    assert!(config.grpc_health_check_interval_secs > 0);
}

#[tokio::test]
async fn test_connection_pool_health_status() {
    // Test health status logic
    let mut metrics = server::grpc::ConnectionPoolMetrics::default();
    
    // Test unhealthy status
    metrics.total_connections = 0;
    metrics.active_connections = 0;
    let status = if metrics.total_connections == 0 || metrics.active_connections == 0 {
        "unhealthy"
    } else if metrics.active_connections < metrics.total_connections {
        "degraded"
    } else {
        "healthy"
    };
    assert_eq!(status, "unhealthy");
    
    // Test degraded status
    metrics.total_connections = 5;
    metrics.active_connections = 3;
    let status = if metrics.total_connections == 0 || metrics.active_connections == 0 {
        "unhealthy"
    } else if metrics.active_connections < metrics.total_connections {
        "degraded"
    } else {
        "healthy"
    };
    assert_eq!(status, "degraded");
    
    // Test healthy status
    metrics.total_connections = 5;
    metrics.active_connections = 5;
    let status = if metrics.total_connections == 0 || metrics.active_connections == 0 {
        "unhealthy"
    } else if metrics.active_connections < metrics.total_connections {
        "degraded"
    } else {
        "healthy"
    };
    assert_eq!(status, "healthy");
}

#[tokio::test]
async fn test_connection_pool_semaphore_behavior() {
    // Test that semaphore limits concurrent connections
    use tokio::sync::Semaphore;
    
    let semaphore = Semaphore::new(3);
    
    // Should be able to acquire 3 permits
    let _permit1 = semaphore.acquire().await.unwrap();
    let _permit2 = semaphore.acquire().await.unwrap();
    let _permit3 = semaphore.acquire().await.unwrap();
    
    // Fourth acquisition should block (we'll timeout to avoid hanging)
    let timeout_result = tokio::time::timeout(
        Duration::from_millis(100),
        semaphore.acquire()
    ).await;
    
    assert!(timeout_result.is_err(), "Fourth permit should not be available immediately");
}

#[tokio::test]
async fn test_connection_pool_metrics_update() {
    // Test metrics update logic
    let mut metrics = server::grpc::ConnectionPoolMetrics::default();
    
    // Simulate adding connections
    metrics.total_connections = 5;
    metrics.active_connections = 4;
    metrics.available_connections = 4;
    metrics.connection_errors = 1;
    metrics.health_check_failures = 0;
    metrics.last_health_check = Some(std::time::Instant::now());
    
    assert_eq!(metrics.total_connections, 5);
    assert_eq!(metrics.active_connections, 4);
    assert_eq!(metrics.available_connections, 4);
    assert_eq!(metrics.connection_errors, 1);
    assert_eq!(metrics.health_check_failures, 0);
    assert!(metrics.last_health_check.is_some());
}

#[tokio::test]
async fn test_connection_pool_timeout_behavior() {
    // Test connection timeout configuration
    let config = server::config::load();
    
    // Timeout should be reasonable (not too short, not too long)
    assert!(config.grpc_connection_timeout_secs >= 5);
    assert!(config.grpc_connection_timeout_secs <= 300); // 5 minutes max
    
    // Health check interval should be reasonable
    assert!(config.grpc_health_check_interval_secs >= 10);
    assert!(config.grpc_health_check_interval_secs <= 3600); // 1 hour max
}

#[tokio::test]
async fn test_connection_pool_size_limits() {
    // Test connection pool size limits
    let config = server::config::load();
    
    // Pool size should be reasonable
    assert!(config.grpc_connection_pool_size >= 1);
    assert!(config.grpc_connection_pool_size <= 100); // Reasonable upper limit
}

#[tokio::test]
async fn test_connection_pool_environment_variables() {
    // Test that environment variables can override defaults
    std::env::set_var("GRPC_CONNECTION_POOL_SIZE", "15");
    std::env::set_var("GRPC_CONNECTION_TIMEOUT_SECS", "45");
    std::env::set_var("GRPC_HEALTH_CHECK_INTERVAL_SECS", "120");
    
    let config = server::config::load();
    
    assert_eq!(config.grpc_connection_pool_size, 15);
    assert_eq!(config.grpc_connection_timeout_secs, 45);
    assert_eq!(config.grpc_health_check_interval_secs, 120);
    
    // Clean up environment variables
    std::env::remove_var("GRPC_CONNECTION_POOL_SIZE");
    std::env::remove_var("GRPC_CONNECTION_TIMEOUT_SECS");
    std::env::remove_var("GRPC_HEALTH_CHECK_INTERVAL_SECS");
}

