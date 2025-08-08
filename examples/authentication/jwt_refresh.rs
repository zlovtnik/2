//! JWT Token Refresh Example
//!
//! This example demonstrates JWT token management and refresh patterns:
//! 1. Token validation and expiration checking
//! 2. Token refresh workflow (placeholder implementation)
//! 3. Automatic token renewal strategies
//! 4. Token storage and security best practices
//!
//! # Kitchen Management Context
//!
//! Kitchen staff often work long shifts (8-12 hours) and need continuous access
//! to the system without frequent re-authentication. This example shows how to
//! manage token lifecycle for uninterrupted kitchen operations.

use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use base64::{Engine as _, engine::general_purpose};

/// Configuration for the API client with token management
struct ApiConfig {
    base_url: String,
    client: Client,
}

impl ApiConfig {
    fn new() -> Self {
        let base_url = env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
        Self {
            base_url,
            client: Client::new(),
        }
    }
}

/// JWT token information extracted from the token payload
#[derive(Debug)]
struct TokenInfo {
    user_id: String,
    expires_at: u64,
    issued_at: u64,
}

impl TokenInfo {
    /// Parse JWT token to extract information (simplified parsing)
    fn from_jwt(token: &str) -> Result<Self, Box<dyn Error>> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".into());
        }
        
        // Decode the payload (second part)
        let payload_b64 = parts[1];
        // Add padding if needed
        let padded = match payload_b64.len() % 4 {
            0 => payload_b64.to_string(),
            n => format!("{}{}", payload_b64, "=".repeat(4 - n)),
        };
        
        let payload_bytes = general_purpose::STANDARD.decode(padded)?;
        let payload_str = String::from_utf8(payload_bytes)?;
        let payload: Value = serde_json::from_str(&payload_str)?;
        
        let user_id = payload["sub"]
            .as_str()
            .ok_or("Missing user ID in token")?
            .to_string();
        
        let expires_at = payload["exp"]
            .as_u64()
            .ok_or("Missing expiration in token")?;
        
        // Calculate issued_at as exp - 24 hours (since tokens are valid for 24 hours)
        let issued_at = expires_at.saturating_sub(24 * 60 * 60);
        
        Ok(TokenInfo {
            user_id,
            expires_at,
            issued_at,
        })
    }
    
    /// Check if token is expired
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= self.expires_at
    }
    
    /// Check if token expires within the given duration
    fn expires_within(&self, duration: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let threshold = now + duration.as_secs();
        self.expires_at <= threshold
    }
    
    /// Get remaining time until expiration
    fn time_until_expiration(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if self.expires_at > now {
            Duration::from_secs(self.expires_at - now)
        } else {
            Duration::from_secs(0)
        }
    }
}

/// Authenticate and get initial token
async fn authenticate_user(config: &ApiConfig, email: &str, password: &str) -> Result<String, Box<dyn Error>> {
    println!("🔐 Authenticating user: {}", email);
    
    let login_data = json!({
        "email": email,
        "password": password
    });
    
    let response = config.client
        .post(&format!("{}/api/v1/auth/login", config.base_url))
        .json(&login_data)
        .send()
        .await?;
    
    if response.status().is_success() {
        let token_response: Value = response.json().await?;
        let token = token_response["token"]
            .as_str()
            .ok_or("Token not found in response")?;
        
        println!("✅ Authentication successful");
        Ok(token.to_string())
    } else {
        let error_text = response.text().await?;
        Err(format!("Authentication failed: {}", error_text).into())
    }
}

/// Refresh JWT token (placeholder implementation)
async fn refresh_token(config: &ApiConfig, current_token: &str) -> Result<String, Box<dyn Error>> {
    println!("🔄 Attempting to refresh JWT token");
    
    let response = config.client
        .post(&format!("{}/api/v1/auth/refresh", config.base_url))
        .header("Authorization", format!("Bearer {}", current_token))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            // Note: Current implementation returns "refresh" string
            // In a real implementation, this would return a new TokenResponse
            println!("✅ Token refresh endpoint called successfully");
            println!("⚠️  Note: This is a placeholder implementation");
            println!("   In production, this would return a new JWT token");
            
            // For demonstration, return the same token
            // In real implementation, parse the new token from response
            Ok(current_token.to_string())
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("❌ Token refresh failed - Invalid or expired token");
            Err("Token refresh unauthorized".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ Token refresh failed with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("Token refresh failed: {}", status).into())
        }
    }
}

