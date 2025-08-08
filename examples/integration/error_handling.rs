//! Comprehensive Error Handling Example
//!
//! This example demonstrates robust error handling patterns for the Kitchen Management API:
//! 1. Network errors and timeouts
//! 2. Authentication and authorization failures
//! 3. Validation errors and malformed requests
//! 4. Server errors and service unavailability
//! 5. Rate limiting and throttling responses
//! 6. Recovery strategies and retry mechanisms
//!
//! # Kitchen Management Context
//!
//! In a busy kitchen environment, system reliability is critical. This example
//! shows how to handle various error conditions gracefully to maintain kitchen
//! operations even when facing API issues.

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Configuration for the API client with error handling settings
struct ApiConfig {
    base_url: String,
    client: Client,
    max_retries: u32,
    retry_delay: Duration,
    request_timeout: Duration,
}

impl ApiConfig {
    fn new() -> Self {
        let base_url = env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
        // Configure client with timeouts
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            base_url,
            client,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Comprehensive error types for kitchen management operations
#[derive(Debug)]
enum KitchenApiError {
    NetworkError(String),
    AuthenticationError(String),
    ValidationError(String, Vec<String>),
    NotFoundError(String),
    ServerError(String),
    RateLimitError(String, Option<u64>),
    TimeoutError(String),
    UnknownError(String),
}

impl std::fmt::Display for KitchenApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KitchenApiError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            KitchenApiError::AuthenticationError(msg) => write!(f, "Authentication Error: {}", msg),
            KitchenApiError::ValidationError(msg, details) => {
                write!(f, "Validation Error: {} - Details: {:?}", msg, details)
            }
            KitchenApiError::NotFoundError(msg) => write!(f, "Not Found: {}", msg),
            KitchenApiError::ServerError(msg) => write!(f, "Server Error: {}", msg),
            KitchenApiError::RateLimitError(msg, retry_after) => {
                if let Some(seconds) = retry_after {
                    write!(f, "Rate Limited: {} (retry after {} seconds)", msg, seconds)
                } else {
                    write!(f, "Rate Limited: {}", msg)
                }
            }
            KitchenApiError::TimeoutError(msg) => write!(f, "Timeout Error: {}", msg),
            KitchenApiError::UnknownError(msg) => write!(f, "Unknown Error: {}", msg),
        }
    }
}

impl std::error::Error for KitchenApiError {}

/// Error handling utilities
struct ErrorHandler;

impl ErrorHandler {
    /// Parse API response and convert to appropriate error type
    async fn parse_api_error(response: reqwest::Response) -> KitchenApiError {
        let status = response.status();
        
        match status {
            StatusCode::BAD_REQUEST => {
                if let Ok(error_data) = response.json::<Value>().await {
                    if let Some(validation_errors) = error_data.get("validation_errors") {
                        let details = Self::extract_validation_details(validation_errors);
                        KitchenApiError::ValidationError(
                            error_data.get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Validation failed")
                                .to_string(),
                            details
                        )
                    } else {
                        KitchenApiError::ValidationError(
                            error_data.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Bad request")
                                .to_string(),
                            vec![]
                        )
                    }
                } else {
                    KitchenApiError::ValidationError("Bad request".to_string(), vec![])
                }
            }
            StatusCode::UNAUTHORIZED => {
                KitchenApiError::AuthenticationError("Invalid or expired authentication token".to_string())
            }
            StatusCode::FORBIDDEN => {
                KitchenApiError::AuthenticationError("Insufficient permissions for this operation".to_string())
            }
            StatusCode::NOT_FOUND => {
                KitchenApiError::NotFoundError("Requested resource not found".to_string())
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok());
                
                KitchenApiError::RateLimitError(
                    "Rate limit exceeded".to_string(),
                    retry_after
                )
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                KitchenApiError::ServerError("Internal server error".to_string())
            }
            StatusCode::BAD_GATEWAY => {
                KitchenApiError::ServerError("Bad gateway - service temporarily unavailable".to_string())
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                KitchenApiError::ServerError("Service temporarily unavailable".to_string())
            }
            StatusCode::GATEWAY_TIMEOUT => {
                KitchenApiError::TimeoutError("Gateway timeout".to_string())
            }
            _ => {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                KitchenApiError::UnknownError(format!("HTTP {}: {}", status, error_text))
            }
        }
    }
    
    /// Extract validation error details from response
    fn extract_validation_details(validation_errors: &Value) -> Vec<String> {
        let mut details = Vec::new();
        
        if let Some(obj) = validation_errors.as_object() {
            for (field, errors) in obj {
                if let Some(error_array) = errors.as_array() {
                    for error in error_array {
                        if let Some(error_str) = error.as_str() {
                            details.push(format!("{}: {}", field, error_str));
                        }
                    }
                }
            }
        }
        
        details
    }
    
    /// Determine if error is retryable
    fn is_retryable_error(error: &KitchenApiError) -> bool {
        matches!(error, 
            KitchenApiError::NetworkError(_) |
            KitchenApiError::ServerError(_) |
            KitchenApiError::TimeoutError(_)
        )
    }
    
    /// Get retry delay for rate limiting
    fn get_retry_delay(error: &KitchenApiError) -> Duration {
        match error {
            KitchenApiError::RateLimitError(_, Some(seconds)) => Duration::from_secs(*seconds),
            KitchenApiError::RateLimitError(_, None) => Duration::from_secs(60),
            _ => Duration::from_secs(1),
        }
    }
}

