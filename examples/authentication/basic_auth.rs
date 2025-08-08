//! Basic Authentication Example
//!
//! This example demonstrates the fundamental authentication flow for the Kitchen Management API:
//! 1. User registration with validation
//! 2. User login with credential verification
//! 3. Using authentication tokens for API requests
//!
//! # Kitchen Management Context
//!
//! This workflow is typically used during staff onboarding when new kitchen personnel
//! need accounts to access order management, inventory tracking, and shift coordination.

use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;

/// Configuration for the API client
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

/// Represents a kitchen staff member for registration
#[derive(Debug)]
struct KitchenStaff {
    email: String,
    password: String,
    full_name: String,
    role: String,
}

impl KitchenStaff {
    fn new(email: &str, password: &str, full_name: &str, role: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
            full_name: full_name.to_string(),
            role: role.to_string(),
        }
    }
}

/// Register a new kitchen staff member
async fn register_staff_member(config: &ApiConfig, staff: &KitchenStaff) -> Result<String, Box<dyn Error>> {
    println!("🔐 Registering new kitchen staff member: {}", staff.full_name);
    
    let registration_data = json!({
        "email": staff.email,
        "password": staff.password,
        "full_name": staff.full_name
    });
    
    let response = config.client
        .post(&format!("{}/api/v1/auth/register", config.base_url))
        .json(&registration_data)
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let token_response: Value = response.json().await?;
            let token = token_response["token"]
                .as_str()
                .ok_or("Token not found in response")?;
            
            println!("✅ Registration successful for {}", staff.email);
            println!("   Role: {}", staff.role);
            println!("   Token received (first 20 chars): {}...", &token[..20.min(token.len())]);
            
            Ok(token.to_string())
        }
        reqwest::StatusCode::BAD_REQUEST => {
            let error_response: Value = response.json().await?;
            println!("❌ Registration failed - Validation error:");
            
            if let Some(validation_errors) = error_response.get("validation_errors") {
                println!("   Validation issues:");
                if let Some(obj) = validation_errors.as_object() {
                    for (field, errors) in obj {
                        if let Some(error_array) = errors.as_array() {
                            for error in error_array {
                                println!("   - {}: {}", field, error);
                            }
                        }
                    }
                }
            } else if let Some(details) = error_response.get("details") {
                println!("   Details: {}", details);
            }
            
            Err("Registration validation failed".into())
        }
        status => {
            println!("❌ Registration failed with status: {}", status);
            let error_text = response.text().await?;
            println!("   Error: {}", error_text);
            Err(format!("Registration failed: {}", status).into())
        }
    }
}

/// Login with existing credentials
async fn login_staff_member(config: &ApiConfig, email: &str, password: &str) -> Result<String, Box<dyn Error>> {
    println!("🔑 Logging in staff member: {}", email);
    
    let login_data = json!({
        "email": email,
        "password": password
    });
    
    let response = config.client
        .post(&format!("{}/api/v1/auth/login", config.base_url))
        .json(&login_data)
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let token_response: Value = response.json().await?;
            let token = token_response["token"]
                .as_str()
                .ok_or("Token not found in response")?;
            
            println!("✅ Login successful for {}", email);
            println!("   Token received (first 20 chars): {}...", &token[..20.min(token.len())]);
            
            Ok(token.to_string())
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("❌ Login failed - Invalid credentials");
            println!("   Please check email and password");
            Err("Invalid credentials".into())
        }
        reqwest::StatusCode::BAD_REQUEST => {
            let error_response: Value = response.json().await?;
            println!("❌ Login failed - Validation error:");
            
            if let Some(validation_errors) = error_response.get("validation_errors") {
                println!("   Validation issues:");
                if let Some(obj) = validation_errors.as_object() {
                    for (field, errors) in obj {
                        if let Some(error_array) = errors.as_array() {
                            for error in error_array {
                                println!("   - {}: {}", field, error);
                            }
                        }
                    }
                }
            }
            
            Err("Login validation failed".into())
        }
        status => {
            println!("❌ Login failed with status: {}", status);
            let error_text = response.text().await?;
            println!("   Error: {}", error_text);
            Err(format!("Login failed: {}", status).into())
        }
    }
}

/// Make an authenticated request to get user profile
async fn get_user_profile(config: &ApiConfig, token: &str) -> Result<Value, Box<dyn Error>> {
    println!("👤 Fetching user profile with authentication token");
    
    let response = config.client
        .get(&format!("{}/api/v1/user/profile", config.base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let profile: Value = response.json().await?;
            println!("✅ Profile retrieved successfully");
            
            if let Some(email) = profile.get("email") {
                println!("   Email: {}", email);
            }
            if let Some(full_name) = profile.get("full_name") {
                println!("   Full Name: {}", full_name);
            }
            if let Some(created_at) = profile.get("created_at") {
                println!("   Account Created: {}", created_at);
            }
            
            Ok(profile)
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("❌ Profile request failed - Invalid or expired token");
            Err("Authentication failed".into())
        }
        status => {
            println!("❌ Profile request failed with status: {}", status);
            let error_text = response.text().await?;
            println!("   Error: {}", error_text);
            Err(format!("Profile request failed: {}", status).into())
        }
    }
}

