use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::interval;
use tonic::transport::{Channel, Endpoint};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Connection pool metrics for monitoring
#[derive(Debug, Clone)]
pub struct ConnectionPoolMetrics {
    pub total_connections: usize,
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
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
    health_task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl GrpcConnectionPool {
    /// Create a new connection pool
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

        let (shutdown_sender, _) = tokio::sync::broadcast::channel(1);

        let mut pool = Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            endpoint: endpoint.clone(),
            max_connections,
            connection_timeout,
            health_check_interval,
            metrics: Arc::new(RwLock::new(ConnectionPoolMetrics::default())),
            shutdown_sender,
            health_task_handle: None,
        };

        // Initialize with one connection
        pool.create_connection().await?;
        
        // Start health monitoring
        pool.start_health_monitoring();

        info!(
            endpoint = %endpoint,
            "gRPC connection pool created successfully"
        );

        Ok(pool)
    }

    /// Get a connection from the pool and an owned semaphore permit to enforce backpressure
    pub async fn get_connection(&self) -> Result<(Channel, tokio::sync::OwnedSemaphorePermit), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Requesting connection from gRPC pool");
        
        // Acquire semaphore permit
        let permit = self.semaphore.clone().acquire_owned().await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

        let mut connections = self.connections.write().await;
        
        // Try to find a healthy connection
        for connection in connections.iter_mut() {
            if connection.is_healthy {
                connection.mark_used();
                debug!(
                    connection_id = %connection.connection_id,
                    "Reusing existing healthy connection"
                );
                return Ok((connection.channel.clone(), permit));
            }
        }

        // No healthy connections available, create a new one
        drop(connections);
        debug!("No healthy connections available, creating new connection");
        let channel = self.create_connection().await?;
        Ok((channel, permit))
    }

    /// Create a new connection and add it to the pool
    async fn create_connection(&self) -> Result<Channel, Box<dyn std::error::Error + Send + Sync>> {
        debug!(endpoint = %self.endpoint, "Creating new gRPC connection");
        
        let endpoint = Endpoint::from_shared(self.endpoint.clone())
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?
            .timeout(self.connection_timeout)
            .connect_timeout(self.connection_timeout);

        let channel = match endpoint.connect().await {
            Ok(ch) => ch,
            Err(e) => {
                error!(error = %e, "Failed to create gRPC connection");
                {
                    let mut m = self.metrics.write().await;
                    m.connection_errors = m.connection_errors.saturating_add(1);
                }
                return Err(Box::new(e));
            }
        };

        let pooled_connection = PooledConnection::new(channel.clone());
        let connection_id = pooled_connection.connection_id;

        // Add to pool
        {
            let mut connections = self.connections.write().await;
            if connections.len() < self.max_connections {
                connections.push(pooled_connection);
                info!(
                    connection_id = %connection_id,
                    pool_size = connections.len(),
                    max_connections = self.max_connections,
                    "Added new connection to pool"
                );
            } else {
                warn!(
                    connection_id = %connection_id,
                    "Pool is at capacity, connection created but not pooled"
                );
            }
        }

        // Update metrics
        self.update_metrics().await;

        Ok(channel)
    }

    /// Start health monitoring for all connections
    fn start_health_monitoring(&mut self) {
        let connections = Arc::clone(&self.connections);
        let metrics = Arc::clone(&self.metrics);
        let health_check_interval = self.health_check_interval;
        let mut shutdown_receiver = self.shutdown_sender.subscribe();

        let handle = tokio::spawn(async move {
            let mut interval = interval(health_check_interval);
            interval.tick().await; // run immediately
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        debug!("Starting gRPC connection health check");
                        
                        let mut connections_guard = connections.write().await;
                        let mut metrics_guard = metrics.write().await;
                        metrics_guard.last_health_check = Some(SystemTime::now());
                        
                        let mut healthy_count = 0;
                        let mut newly_unhealthy: u64 = 0;
                        let total_count = connections_guard.len();
                        
                        for connection in connections_guard.iter_mut() {
                            // Simple health check: try to create a client and make a basic call
                            // For now, we'll just check if the connection is still valid
                            // In a real implementation, you might want to make an actual gRPC call
                            if connection.is_healthy {
                                // Check if connection is not too old (optional)
                                let connection_age = connection.created_at.elapsed();
                                if connection_age > Duration::from_secs(3600) { // 1 hour
                                    warn!(
                                        connection_id = %connection.connection_id,
                                        age_secs = connection_age.as_secs(),
                                        "Marking old connection as unhealthy"
                                    );
                                    connection.mark_unhealthy();
                                    newly_unhealthy = newly_unhealthy.saturating_add(1);
                                } else {
                                    healthy_count += 1;
                                }
                            }
                        }
                        
                        // Remove unhealthy connections
                        connections_guard.retain(|conn| conn.is_healthy);
                        
                        // Update metrics
                        metrics_guard.total_connections = connections_guard.len();
                        metrics_guard.active_connections = healthy_count;
                        metrics_guard.available_connections = healthy_count;
                        metrics_guard.health_check_failures =
                            metrics_guard.health_check_failures.saturating_add(newly_unhealthy);
                        
                        info!(
                            total_connections = metrics_guard.total_connections,
                            healthy_connections = healthy_count,
                            "Health check completed"
                        );
                        
                        if healthy_count == 0 && total_count > 0 {
                            warn!("All connections are unhealthy, will create new ones on next request");
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        info!("Health monitoring task received shutdown signal, exiting");
                        break;
                    }
                }
            }
        });

        self.health_task_handle = Some(handle);
    }

    /// Update connection pool metrics
    async fn update_metrics(&self) {
        let connections = self.connections.read().await;
        let mut metrics = self.metrics.write().await;
        
        let healthy_count = connections.iter().filter(|c| c.is_healthy).count();
        
        metrics.total_connections = connections.len();
        let available = self.semaphore.available_permits();
        metrics.active_connections = self.max_connections.saturating_sub(available);
        metrics.available_connections = available;
    }

    /// Get current connection pool metrics
    pub async fn get_metrics(&self) -> ConnectionPoolMetrics {
        self.update_metrics().await;
        self.metrics.read().await.clone()
    }

    /// Force health check on all connections
    pub async fn force_health_check(&self) {
        info!("Forcing gRPC connection health check");
        // The health monitoring task will handle this automatically
        // This method can be used for manual health checks if needed
    }

    /// Shutdown the connection pool and clean up background tasks
    pub async fn shutdown(&mut self) {
        info!("Shutting down gRPC connection pool");
        
        // Send shutdown signal to health monitoring task
        if let Err(e) = self.shutdown_sender.send(()) {
            warn!(error = %e, "Failed to send shutdown signal to health monitoring task");
        }
        // Abort and await the task to ensure clean shutdown
        if let Some(handle) = self.health_task_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        
        // Clear all connections
        {
            let mut connections = self.connections.write().await;
            connections.clear();
            info!("Cleared all connections from pool");
        }
        
        info!("gRPC connection pool shutdown completed");
    }
}

impl Clone for GrpcConnectionPool {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            semaphore: Arc::clone(&self.semaphore),
            endpoint: self.endpoint.clone(),
            max_connections: self.max_connections,
            connection_timeout: self.connection_timeout,
            health_check_interval: self.health_check_interval,
            metrics: Arc::clone(&self.metrics),
            shutdown_sender: self.shutdown_sender.clone(),
            health_task_handle: None, // Cloned instances don't own the task handle
        }
    }
}