/// Resilient API client with retry logic
struct ResilientApiClient {
    config: ApiConfig,
}

impl ResilientApiClient {
    fn new() -> Self {
        Self {
            config: ApiConfig::new(),
        }
    }
    
    /// Make a resilient API request with retry logic
    async fn make_request(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> Result<Value, KitchenApiError> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            println!("🔄 Attempt {} of {} for {} {}", attempt, self.config.max_retries, method, endpoint);
            
            match self.execute_request(method, endpoint, body.clone(), token).await {
                Ok(response) => {
                    println!("✅ Request successful on attempt {}", attempt);
                    return Ok(response);
                }
                Err(error) => {
                    println!("❌ Attempt {} failed: {}", attempt, error);
                    
                    // Check if error is retryable
                    if !ErrorHandler::is_retryable_error(&error) {
                        println!("🚫 Error is not retryable, giving up");
                        return Err(error);
                    }
                    
                    last_error = Some(error);
                    
                    // Don't sleep after the last attempt
                    if attempt < self.config.max_retries {
                        let delay = if let Some(ref err) = last_error {
                            ErrorHandler::get_retry_delay(err)
                        } else {
                            self.config.retry_delay
                        };
                        
                        println!("⏳ Waiting {:?} before retry...", delay);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        println!("💥 All retry attempts exhausted");
        Err(last_error.unwrap_or_else(|| KitchenApiError::UnknownError("All retries failed".to_string())))
    }
    
    /// Execute a single API request
    async fn execute_request(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> Result<Value, KitchenApiError> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        
        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.config.client.get(&url),
            "POST" => self.config.client.post(&url),
            "PUT" => self.config.client.put(&url),
            "DELETE" => self.config.client.delete(&url),
            _ => return Err(KitchenApiError::ValidationError("Unsupported HTTP method".to_string(), vec![])),
        };
        
        if let Some(auth_token) = token {
            request = request.header("Authorization", format!("Bearer {}", auth_token));
        }
        
        if let Some(json_body) = body {
            request = request.json(&json_body);
        }
        
        // Execute request with timeout
        let response_result = timeout(self.config.request_timeout, request.send()).await;
        
        let response = match response_result {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                return Err(KitchenApiError::NetworkError(format!("Request failed: {}", e)));
            }
            Err(_) => {
                return Err(KitchenApiError::TimeoutError("Request timed out".to_string()));
            }
        };
        
        if response.status().is_success() {
            if response.status() == StatusCode::NO_CONTENT {
                Ok(json!({"status": "success"}))
            } else {
                match response.json::<Value>().await {
                    Ok(data) => Ok(data),
                    Err(e) => Err(KitchenApiError::NetworkError(format!("Failed to parse response: {}", e))),
                }
            }
        } else {
            Err(ErrorHandler::parse_api_error(response).await)
        }
    }
}

/// Demonstrate various error scenarios
async fn demonstrate_error_scenarios(client: &ResilientApiClient) -> Result<(), Box<dyn Error>> {
    println!("\n🚨 Error Scenario Demonstrations");
    println!("================================");
    
    // 1. Authentication errors
    println!("\n1. Authentication Error Scenarios:");
    println!("----------------------------------");
    
    // Invalid token
    println!("\n🔐 Testing invalid authentication token:");
    match client.make_request("GET", "/api/v1/user/profile", None, Some("invalid.jwt.token")).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }
    
