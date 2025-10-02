use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Semaphore, broadcast};
use tokio::time::interval;
use tonic::transport::{Channel, Endpoint};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Connection pool metrics for monitoring
#[derive(Debug, Clone)]
pub struct ConnectionPoolMetrics {
    pub total_connections: usize,
    pub healthy_connections: usize,
    pub active_connections: usize,
    pub available_connections: usize,
    pub connection_errors: u64,
    pub health_check_failures: u64,
    pub last_health_check: Option<SystemTime>,
}

impl Default for ConnectionPoolMetrics {
    fn default() -> Self {
        Self {
            total_connections: 0,
            healthy_connections: 0,
            active_connections: 0,
            available_connections: 0,
            connection_errors: 0,
            health_check_failures: 0,
            last_health_check: None,
        }
    }
}

/// Individual connection wrapper with health tracking
#[derive(Debug)]
struct PooledConnection {
    channel: Channel,
    created_at: Instant,
    last_used: Instant,
    is_healthy: bool,
    connection_id: Uuid,
}

impl PooledConnection {
    fn new(channel: Channel) -> Self {
        let now = Instant::now();
        Self {
            channel,
            created_at: now,
            last_used: now,
            is_healthy: true,
            connection_id: Uuid::new_v4(),
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }

    fn mark_unhealthy(&mut self) {
        self.is_healthy = false;
    }
}

/// gRPC connection pool with health monitoring
pub struct GrpcConnectionPool {
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    endpoint: String,
    max_connections: usize,
    connection_timeout: Duration,
    health_check_interval: Duration,
    metrics: Arc<RwLock<ConnectionPoolMetrics>>,
    shutdown_sender: broadcast::Sender<()>,
    force_check_sender: broadcast::Sender<()>,
    health_task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl GrpcConnectionPool {
    /// Force an immediate health check on all connections
    /// 
    /// This will trigger the health monitoring task to perform a health check
    /// on all connections as soon as possible, rather than waiting for the next
    /// scheduled check interval.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the health check signal could not be sent. This typically
    /// means the health monitoring task has been shut down.
    pub fn force_health_check(&self) -> Result<(), tokio::sync::broadcast::error::SendError<()>> {
        debug!("Sending force health check signal");
        self.force_check_sender.send(()).map_err(|e| {
            warn!(error = %e, "Failed to send force health check signal");
            e
        })
    }

    /// Create a new connection pool with the specified configuration
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The gRPC server endpoint to connect to
    /// * `max_connections` - Maximum number of connections to maintain in the pool
    /// * `connection_timeout` - Timeout for establishing new connections
    /// * `health_check_interval` - Interval between automatic health checks
    ///
    /// # Returns
    ///
    /// Returns a new `GrpcConnectionPool` instance or an error if initialization fails
    pub async fn new(
        endpoint: String,
        max_connections: usize,
        connection_timeout: Duration,
        health_check_interval: Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            endpoint = %endpoint,
            max_connections = max_connections,
            connection_timeout_secs = connection_timeout.as_secs(),
            health_check_interval_secs = health_check_interval.as_secs(),
            "Creating gRPC connection pool"
        );

        let (shutdown_sender, _) = broadcast::channel(1);
        let (force_check_sender, _) = broadcast::channel(1);

        let pool = Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            endpoint: endpoint.clone(),
            max_connections,
            connection_timeout,
            health_check_interval,
            metrics: Arc::new(RwLock::new(Default::default())),
            shutdown_sender,
            force_check_sender,
            health_task_handle: None,
        };

        // Start health monitoring
        let mut pool_with_health = pool;
        pool_with_health.start_health_monitoring();
        
        // Initialize with one connection
        pool_with_health.create_connection().await?;
        
        Ok(pool_with_health)
    }

    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            semaphore: self.semaphore.clone(),
            endpoint: self.endpoint.clone(),
            max_connections: self.max_connections,
            connection_timeout: self.connection_timeout,
            health_check_interval: self.health_check_interval,
            metrics: self.metrics.clone(),
            shutdown_sender: self.shutdown_sender.clone(),
            force_check_sender: self.force_check_sender.clone(),
            health_task_handle: None, // Clones don't get a new health task
        }
    }
}
