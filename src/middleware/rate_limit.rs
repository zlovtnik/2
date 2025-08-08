use axum::{
    extract::{ConnectInfo, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tracing::{debug, info, warn};
use serde::Serialize;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Burst allowance (requests allowed above the limit temporarily)
    pub burst_allowance: u32,
    /// Whether to use Redis for distributed rate limiting
    pub use_redis: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60), // 1 minute
            burst_allowance: 10,
            use_redis: false,
        }
    }
}

/// Different rate limiting strategies
#[derive(Debug, Clone)]
pub enum RateLimitStrategy {
    /// By IP address
    ByIp,
    /// By user ID (requires authentication)
    ByUser,
    /// By API key
    ByApiKey,
    /// Global rate limit
    Global,
}

/// Rate limit bucket for tracking requests
#[derive(Debug, Clone)]
struct RateLimitBucket {
    /// Number of requests in current window
    requests: u32,
    /// Window start time
    window_start: u64,
    /// Total requests (for metrics)
    total_requests: u64,
    /// Last request time
    last_request: u64,
}

impl RateLimitBucket {
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            requests: 0,
            window_start: now,
            total_requests: 0,
            last_request: now,
        }
    }

    fn is_window_expired(&self, window_duration: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        now - self.window_start >= window_duration.as_millis() as u64
    }

    fn reset_window(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.window_start = now;
        self.requests = 0;
    }

    fn add_request(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.requests += 1;
        self.total_requests += 1;
        self.last_request = now;
    }
}

/// In-memory rate limiter using DashMap for concurrent access
#[derive(Debug)]
pub struct InMemoryRateLimiter {
    buckets: DashMap<String, RateLimitBucket>,
    config: RateLimitConfig,
}

impl InMemoryRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: DashMap::new(),
            config,
        }
    }

    pub async fn check_rate_limit(&self, key: &str) -> RateLimitResult {
        debug!(key = %key, "Checking rate limit for key");
        
        let mut bucket = self.buckets.entry(key.to_string())
            .or_insert_with(RateLimitBucket::new)
            .clone();

        // Check if window has expired
        if bucket.is_window_expired(self.config.window_duration) {
            bucket.reset_window();
        }

        // Check if request is allowed
        let allowed = if bucket.requests < self.config.max_requests {
            true
        } else if bucket.requests < self.config.max_requests + self.config.burst_allowance {
            // Allow burst but with a warning
            warn!(key = %key, requests = bucket.requests, "Rate limit burst allowance used");
            true
        } else {
            false
        };

        if allowed {
            bucket.add_request();
            self.buckets.insert(key.to_string(), bucket.clone());
            
            debug!(
                key = %key, 
                requests = bucket.requests, 
                max = self.config.max_requests,
                "Rate limit check passed"
            );
        } else {
            warn!(
                key = %key, 
                requests = bucket.requests, 
                max = self.config.max_requests,
                "Rate limit exceeded"
            );
        }

        RateLimitResult {
            allowed,
            requests_remaining: if bucket.requests <= self.config.max_requests {
                self.config.max_requests - bucket.requests
            } else {
                0
            },
            reset_time: bucket.window_start + self.config.window_duration.as_secs(),
            total_requests: bucket.total_requests,
        }
    }

    /// Clean up expired buckets (should be called periodically)
    pub async fn cleanup_expired(&self) {
        self.buckets.retain(|_, bucket| {
            // Keep buckets that are still within the window or recently active
            !bucket.is_window_expired(self.config.window_duration * 2)
        });
        
        debug!("Cleaned up expired rate limit buckets");
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub requests_remaining: u32,
    pub reset_time: u64,
    pub total_requests: u64,
}

/// Rate limiter that can use either Redis or in-memory storage
#[derive(Debug, Clone)]
pub enum RateLimiter {
    InMemory(Arc<InMemoryRateLimiter>),
    // Redis support can be added in the future when the redis feature is implemented
}

impl RateLimiter {
    /// Create a new in-memory rate limiter
    pub fn new_in_memory(config: RateLimitConfig) -> Self {
        Self::InMemory(Arc::new(InMemoryRateLimiter::new(config)))
    }

    /// Check rate limit for a given key
    pub async fn check_rate_limit(&self, key: &str) -> RateLimitResult {
        match self {
            Self::InMemory(limiter) => limiter.check_rate_limit(key).await,
        }
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) {
        match self {
            Self::InMemory(limiter) => limiter.cleanup_expired().await,
        }
    }
}

/// Rate limiting middleware
#[derive(Clone)]
pub struct RateLimitMiddleware {
    limiter: RateLimiter,
    strategy: RateLimitStrategy,
}

impl RateLimitMiddleware {
    pub fn new(limiter: RateLimiter, strategy: RateLimitStrategy) -> Self {
        Self { limiter, strategy }
    }

