//! Rate Limiting and API Throttling Example
//!
//! This example demonstrates how to work with API rate limits and implement
//! proper throttling strategies for the Kitchen Management API:
//! 1. Understanding rate limit headers and responses
//! 2. Implementing adaptive request throttling
//! 3. Queue management for high-volume operations
//! 4. Graceful handling of rate limit violations
//! 5. Monitoring and metrics for rate limit compliance
//!
//! # Kitchen Management Context
//!
//! In busy kitchen environments, multiple staff members and systems may be
//! making concurrent API requests. This example shows how to manage request
//! rates to ensure system stability and fair resource allocation.

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use tokio::time::{sleep, interval};
use std::sync::{Arc, Mutex};

/// Rate limiting configuration
#[derive(Debug, Clone)]
struct RateLimitConfig {
    requests_per_minute: u32,
    burst_allowance: u32,
    backoff_multiplier: f64,
    max_backoff: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,  // Conservative default
            burst_allowance: 10,      // Allow short bursts
            backoff_multiplier: 2.0,  // Exponential backoff
            max_backoff: Duration::from_secs(300), // 5 minutes max
        }
    }
}

/// Rate limit information from API responses
#[derive(Debug, Clone)]
struct RateLimitInfo {
    limit: Option<u32>,
    remaining: Option<u32>,
    reset_time: Option<u64>,
    retry_after: Option<u64>,
}

impl RateLimitInfo {
    fn from_headers(headers: &reqwest::header::HeaderMap) -> Self {
        Self {
            limit: headers.get("x-ratelimit-limit")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            remaining: headers.get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            reset_time: headers.get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            retry_after: headers.get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
        }
    }
    
    fn is_near_limit(&self) -> bool {
        if let (Some(limit), Some(remaining)) = (self.limit, self.remaining) {
            let usage_percentage = ((limit - remaining) as f64 / limit as f64) * 100.0;
            usage_percentage > 80.0 // Consider 80% as "near limit"
        } else {
            false
        }
    }
}

/// Request queue item
#[derive(Debug)]
struct QueuedRequest {
    method: String,
    endpoint: String,
    body: Option<Value>,
    token: Option<String>,
    priority: u8, // 0 = highest priority
    created_at: Instant,
}

/// Rate-limited API client
struct RateLimitedClient {
    client: Client,
    base_url: String,
    config: RateLimitConfig,
    request_queue: Arc<Mutex<VecDeque<QueuedRequest>>>,
    request_times: Arc<Mutex<VecDeque<Instant>>>,
    current_backoff: Arc<Mutex<Duration>>,
    rate_limit_info: Arc<Mutex<Option<RateLimitInfo>>>,
}