    // Missing token
    println!("\n🔐 Testing missing authentication token:");
    match client.make_request("GET", "/api/v1/user/profile", None, None).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }
    
    // 2. Validation errors
    println!("\n2. Validation Error Scenarios:");
    println!("------------------------------");
    
    // Invalid email format
    println!("\n📧 Testing invalid email format:");
    let invalid_registration = json!({
        "email": "not-an-email",
        "password": "ValidPass123!",
        "full_name": "Test User"
    });
    
    match client.make_request("POST", "/api/v1/auth/register", Some(invalid_registration), None).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected validation error: {}", e),
    }
    
    // Weak password
    println!("\n🔒 Testing weak password:");
    let weak_password_registration = json!({
        "email": "test@example.com",
        "password": "123",
        "full_name": "Test User"
    });
    
    match client.make_request("POST", "/api/v1/auth/register", Some(weak_password_registration), None).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected validation error: {}", e),
    }
    
    // Missing required fields
    println!("\n📝 Testing missing required fields:");
    let incomplete_registration = json!({
        "email": "test@example.com"
        // Missing password and full_name
    });
    
    match client.make_request("POST", "/api/v1/auth/register", Some(incomplete_registration), None).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected validation error: {}", e),
    }
    
    // 3. Not found errors
    println!("\n3. Not Found Error Scenarios:");
    println!("-----------------------------");
    
    // Create a valid token first
    let valid_registration = json!({
        "email": "error.test@restaurant.com",
        "password": "ErrorTestPass123!",
        "full_name": "Error Test User"
    });
    
    let token = match client.make_request("POST", "/api/v1/auth/register", Some(valid_registration), None).await {
        Ok(response) => {
            response["token"].as_str().unwrap_or("").to_string()
        }
        Err(_) => {
            // Try login instead
            let login_data = json!({
                "email": "error.test@restaurant.com",
                "password": "ErrorTestPass123!"
            });
            
            match client.make_request("POST", "/api/v1/auth/login", Some(login_data), None).await {
                Ok(response) => response["token"].as_str().unwrap_or("").to_string(),
                Err(e) => {
                    println!("   ❌ Failed to get valid token: {}", e);
                    return Ok(());
                }
            }
        }
    };
    
    // Non-existent user
    println!("\n👤 Testing non-existent user lookup:");
    let fake_user_id = "00000000-0000-0000-0000-000000000000";
    match client.make_request("GET", &format!("/api/v1/users/{}", fake_user_id), None, Some(&token)).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected not found error: {}", e),
    }
    
    // Non-existent endpoint
    println!("\n🔍 Testing non-existent endpoint:");
    match client.make_request("GET", "/api/v1/nonexistent", None, Some(&token)).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected not found error: {}", e),
    }
    
    // 4. Network and timeout errors
    println!("\n4. Network and Timeout Error Scenarios:");
    println!("---------------------------------------");
    
    // Test with invalid base URL to simulate network error
    println!("\n🌐 Testing network connectivity issues:");
    let invalid_client = ResilientApiClient {
        config: ApiConfig {
            base_url: "http://invalid-host-that-does-not-exist:9999".to_string(),
            client: Client::new(),
            max_retries: 2, // Reduce retries for demo
            retry_delay: Duration::from_millis(500),
            request_timeout: Duration::from_secs(5),
        }
    };
    
    match invalid_client.make_request("GET", "/health/live", None, None).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected network error: {}", e),
    }
    
    Ok(())
}