/// Make authenticated request with automatic token refresh
async fn make_authenticated_request(
    config: &ApiConfig,
    token: &mut String,
    endpoint: &str,
) -> Result<Value, Box<dyn Error>> {
    println!("📡 Making authenticated request to: {}", endpoint);
    
    // Check if token needs refresh before making request
    if let Ok(token_info) = TokenInfo::from_jwt(token) {
        if token_info.is_expired() {
            println!("⚠️  Token is expired, attempting refresh");
            *token = refresh_token(config, token).await?;
        } else if token_info.expires_within(Duration::from_secs(300)) { // 5 minutes
            println!("⚠️  Token expires soon, proactively refreshing");
            if let Ok(new_token) = refresh_token(config, token).await {
                *token = new_token;
            }
        }
    }
    
    let response = config.client
        .get(&format!("{}{}", config.base_url, endpoint))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    if response.status().is_success() {
        let data: Value = response.json().await?;
        println!("✅ Request successful");
        Ok(data)
    } else {
        let error_text = response.text().await?;
        Err(format!("Request failed: {}", error_text).into())
    }
}

/// Demonstrate token lifecycle management
async fn demonstrate_token_lifecycle(config: &ApiConfig, token: &str) -> Result<(), Box<dyn Error>> {
    println!("\n🔍 Token Lifecycle Analysis");
    println!("===========================");
    
    match TokenInfo::from_jwt(token) {
        Ok(token_info) => {
            println!("📊 Token Information:");
            println!("   User ID: {}", token_info.user_id);
            println!("   Issued At: {} (Unix timestamp)", token_info.issued_at);
            println!("   Expires At: {} (Unix timestamp)", token_info.expires_at);
            println!("   Is Expired: {}", token_info.is_expired());
            
            let time_remaining = token_info.time_until_expiration();
            if time_remaining.as_secs() > 0 {
                let hours = time_remaining.as_secs() / 3600;
                let minutes = (time_remaining.as_secs() % 3600) / 60;
                println!("   Time Remaining: {}h {}m", hours, minutes);
            } else {
                println!("   Time Remaining: Expired");
            }
            
            // Check various expiration thresholds
            println!("\n⏰ Expiration Checks:");
            println!("   Expires within 1 hour: {}", token_info.expires_within(Duration::from_secs(3600)));
            println!("   Expires within 5 minutes: {}", token_info.expires_within(Duration::from_secs(300)));
            println!("   Expires within 1 minute: {}", token_info.expires_within(Duration::from_secs(60)));
        }
        Err(e) => {
            println!("❌ Failed to parse token: {}", e);
        }
    }
    
    Ok(())
}