    /// Extract the rate limiting key based on the strategy
    fn extract_key(&self, request: &Request, headers: &HeaderMap) -> Option<String> {
        match &self.strategy {
            RateLimitStrategy::ByIp => {
                // Try to get IP from ConnectInfo or X-Forwarded-For header
                if let Some(connect_info) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
                    Some(format!("ip:{}", connect_info.ip()))
                } else if let Some(forwarded_for) = headers.get("x-forwarded-for") {
                    forwarded_for.to_str().ok()
                        .and_then(|s| s.split(',').next())
                        .map(|ip| format!("ip:{}", ip.trim()))
                } else if let Some(real_ip) = headers.get("x-real-ip") {
                    real_ip.to_str().ok()
                        .map(|ip| format!("ip:{}", ip.trim()))
                } else {
                    Some("ip:unknown".to_string())
                }
            }
            RateLimitStrategy::ByUser => {
                // Extract user ID from Authorization header (if present)
                headers.get("authorization")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|h| h.strip_prefix("Bearer "))
                    .and_then(|token| {
                        crate::core::auth::verify_jwt(token).ok()
                    })
                    .map(|user_id| format!("user:{}", user_id))
            }
            RateLimitStrategy::ByApiKey => {
                headers.get("x-api-key")
                    .and_then(|h| h.to_str().ok())
                    .map(|key| format!("api_key:{}", key))
            }
            RateLimitStrategy::Global => {
                Some("global".to_string())
            }
        }
    }

    /// Create the rate limiting middleware function
    pub async fn middleware(
        &self,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let headers = request.headers().clone();
        
        if let Some(key) = self.extract_key(&request, &headers) {
            let result = self.limiter.check_rate_limit(&key).await;
            
            if !result.allowed {
                warn!(key = %key, "Request blocked by rate limiter");
                
                let response = axum::Json(serde_json::json!({
                    "error": "Rate limit exceeded",
                    "details": "Too many requests. Please try again later.",
                    "requests_remaining": result.requests_remaining,
                    "reset_time": result.reset_time
                }));
                
                return Ok((StatusCode::TOO_MANY_REQUESTS, response).into_response());
            }
            
            info!(
                key = %key,
                remaining = result.requests_remaining,
                "Request allowed by rate limiter"
            );
            
            let mut response = next.run(request).await;
            
            // Add rate limit headers to response
            let headers = response.headers_mut();
            headers.insert("X-RateLimit-Remaining", result.requests_remaining.to_string().parse().unwrap());
            headers.insert("X-RateLimit-Reset", result.reset_time.to_string().parse().unwrap());
            
            Ok(response)
        } else {
            warn!("Could not extract rate limiting key, allowing request");
            Ok(next.run(request).await)
        }
    }
}

/// Convenience function to create IP-based rate limiting middleware
pub fn create_ip_rate_limiter(config: RateLimitConfig) -> RateLimitMiddleware {
    let limiter = RateLimiter::new_in_memory(config);
    RateLimitMiddleware::new(limiter, RateLimitStrategy::ByIp)
}

/// Convenience function to create user-based rate limiting middleware
pub fn create_user_rate_limiter(config: RateLimitConfig) -> RateLimitMiddleware {
    let limiter = RateLimiter::new_in_memory(config);
    RateLimitMiddleware::new(limiter, RateLimitStrategy::ByUser)
}

/// Convenience function to create global rate limiting middleware
pub fn create_global_rate_limiter(config: RateLimitConfig) -> RateLimitMiddleware {
    let limiter = RateLimiter::new_in_memory(config);
    RateLimitMiddleware::new(limiter, RateLimitStrategy::Global)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_rate_limit_basic() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(60),
            burst_allowance: 1,
            use_redis: false,
        };
        
        let limiter = InMemoryRateLimiter::new(config);
        let key = "test_key";
        
        // First 3 requests should be allowed
        for i in 1..=3 {
            let result = limiter.check_rate_limit(key).await;
            assert!(result.allowed, "Request {} should be allowed", i);
            assert_eq!(result.requests_remaining, 3 - i);
        }
        
        // 4th request should be allowed due to burst
        let result = limiter.check_rate_limit(key).await;
        assert!(result.allowed, "Burst request should be allowed");
        assert_eq!(result.requests_remaining, 0);
        
        // 5th request should be blocked
        let result = limiter.check_rate_limit(key).await;
        assert!(!result.allowed, "Request should be blocked");
    }

    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100), // Short window for testing
            burst_allowance: 0,
            use_redis: false,
        };
        
        let limiter = InMemoryRateLimiter::new(config);
        let key = "test_window_reset";
        
        // Use up the limit
        for i in 1..=2 {
            let result = limiter.check_rate_limit(key).await;
            assert!(result.allowed, "Request {} should be allowed", i);
        }
        
        // Next request should be blocked
        let result = limiter.check_rate_limit(key).await;
        assert!(!result.allowed, "Request should be blocked");
        
        // Wait for window to reset
        sleep(Duration::from_millis(150)).await;
        
        // Request should be allowed again
        let result = limiter.check_rate_limit(key).await;
        assert!(result.allowed, "Request should be allowed after window reset");
    }

    #[tokio::test]
    async fn test_rate_limit_different_keys() {
        let config = RateLimitConfig {
            max_requests: 1,
            window_duration: Duration::from_secs(60),
            burst_allowance: 0,
            use_redis: false,
        };
        
        let limiter = InMemoryRateLimiter::new(config);
        
        // Different keys should have separate limits
        let result1 = limiter.check_rate_limit("key1").await;
        assert!(result1.allowed);
        
        let result2 = limiter.check_rate_limit("key2").await;
        assert!(result2.allowed);
        
        // But same key should be limited
        let result3 = limiter.check_rate_limit("key1").await;
        assert!(!result3.allowed);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = RateLimitConfig {
            max_requests: 1,
            window_duration: Duration::from_millis(50),
            burst_allowance: 0,
            use_redis: false,
        };
        
        let limiter = InMemoryRateLimiter::new(config);
        
        // Add some entries
        limiter.check_rate_limit("key1").await;
        limiter.check_rate_limit("key2").await;
        
        assert_eq!(limiter.buckets.len(), 2);
        
        // Wait for expiration
        sleep(Duration::from_millis(120)).await;
        
        // Cleanup
        limiter.cleanup_expired().await;
        
        // Buckets should be cleaned up
        assert_eq!(limiter.buckets.len(), 0);
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_duration, Duration::from_secs(60));
        assert_eq!(config.burst_allowance, 10);
        assert!(!config.use_redis);
    }
}