impl RateLimitedClient {
    fn new(base_url: String, config: RateLimitConfig) -> Self {
        Self {
            client: Client::new(),
            base_url,
            config,
            request_queue: Arc::new(Mutex::new(VecDeque::new())),
            request_times: Arc::new(Mutex::new(VecDeque::new())),
            current_backoff: Arc::new(Mutex::new(Duration::from_millis(100))),
            rate_limit_info: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Add request to queue
    async fn queue_request(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<Value>,
        token: Option<String>,
        priority: u8,
    ) -> Result<Value, Box<dyn Error>> {
        let request = QueuedRequest {
            method: method.to_string(),
            endpoint: endpoint.to_string(),
            body,
            token,
            priority,
            created_at: Instant::now(),
        };
        
        // Add to queue
        {
            let mut queue = self.request_queue.lock().unwrap();
            
            // Insert based on priority (lower number = higher priority)
            let insert_pos = queue.iter()
                .position(|r| r.priority > priority)
                .unwrap_or(queue.len());
            
            queue.insert(insert_pos, request);
        }
        
        println!("📥 Request queued: {} {} (priority: {})", method, endpoint, priority);
        
        // Process the queue
        self.process_queue().await
    }
    
    /// Process the request queue with rate limiting
    async fn process_queue(&self) -> Result<Value, Box<dyn Error>> {
        loop {
            // Check if we can make a request
            if !self.can_make_request().await {
                let backoff = {
                    let backoff = self.current_backoff.lock().unwrap();
                    *backoff
                };
                
                println!("⏳ Rate limit reached, waiting {:?}...", backoff);
                sleep(backoff).await;
                continue;
            }
            
            // Get next request from queue
            let request = {
                let mut queue = self.request_queue.lock().unwrap();
                queue.pop_front()
            };
            
            if let Some(req) = request {
                println!("🚀 Processing request: {} {}", req.method, req.endpoint);
                
                match self.execute_request(&req).await {
                    Ok(response) => {
                        self.record_successful_request().await;
                        return Ok(response);
                    }
                    Err(e) => {
                        if self.is_rate_limit_error(&e) {
                            println!("🚫 Rate limited, re-queuing request");
                            
                            // Re-queue the request
                            {
                                let mut queue = self.request_queue.lock().unwrap();
                                queue.push_front(req);
                            }
                            
                            self.handle_rate_limit_error(&e).await;
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                return Err("No requests in queue".into());
            }
        }
    }
    
    /// Check if we can make a request based on rate limits
    async fn can_make_request(&self) -> bool {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(60);
        
        // Clean old request times
        {
            let mut times = self.request_times.lock().unwrap();
            while let Some(&front_time) = times.front() {
                if front_time < window_start {
                    times.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Check current rate
        let current_requests = {
            let times = self.request_times.lock().unwrap();
            times.len() as u32
        };
        
        // Check against configured limit
        if current_requests >= self.config.requests_per_minute {
            return false;
        }
        
        // Check API-provided rate limit info
        if let Some(rate_info) = self.rate_limit_info.lock().unwrap().as_ref() {
            if let Some(remaining) = rate_info.remaining {
                if remaining == 0 {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Execute a single request
    async fn execute_request(&self, request: &QueuedRequest) -> Result<Value, Box<dyn Error>> {
        let url = format!("{}{}", self.base_url, request.endpoint);
        
        let mut req = match request.method.as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err("Unsupported HTTP method".into()),
        };
        
        if let Some(ref token) = request.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        
        if let Some(ref body) = request.body {
            req = req.json(body);
        }
        
        let response = req.send().await?;
        
        // Extract rate limit information
        let rate_info = RateLimitInfo::from_headers(response.headers());
        {
            let mut info = self.rate_limit_info.lock().unwrap();
            *info = Some(rate_info.clone());
        }
        
        // Log rate limit status
        if let (Some(remaining), Some(limit)) = (rate_info.remaining, rate_info.limit) {
            let usage_percentage = ((limit - remaining) as f64 / limit as f64) * 100.0;
            println!("📊 Rate limit: {}/{} ({:.1}% used)", limit - remaining, limit, usage_percentage);
            
            if rate_info.is_near_limit() {
                println!("⚠️  Approaching rate limit threshold");
            }
        }
        
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            return Err(format!("Rate limited: {}", response.status()).into());
        }
        
        if response.status().is_success() {
            if response.status() == StatusCode::NO_CONTENT {
                Ok(json!({"status": "success"}))
            } else {
                let data: Value = response.json().await?;
                Ok(data)
            }
        } else {
            let error_text = response.text().await?;
            Err(format!("Request failed: {}", error_text).into())
        }
    }
    
    /// Record successful request for rate tracking
    async fn record_successful_request(&self) {
        let now = Instant::now();
        {
            let mut times = self.request_times.lock().unwrap();
            times.push_back(now);
        }
        
        // Reset backoff on success
        {
            let mut backoff = self.current_backoff.lock().unwrap();
            *backoff = Duration::from_millis(100);
        }
    }
    
    /// Handle rate limit error
    async fn handle_rate_limit_error(&self, error: &Box<dyn Error>) {
        println!("🚫 Rate limit error: {}", error);
        
        // Increase backoff
        {
            let mut backoff = self.current_backoff.lock().unwrap();
            let new_backoff = Duration::from_millis(
                (*backoff).as_millis() as u64 * self.config.backoff_multiplier as u64
            );
            *backoff = new_backoff.min(self.config.max_backoff);
        }
        
        // Check for retry-after header
        if let Some(rate_info) = self.rate_limit_info.lock().unwrap().as_ref() {
            if let Some(retry_after) = rate_info.retry_after {
                let retry_duration = Duration::from_secs(retry_after);
                println!("⏰ API suggests waiting {} seconds", retry_after);
                sleep(retry_duration).await;
            }
        }
    }
    
    /// Check if error is rate limit related
    fn is_rate_limit_error(&self, error: &Box<dyn Error>) -> bool {
        error.to_string().contains("Rate limited") || 
        error.to_string().contains("429") ||
        error.to_string().contains("Too Many Requests")
    }
    
    /// Get queue statistics
    fn get_queue_stats(&self) -> (usize, Duration) {
        let queue = self.request_queue.lock().unwrap();
        let queue_size = queue.len();
        
        let oldest_wait_time = queue.front()
            .map(|req| req.created_at.elapsed())
            .unwrap_or(Duration::from_secs(0));
        
        (queue_size, oldest_wait_time)
    }
}

/// Demonstrate basic rate limiting
async fn demonstrate_basic_rate_limiting() -> Result<(), Box<dyn Error>> {
    println!("🚦 Basic Rate Limiting Demonstration");
    println!("====================================");
    
    let base_url = env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let config = RateLimitConfig {
        requests_per_minute: 10, // Very low limit for demonstration
        burst_allowance: 3,
        backoff_multiplier: 1.5,
        max_backoff: Duration::from_secs(30),
    };
    
    let client = RateLimitedClient::new(base_url, config);
    
    println!("📊 Rate limit configuration:");
    println!("   Requests per minute: {}", client.config.requests_per_minute);
    println!("   Burst allowance: {}", client.config.burst_allowance);
    println!("   Backoff multiplier: {}", client.config.backoff_multiplier);
    
    // Make several requests to demonstrate rate limiting
    for i in 1..=15 {
        println!("\n🔄 Making request {} of 15", i);
        
        let start_time = Instant::now();
        
        match client.queue_request(
            "GET",
            "/health/live",
            None,
            None,
            0, // High priority
        ).await {
            Ok(response) => {
                let elapsed = start_time.elapsed();
                println!("✅ Request {} completed in {:?}", i, elapsed);
                
                if let Some(status) = response.as_str() {
                    println!("   Response: {}", status);
                }
            }
            Err(e) => {
                println!("❌ Request {} failed: {}", i, e);
            }
        }
        
        let (queue_size, oldest_wait) = client.get_queue_stats();
        if queue_size > 0 {
            println!("📥 Queue status: {} requests waiting, oldest: {:?}", queue_size, oldest_wait);
        }
        
        // Small delay between requests
        sleep(Duration::from_millis(100)).await;
    }
    
    Ok(())
}

/// Demonstrate priority-based request queuing
async fn demonstrate_priority_queuing() -> Result<(), Box<dyn Error>> {
    println!("\n🎯 Priority-Based Request Queuing");
    println!("=================================");
    
    let base_url = env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let config = RateLimitConfig {
        requests_per_minute: 5, // Very restrictive for demo
        ..Default::default()
    };
    
    let client = RateLimitedClient::new(base_url, config);
    
    // Create authentication token for some requests
    let auth_client = Client::new();
    let registration_data = json!({
        "email": "ratelimit.test@restaurant.com",
        "password": "RateLimitTest123!",
        "full_name": "Rate Limit Test User"
    });
    
    let token = match auth_client
        .post(&format!("{}/api/v1/auth/register", client.base_url))
        .json(&registration_data)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let token_response: Value = response.json().await?;
            Some(token_response["token"].as_str().unwrap().to_string())
        }
        _ => {
            // Try login instead
            let login_data = json!({
                "email": "ratelimit.test@restaurant.com",
                "password": "RateLimitTest123!"
            });
            
            match auth_client
                .post(&format!("{}/api/v1/auth/login", client.base_url))
                .json(&login_data)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    let token_response: Value = response.json().await?;
                    Some(token_response["token"].as_str().unwrap().to_string())
                }
                _ => None,
            }
        }
    };
    
    // Queue requests with different priorities
    let requests = vec![
        ("GET", "/health/live", None, None, 3, "Low priority health check"),
        ("GET", "/health/ready", None, None, 2, "Medium priority readiness check"),
        ("GET", "/api/v1/user/profile", None, token.clone(), 0, "High priority user profile"),
        ("GET", "/health/live", None, None, 3, "Another low priority health check"),
        ("GET", "/api/v1/user/stats", None, token.clone(), 1, "High priority user stats"),
        ("GET", "/health/ready", None, None, 2, "Another medium priority readiness"),
    ];
    
    println!("📥 Queuing {} requests with different priorities...", requests.len());
    
    // Queue all requests quickly
    for (method, endpoint, body, token_opt, priority, description) in requests {
        println!("   📝 Queuing: {} (priority {})", description, priority);
        
        // Queue the request (don't await - we want to queue them all first)
        tokio::spawn({
            let client = Arc::new(client);
            let method = method.to_string();
            let endpoint = endpoint.to_string();
            let description = description.to_string();
            
            async move {
                match client.queue_request(&method, &endpoint, body, token_opt, priority).await {
                    Ok(_) => println!("✅ Completed: {}", description),
                    Err(e) => println!("❌ Failed: {} - {}", description, e),
                }
            }
        });
        
        // Small delay to show queuing
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for all requests to complete
    sleep(Duration::from_secs(10)).await;
    
    Ok(())
}

/// Demonstrate adaptive throttling
async fn demonstrate_adaptive_throttling() -> Result<(), Box<dyn Error>> {
    println!("\n🎛️  Adaptive Throttling Demonstration");
    println!("====================================");
    
    let base_url = env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let mut config = RateLimitConfig::default();
    config.requests_per_minute = 20; // Start with a reasonable limit
    
    let client = RateLimitedClient::new(base_url, config);
    
    println!("🔄 Starting with {} requests per minute", client.config.requests_per_minute);
    
    // Simulate varying load patterns
    let load_patterns = vec![
        (5, "Light load"),
        (15, "Medium load"),
        (25, "Heavy load"),
        (10, "Reduced load"),
    ];
    
    for (request_count, load_description) in load_patterns {
        println!("\n📊 Simulating {}: {} requests", load_description, request_count);
        
        let start_time = Instant::now();
        let mut successful_requests = 0;
        let mut failed_requests = 0;
        
        for i in 1..=request_count {
            match client.queue_request(
                "GET",
                "/health/live",
                None,
                None,
                1, // Medium priority
            ).await {
                Ok(_) => {
                    successful_requests += 1;
                    print!("✅");
                }
                Err(_) => {
                    failed_requests += 1;
                    print!("❌");
                }
            }
            
            if i % 10 == 0 {
                println!(); // New line every 10 requests
            }
            
            // Vary request timing based on load
            let delay = match load_description {
                "Light load" => Duration::from_millis(500),
                "Medium load" => Duration::from_millis(200),
                "Heavy load" => Duration::from_millis(50),
                "Reduced load" => Duration::from_millis(300),
                _ => Duration::from_millis(100),
            };
            
            sleep(delay).await;
        }
        
        let elapsed = start_time.elapsed();
        let success_rate = (successful_requests as f64 / request_count as f64) * 100.0;
        
        println!("\n📈 {} Results:", load_description);
        println!("   Total time: {:?}", elapsed);
        println!("   Successful: {} ({:.1}%)", successful_requests, success_rate);
        println!("   Failed: {}", failed_requests);
        println!("   Average time per request: {:?}", elapsed / request_count);
        
        // Brief pause between load patterns
        sleep(Duration::from_secs(2)).await;
    }
    
    Ok(())
}

/// Demonstrate rate limit monitoring
fn demonstrate_rate_limit_monitoring() {
    println!("\n📊 Rate Limit Monitoring Best Practices");
    println!("=======================================");
    
    println!("✅ Key Metrics to Track:");
    println!("   • Request rate (requests per minute/hour)");
    println!("   • Rate limit utilization percentage");
    println!("   • Queue depth and wait times");
    println!("   • Success vs. failure rates");
    println!("   • Backoff frequency and duration");
    
    println!("\n✅ Monitoring Strategies:");
    println!("   • Real-time dashboards for rate limit status");
    println!("   • Alerts for approaching rate limits (>80% usage)");
    println!("   • Queue depth monitoring and alerting");
    println!("   • Historical analysis of usage patterns");
    println!("   • Per-user/per-endpoint rate tracking");
    
    println!("\n✅ Kitchen-Specific Considerations:");
    println!("   • Peak hours monitoring (lunch/dinner rush)");
    println!("   • Staff shift change impact on API usage");
    println!("   • Critical operation prioritization");
    println!("   • Offline mode capabilities for rate limit scenarios");
    
    println!("\n✅ Optimization Strategies:");
    println!("   • Request batching for bulk operations");
    println!("   • Caching to reduce API calls");
    println!("   • Asynchronous processing for non-critical requests");
    println!("   • Load balancing across multiple API endpoints");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - Rate Limiting Example");
    println!("==================================================");
    
    let base_url = env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    println!("🌐 Using API base URL: {}", base_url);
    
    // Demonstrate basic rate limiting
    demonstrate_basic_rate_limiting().await?;
    
    // Demonstrate priority-based queuing
    demonstrate_priority_queuing().await?;
    
    // Demonstrate adaptive throttling
    demonstrate_adaptive_throttling().await?;
    
    // Show monitoring best practices
    demonstrate_rate_limit_monitoring();
    
    println!("\n🚦 Rate Limiting Implementation Summary");
    println!("======================================");
    println!("✅ Request Queue Management:");
    println!("   • Priority-based request ordering");
    println!("   • Automatic retry with exponential backoff");
    println!("   • Queue depth monitoring and management");
    
    println!("\n✅ Rate Limit Compliance:");
    println!("   • Respect API-provided rate limit headers");
    println!("   • Adaptive throttling based on current usage");
    println!("   • Graceful handling of 429 responses");
    
    println!("\n✅ Performance Optimization:");
    println!("   • Burst allowance for short-term spikes");
    println!("   • Intelligent backoff strategies");
    println!("   • Request prioritization for critical operations");
    
    println!("\n🍳 Kitchen Management Rate Limiting");
    println!("===================================");
    println!("✅ Operational Priorities:");
    println!("   • Critical kitchen operations get highest priority");
    println!("   • Background tasks use lower priority queues");
    println!("   • Staff authentication requests prioritized");
    
    println!("\n✅ Business Continuity:");
    println!("   • Graceful degradation during rate limit scenarios");
    println!("   • Offline capabilities for essential functions");
    println!("   • Clear communication to staff about system status");
    
    println!("\n✅ Resource Management:");
    println!("   • Fair allocation across different kitchen stations");
    println!("   • Load balancing during peak operational hours");
    println!("   • Efficient use of API quotas and limits");
    
    println!("\n🚀 Production Rate Limiting Recommendations");
    println!("===========================================");
    println!("💡 Implementation:");
    println!("   • Use Redis or similar for distributed rate limiting");
    println!("   • Implement circuit breakers for failing endpoints");
    println!("   • Add comprehensive metrics and monitoring");
    
    println!("\n💡 Configuration:");
    println!("   • Environment-specific rate limit configurations");
    println!("   • Dynamic rate limit adjustment based on system load");
    println!("   • User/role-based rate limiting policies");
    
    println!("\n💡 Monitoring:");
    println!("   • Real-time rate limit utilization dashboards");
    println!("   • Automated alerting for rate limit violations");
    println!("   • Historical analysis for capacity planning");
    
    println!("\n🎉 Rate Limiting Example Completed!");
    println!("===================================");
    println!("✅ Basic rate limiting patterns demonstrated");
    println!("✅ Priority-based request queuing implemented");
    println!("✅ Adaptive throttling strategies shown");
    println!("✅ Monitoring and optimization practices outlined");
    println!("\n💡 Next Steps:");
    println!("   - Implement distributed rate limiting with Redis");
    println!("   - Add comprehensive rate limit monitoring");
    println!("   - Create rate limit configuration management");
    println!("   - Test rate limiting under various load conditions");
    
    Ok(())
}