use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use tokio::runtime::Runtime;
use tonic::transport::{Channel, Endpoint};
use uuid::Uuid;

// Mock gRPC client for benchmarking
struct MockGrpcClient {
    channel: Channel,
}

impl MockGrpcClient {
    async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = Endpoint::from_shared(endpoint.to_string())?
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(30));
        
        let channel = endpoint.connect_lazy();
        Ok(Self { channel })
    }

    async fn make_request(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Touch the channel so the field is read (avoid dead_code warning)
        let _ = &self.channel;
        // Simulate a gRPC request
        tokio::time::sleep(Duration::from_millis(1)).await;
        Ok(())
    }
}

// Simple connection pool for benchmarking
struct SimpleConnectionPool {
    connections: Vec<Channel>,
    current_index: usize,
}

impl SimpleConnectionPool {
    async fn new(endpoint: &str, pool_size: usize) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut connections = Vec::new();
        
        for _ in 0..pool_size {
            let endpoint = Endpoint::from_shared(endpoint.to_string())?
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(30));
            
            let channel = endpoint.connect_lazy();
            connections.push(channel);
        }
        
        Ok(Self {
            connections,
            current_index: 0,
        })
    }

    fn get_connection(&mut self) -> &Channel {
        let connection = &self.connections[self.current_index];
        self.current_index = (self.current_index + 1) % self.connections.len();
        connection
    }
}

async fn benchmark_without_pooling(endpoint: &str, num_requests: usize) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    
    for _ in 0..num_requests {
        let client = MockGrpcClient::new(endpoint).await?;
        client.make_request().await?;
    }
    
    Ok(start.elapsed())
}

async fn benchmark_with_pooling(endpoint: &str, num_requests: usize, pool_size: usize) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let mut pool = SimpleConnectionPool::new(endpoint, pool_size).await?;
    
    for _ in 0..num_requests {
        let _channel = pool.get_connection();
        // Simulate using the pooled connection
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    Ok(start.elapsed())
}

fn benchmark_grpc_connection_pooling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let endpoint = "http://127.0.0.1:8081"; // Mock endpoint for testing
    
    let mut group = c.benchmark_group("grpc_connection_pooling");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);
    
    // Test different numbers of concurrent requests
    for num_requests in [10, 50, 100, 200].iter() {
        // Benchmark without connection pooling
        group.bench_with_input(
            BenchmarkId::new("without_pooling", num_requests),
            num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    rt.block_on(async {
                        benchmark_without_pooling(endpoint, num_requests).await
                    })
                })
            },
        );
        
        // Benchmark with connection pooling (pool size 5)
        group.bench_with_input(
            BenchmarkId::new("with_pooling_5", num_requests),
            num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    rt.block_on(async {
                        benchmark_with_pooling(endpoint, num_requests, 5).await
                    })
                })
            },
        );
        
        // Benchmark with connection pooling (pool size 10)
        group.bench_with_input(
            BenchmarkId::new("with_pooling_10", num_requests),
            num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    rt.block_on(async {
                        benchmark_with_pooling(endpoint, num_requests, 10).await
                    })
                })
            },
        );
    }
    
    group.finish();
}

// Benchmark connection pool metrics collection
fn benchmark_connection_pool_metrics(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("connection_pool_metrics");
    group.measurement_time(Duration::from_secs(5));
    
    group.bench_function("metrics_collection", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate metrics collection overhead
                let start = std::time::Instant::now();
                
                // Simulate collecting various metrics
                let _total_connections = 10;
                let _active_connections = 8;
                let _available_connections = 8;
                let _connection_errors = 2;
                let _health_check_failures = 1;
                let _last_health_check = Some(std::time::Instant::now());
                
                // Simulate some computation
                tokio::time::sleep(Duration::from_micros(10)).await;
                
                start.elapsed()
            })
        })
    });
    
    group.finish();
}

// Benchmark health check performance
fn benchmark_health_checks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("health_checks");
    group.measurement_time(Duration::from_secs(5));
    
    group.bench_function("health_check_overhead", |b| {
        b.iter(|| {
            rt.block_on(async {
                let start = std::time::Instant::now();
                
                // Simulate health check logic
                let connections = vec![
                    (Uuid::new_v4(), true, std::time::Instant::now()),
                    (Uuid::new_v4(), true, std::time::Instant::now()),
                    (Uuid::new_v4(), false, std::time::Instant::now()),
                ];
                
                let healthy_count = connections.iter().filter(|(_, healthy, _)| *healthy).count();
                let total_count = connections.len();
                
                // Simulate some health check computation
                tokio::time::sleep(Duration::from_micros(5)).await;
                
                let _status = if healthy_count == 0 {
                    "unhealthy"
                } else if healthy_count < total_count {
                    "degraded"
                } else {
                    "healthy"
                };
                
                start.elapsed()
            })
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_grpc_connection_pooling,
    benchmark_connection_pool_metrics,
    benchmark_health_checks
);
criterion_main!(benches);