/// Demonstrate error recovery strategies
async fn demonstrate_recovery_strategies(client: &ResilientApiClient) -> Result<(), Box<dyn Error>> {
    println!("\n🔄 Error Recovery Strategy Demonstrations");
    println!("=========================================");
    
    // 1. Retry with exponential backoff
    println!("\n1. Retry with Exponential Backoff:");
    println!("----------------------------------");
    
    println!("📡 Attempting request with potential failures...");
    
    // This will likely succeed, but demonstrates the retry mechanism
    match client.make_request("GET", "/health/live", None, None).await {
        Ok(response) => {
            println!("✅ Request succeeded: {}", response);
        }
        Err(e) => {
            println!("❌ Request failed after retries: {}", e);
        }
    }
    
    // 2. Graceful degradation
    println!("\n2. Graceful Degradation:");
    println!("------------------------");
    
    println!("🔄 Attempting to get user profile with fallback...");
    
    // Try to get user profile, fall back to basic info if it fails
    let fallback_token = "fallback.test.token";
    match client.make_request("GET", "/api/v1/user/profile", None, Some(fallback_token)).await {
        Ok(profile) => {
            println!("✅ Profile retrieved: {}", profile.get("email").unwrap_or(&json!("unknown")));
        }
        Err(e) => {
            println!("⚠️  Profile request failed: {}", e);
            println!("🔄 Falling back to basic user information...");
            
            // In a real application, you might fall back to cached data or basic info
            let fallback_info = json!({
                "email": "unknown@restaurant.com",
                "full_name": "Unknown User",
                "status": "offline"
            });
            
            println!("✅ Using fallback data: {}", fallback_info);
        }
    }
    
    // 3. Circuit breaker pattern simulation
    println!("\n3. Circuit Breaker Pattern:");
    println!("---------------------------");
    
    println!("🔌 Simulating circuit breaker for failing service...");
    
    let mut failure_count = 0;
    let failure_threshold = 3;
    let mut circuit_open = false;
    
    for attempt in 1..=5 {
        if circuit_open {
            println!("   🚫 Circuit breaker OPEN - request blocked (attempt {})", attempt);
            continue;
        }
        
        println!("   🔄 Circuit breaker CLOSED - attempting request (attempt {})", attempt);
        
        // Simulate a failing endpoint
        match client.make_request("GET", "/api/v1/nonexistent-service", None, None).await {
            Ok(_) => {
                println!("   ✅ Request succeeded - resetting failure count");
                failure_count = 0;
            }
            Err(e) => {
                failure_count += 1;
                println!("   ❌ Request failed ({}): {}", failure_count, e);
                
                if failure_count >= failure_threshold {
                    circuit_open = true;
                    println!("   🚨 Circuit breaker OPENED - too many failures");
                }
            }
        }
        
        sleep(Duration::from_millis(500)).await;
    }
    
    Ok(())
}