/// Demonstrate authentication error handling
async fn demonstrate_error_handling(config: &ApiConfig) -> Result<(), Box<dyn Error>> {
    println!("\n🔍 Demonstrating error handling scenarios:");
    
    // 1. Invalid email format
    println!("\n1. Testing invalid email format:");
    let invalid_email_staff = KitchenStaff::new(
        "invalid-email-format",
        "ValidPass123!",
        "Test User",
        "Kitchen Staff"
    );
    
    if let Err(e) = register_staff_member(config, &invalid_email_staff).await {
        println!("   Expected error caught: {}", e);
    }
    
    // 2. Weak password
    println!("\n2. Testing weak password:");
    let weak_password_staff = KitchenStaff::new(
        "weak@example.com",
        "123",
        "Weak Password User",
        "Kitchen Staff"
    );
    
    if let Err(e) = register_staff_member(config, &weak_password_staff).await {
        println!("   Expected error caught: {}", e);
    }
    
    // 3. Invalid login credentials
    println!("\n3. Testing invalid login credentials:");
    if let Err(e) = login_staff_member(config, "nonexistent@example.com", "wrongpassword").await {
        println!("   Expected error caught: {}", e);
    }
    
    // 4. Invalid authentication token
    println!("\n4. Testing invalid authentication token:");
    if let Err(e) = get_user_profile(config, "invalid.jwt.token").await {
        println!("   Expected error caught: {}", e);
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - Basic Authentication Example");
    println!("========================================================");
    
    let config = ApiConfig::new();
    println!("🌐 Using API base URL: {}", config.base_url);
    
    // Create sample kitchen staff members for different roles
    let head_chef = KitchenStaff::new(
        "head.chef@restaurant.com",
        "SecureChefPass123!",
        "Head Chef",
        "Head Chef"
    );
    
    let line_cook = KitchenStaff::new(
        "line.cook@restaurant.com",
        "CookPassword456!",
        "Line Cook",
        "Line Cook"
    );
    
    println!("\n📋 Kitchen Staff Registration Process");
    println!("=====================================");
    
    // Register head chef
    let head_chef_token = match register_staff_member(&config, &head_chef).await {
        Ok(token) => token,
        Err(e) => {
            println!("⚠️  Head chef registration failed (may already exist): {}", e);
            // Try to login instead
            match login_staff_member(&config, &head_chef.email, &head_chef.password).await {
                Ok(token) => {
                    println!("✅ Logged in existing head chef instead");
                    token
                }
                Err(login_err) => {
                    println!("❌ Both registration and login failed: {}", login_err);
                    return Err(login_err);
                }
            }
        }
    };
    
    // Register line cook
    let line_cook_token = match register_staff_member(&config, &line_cook).await {
        Ok(token) => token,
        Err(e) => {
            println!("⚠️  Line cook registration failed (may already exist): {}", e);
            // Try to login instead
            match login_staff_member(&config, &line_cook.email, &line_cook.password).await {
                Ok(token) => {
                    println!("✅ Logged in existing line cook instead");
                    token
                }
                Err(login_err) => {
                    println!("❌ Both registration and login failed: {}", login_err);
                    return Err(login_err);
                }
            }
        }
    };
    
    println!("\n🔐 Authentication Testing");
    println!("========================");
    
    // Test login with head chef credentials
    let login_token = login_staff_member(&config, &head_chef.email, &head_chef.password).await?;
    println!("🔄 Login token matches registration: {}", login_token == head_chef_token);
    
    println!("\n👥 Authenticated API Requests");
    println!("=============================");
    
    // Get profiles for both users
    println!("\n📄 Head Chef Profile:");
    get_user_profile(&config, &head_chef_token).await?;
    
    println!("\n📄 Line Cook Profile:");
    get_user_profile(&config, &line_cook_token).await?;
    
    // Demonstrate error handling
    demonstrate_error_handling(&config).await?;
    
    println!("\n🎉 Basic Authentication Example Completed Successfully!");
    println!("======================================================");
    println!("✅ Registration workflow tested");
    println!("✅ Login workflow tested");
    println!("✅ Authenticated requests tested");
    println!("✅ Error handling demonstrated");
    println!("\n💡 Next Steps:");
    println!("   - Run 'cargo run --example jwt_refresh' for token refresh examples");
    println!("   - Run 'cargo run --example user_crud' for user management examples");
    println!("   - Run 'cargo run --example full_workflow' for complete integration examples");
    
    Ok(())
}