/// Simulate long-running kitchen operations with token management
async fn simulate_kitchen_shift(config: &ApiConfig, mut token: String) -> Result<(), Box<dyn Error>> {
    println!("\n🍳 Simulating Kitchen Shift Operations");
    println!("=====================================");
    
    // Simulate various kitchen operations throughout a shift
    let operations = vec![
        ("Check user profile", "/api/v1/user/profile"),
        ("Get user statistics", "/api/v1/user/stats"),
        ("Check system health", "/health/ready"),
    ];
    
    for (i, (operation_name, endpoint)) in operations.iter().enumerate() {
        println!("\n🔄 Operation {}: {}", i + 1, operation_name);
        
        match make_authenticated_request(config, &mut token, endpoint).await {
            Ok(data) => {
                println!("✅ {} completed successfully", operation_name);
                
                // Show relevant data based on endpoint
                match *endpoint {
                    "/api/v1/user/profile" => {
                        if let Some(email) = data.get("email") {
                            println!("   Staff Email: {}", email);
                        }
                        if let Some(full_name) = data.get("full_name") {
                            println!("   Staff Name: {}", full_name);
                        }
                    }
                    "/api/v1/user/stats" => {
                        if let Some(refresh_count) = data.get("refresh_token_count") {
                            println!("   Refresh Token Count: {}", refresh_count);
                        }
                    }
                    "/health/ready" => {
                        if let Some(status) = data.get("status") {
                            println!("   System Status: {}", status);
                        }
                        if let Some(database) = data.get("database") {
                            println!("   Database Status: {}", database);
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                println!("❌ {} failed: {}", operation_name, e);
            }
        }
        
        // Simulate time between operations
        println!("   ⏳ Waiting before next operation...");
        sleep(Duration::from_secs(1)).await;
    }
    
    Ok(())
}

/// Demonstrate token security best practices
fn demonstrate_security_practices() {
    println!("\n🔒 JWT Token Security Best Practices");
    println!("====================================");
    
    println!("✅ DO:");
    println!("   • Store tokens securely (encrypted storage, secure cookies)");
    println!("   • Use HTTPS for all token transmission");
    println!("   • Implement automatic token refresh before expiration");
    println!("   • Clear tokens on logout or application close");
    println!("   • Validate token expiration before each request");
    println!("   • Use short-lived tokens (24 hours or less)");
    
    println!("\n❌ DON'T:");
    println!("   • Store tokens in localStorage in production");
    println!("   • Log tokens in application logs");
    println!("   • Send tokens in URL parameters");
    println!("   • Use tokens after expiration");
    println!("   • Share tokens between different users");
    println!("   • Ignore token refresh failures");
    
    println!("\n🍽️  Kitchen Management Specific:");
    println!("   • Implement role-based token validation");
    println!("   • Use different token scopes for different kitchen areas");
    println!("   • Implement shift-based token expiration");
    println!("   • Log authentication events for audit trails");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - JWT Token Refresh Example");
    println!("=======================================================");
    
    let config = ApiConfig::new();
    println!("🌐 Using API base URL: {}", config.base_url);
    
    // First, we need to register or login to get a token
    println!("\n📋 Initial Authentication");
    println!("=========================");
    
    // Try to register a test user first
    let test_email = "jwt.test@restaurant.com";
    let test_password = "JwtTestPass123!";
    let test_name = "JWT Test User";
    
    let registration_data = json!({
        "email": test_email,
        "password": test_password,
        "full_name": test_name
    });
    
    let token = match config.client
        .post(&format!("{}/api/v1/auth/register", config.base_url))
        .json(&registration_data)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let token_response: Value = response.json().await?;
            println!("✅ New user registered successfully");
            token_response["token"].as_str().unwrap().to_string()
        }
        _ => {
            // Registration failed, try login
            println!("⚠️  Registration failed (user may exist), trying login...");
            authenticate_user(&config, test_email, test_password).await?
        }
    };
    
    // Analyze the token
    demonstrate_token_lifecycle(&config, &token).await?;
    
    // Test token refresh functionality
    println!("\n🔄 Token Refresh Testing");
    println!("========================");
    
    match refresh_token(&config, &token).await {
        Ok(_) => {
            println!("✅ Token refresh mechanism is available");
        }
        Err(e) => {
            println!("⚠️  Token refresh failed: {}", e);
            println!("   This is expected with the current placeholder implementation");
        }
    }
    
    // Simulate kitchen operations with token management
    simulate_kitchen_shift(&config, token).await?;
    
    // Show security best practices
    demonstrate_security_practices();
    
    println!("\n🎉 JWT Token Refresh Example Completed!");
    println!("=======================================");
    println!("✅ Token parsing and analysis demonstrated");
    println!("✅ Token refresh workflow tested");
    println!("✅ Automatic token management shown");
    println!("✅ Security best practices outlined");
    println!("\n💡 Next Steps:");
    println!("   - Implement proper token refresh endpoint in the API");
    println!("   - Add token storage and persistence mechanisms");
    println!("   - Run 'cargo run --example role_based_access' for permission examples");
    println!("   - Run 'cargo run --example user_crud' for user management examples");
    
    Ok(())
}