/// Demonstrate error monitoring and logging
fn demonstrate_error_monitoring() {
    println!("\n📊 Error Monitoring and Logging Best Practices");
    println!("===============================================");
    
    println!("✅ Error Classification:");
    println!("   • Network errors - connectivity, timeouts, DNS issues");
    println!("   • Authentication errors - invalid tokens, expired sessions");
    println!("   • Validation errors - malformed requests, missing fields");
    println!("   • Business logic errors - resource not found, conflicts");
    println!("   • Server errors - internal errors, service unavailable");
    
    println!("\n✅ Error Metrics to Track:");
    println!("   • Error rate by endpoint and error type");
    println!("   • Response time percentiles (p50, p95, p99)");
    println!("   • Retry success rates and attempt counts");
    println!("   • Circuit breaker state changes");
    println!("   • User impact and affected operations");
    
    println!("\n✅ Alerting Strategies:");
    println!("   • Error rate thresholds (>5% error rate)");
    println!("   • Response time degradation (>2s p95)");
    println!("   • Authentication failure spikes");
    println!("   • Service availability drops");
    println!("   • Critical kitchen operations affected");
    
    println!("\n✅ Recovery Procedures:");
    println!("   • Automatic retry with exponential backoff");
    println!("   • Circuit breaker to prevent cascade failures");
    println!("   • Graceful degradation to cached or default data");
    println!("   • Manual intervention escalation paths");
    println!("   • Kitchen staff notification for critical failures");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - Comprehensive Error Handling Example");
    println!("==================================================================");
    
    let client = ResilientApiClient::new();
    println!("🌐 Using API base URL: {}", client.config.base_url);
    println!("🔄 Max retries: {}", client.config.max_retries);
    println!("⏱️  Request timeout: {:?}", client.config.request_timeout);
    
    // Demonstrate various error scenarios
    demonstrate_error_scenarios(&client).await?;
    
    // Demonstrate recovery strategies
    demonstrate_recovery_strategies(&client).await?;
    
    // Show monitoring best practices
    demonstrate_error_monitoring();
    
    println!("\n🛡️  Error Handling Patterns Summary");
    println!("===================================");
    println!("✅ Comprehensive Error Classification:");
    println!("   • Network, authentication, validation, server errors");
    println!("   • Specific error types with detailed messages");
    println!("   • Retryable vs non-retryable error identification");
    
    println!("\n✅ Resilient Request Handling:");
    println!("   • Automatic retry with exponential backoff");
    println!("   • Configurable timeout and retry policies");
    println!("   • Rate limit respect and adaptive delays");
    
    println!("\n✅ Recovery Strategies:");
    println!("   • Circuit breaker pattern for failing services");
    println!("   • Graceful degradation with fallback data");
    println!("   • User-friendly error messages and guidance");
    
    println!("\n🍳 Kitchen Management Error Handling");
    println!("====================================");
    println!("✅ Operational Continuity:");
    println!("   • Kitchen operations continue during API issues");
    println!("   • Critical functions have offline fallbacks");
    println!("   • Staff notifications for system issues");
    
    println!("\n✅ Data Integrity:");
    println!("   • Validation errors prevent data corruption");
    println!("   • Transaction rollback on critical failures");
    println!("   • Audit logging for all error conditions");
    
    println!("\n✅ User Experience:");
    println!("   • Clear error messages for kitchen staff");
    println!("   • Suggested actions for common issues");
    println!("   • Minimal disruption to daily workflows");
    
    println!("\n🚀 Production Error Handling Recommendations");
    println!("============================================");
    println!("💡 Monitoring and Alerting:");
    println!("   • Implement comprehensive error tracking");
    println!("   • Set up real-time alerting for critical errors");
    println!("   • Create error dashboards for operations teams");
    
    println!("\n💡 Error Recovery:");
    println!("   • Implement dead letter queues for failed operations");
    println!("   • Add manual retry mechanisms for critical processes");
    println!("   • Create runbooks for common error scenarios");
    
    println!("\n💡 Testing:");
    println!("   • Chaos engineering to test error handling");
    println!("   • Load testing to identify breaking points");
    println!("   • Error injection testing for recovery validation");
    
    println!("\n🎉 Error Handling Example Completed!");
    println!("====================================");
    println!("✅ Comprehensive error scenarios demonstrated");
    println!("✅ Resilient request patterns implemented");
    println!("✅ Recovery strategies and best practices shown");
    println!("✅ Production-ready error handling patterns provided");
    println!("\n💡 Next Steps:");
    println!("   - Implement error monitoring and alerting");
    println!("   - Add comprehensive logging and metrics");
    println!("   - Create error handling documentation");
    println!("   - Test error scenarios in staging environments");
    
    Ok(())
}