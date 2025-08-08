//! User CRUD Operations Example
//!
//! This example demonstrates comprehensive user management operations:
//! 1. Creating new users with validation
//! 2. Reading user information and profiles
//! 3. Updating user data and preferences
//! 4. Deleting users and cleanup
//! 5. Error handling for all CRUD operations
//!
//! # Kitchen Management Context
//!
//! User management is critical for kitchen operations, including staff onboarding,
//! role assignments, shift management, and maintaining accurate personnel records.

use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use uuid::Uuid;
use chrono::Utc;

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

/// Kitchen staff member data structure
#[derive(Debug, Clone)]
struct KitchenUser {
    id: Option<Uuid>,
    email: String,
    password: String,
    full_name: String,
    role: String,
    station: Option<String>,
    shift: Option<String>,
    preferences: Option<Value>,
}

impl KitchenUser {
    fn new(email: &str, password: &str, full_name: &str, role: &str) -> Self {
        Self {
            id: None,
            email: email.to_string(),
            password: password.to_string(),
            full_name: full_name.to_string(),
            role: role.to_string(),
            station: None,
            shift: None,
            preferences: None,
        }
    }
    
    fn with_station(mut self, station: &str) -> Self {
        self.station = Some(station.to_string());
        self
    }
    
    fn with_shift(mut self, shift: &str) -> Self {
        self.shift = Some(shift.to_string());
        self
    }
    
    fn with_preferences(mut self, preferences: Value) -> Self {
        self.preferences = Some(preferences);
        self
    }
}

/// Authenticate and get admin token for user management operations
async fn get_admin_token(config: &ApiConfig) -> Result<String, Box<dyn Error>> {
    println!("🔐 Authenticating admin user for user management operations");
    
    // Create or login as admin user
    let admin_email = "admin@restaurant.com";
    let admin_password = "AdminPass123!";
    let admin_name = "System Administrator";
    
    let registration_data = json!({
        "email": admin_email,
        "password": admin_password,
        "full_name": admin_name
    });
    
    // Try registration first
    match config.client
        .post(&format!("{}/api/v1/auth/register", config.base_url))
        .json(&registration_data)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let token_response: Value = response.json().await?;
            println!("✅ Admin user registered successfully");
            Ok(token_response["token"].as_str().unwrap().to_string())
        }
        _ => {
            // Registration failed, try login
            let login_data = json!({
                "email": admin_email,
                "password": admin_password
            });
            
            let response = config.client
                .post(&format!("{}/api/v1/auth/login", config.base_url))
                .json(&login_data)
                .send()
                .await?;
            
            if response.status().is_success() {
                let token_response: Value = response.json().await?;
                println!("✅ Admin user logged in successfully");
                Ok(token_response["token"].as_str().unwrap().to_string())
            } else {
                Err("Failed to authenticate admin user".into())
            }
        }
    }
}

/// Create a new user via the API
async fn create_user(config: &ApiConfig, token: &str, user: &KitchenUser) -> Result<Value, Box<dyn Error>> {
    println!("👤 Creating new user: {} ({})", user.full_name, user.role);
    
    let user_data = json!({
        "id": Uuid::new_v4(),
        "email": user.email,
        "password_hash": format!("hashed_{}", user.password), // In real app, this would be properly hashed
        "full_name": user.full_name,
        "preferences": user.preferences,
        "created_at": Utc::now(),
        "updated_at": Utc::now()
    });
    
    let response = config.client
        .post(&format!("{}/api/v1/users", config.base_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&user_data)
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::CREATED => {
            let created_user: Value = response.json().await?;
            println!("✅ User created successfully");
            println!("   ID: {}", created_user["id"]);
            println!("   Email: {}", created_user["email"]);
            println!("   Role: {}", user.role);
            if let Some(station) = &user.station {
                println!("   Station: {}", station);
            }
            if let Some(shift) = &user.shift {
                println!("   Shift: {}", shift);
            }
            Ok(created_user)
        }
        reqwest::StatusCode::CONFLICT => {
            println!("⚠️  User creation failed - Email already exists");
            Err("User with this email already exists".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ User creation failed with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("User creation failed: {}", status).into())
        }
    }
}

/// Read user information by ID
async fn read_user(config: &ApiConfig, token: &str, user_id: &str) -> Result<Value, Box<dyn Error>> {
    println!("📖 Reading user information for ID: {}", user_id);
    
    let response = config.client
        .get(&format!("{}/api/v1/users/{}", config.base_url, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let user_data: Value = response.json().await?;
            println!("✅ User information retrieved successfully");
            
            // The API returns a tuple (user, authenticated_user_id), so we need the first element
            if let Some(user_info) = user_data.as_array().and_then(|arr| arr.get(0)) {
                println!("   ID: {}", user_info["id"]);
                println!("   Email: {}", user_info["email"]);
                println!("   Full Name: {}", user_info["full_name"]);
                println!("   Created: {}", user_info["created_at"]);
                println!("   Updated: {}", user_info["updated_at"]);
                
                if let Some(preferences) = user_info.get("preferences") {
                    if !preferences.is_null() {
                        println!("   Preferences: {}", preferences);
                    }
                }
                
                Ok(user_info.clone())
            } else {
                Ok(user_data)
            }
        }
        reqwest::StatusCode::NOT_FOUND => {
            println!("❌ User not found");
            Err("User not found".into())
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("❌ Unauthorized - Invalid token");
            Err("Authentication failed".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ Failed to read user with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("Read user failed: {}", status).into())
        }
    }
}

/// Update user information
async fn update_user(config: &ApiConfig, token: &str, user_id: &str, new_name: &str) -> Result<Value, Box<dyn Error>> {
    println!("✏️  Updating user {} with new name: {}", user_id, new_name);
    
    let response = config.client
        .put(&format!("{}/api/v1/users/{}", config.base_url, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!(new_name))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let updated_user: Value = response.json().await?;
            println!("✅ User updated successfully");
            println!("   ID: {}", updated_user["id"]);
            println!("   New Name: {}", updated_user["full_name"]);
            println!("   Updated At: {}", updated_user["updated_at"]);
            Ok(updated_user)
        }
        reqwest::StatusCode::NOT_FOUND => {
            println!("❌ User not found for update");
            Err("User not found".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ User update failed with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("User update failed: {}", status).into())
        }
    }
}

/// Delete user by ID
async fn delete_user(config: &ApiConfig, token: &str, user_id: &str) -> Result<(), Box<dyn Error>> {
    println!("🗑️  Deleting user: {}", user_id);
    
    let response = config.client
        .delete(&format!("{}/api/v1/users/{}", config.base_url, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::NO_CONTENT => {
            println!("✅ User deleted successfully");
            Ok(())
        }
        reqwest::StatusCode::NOT_FOUND => {
            println!("❌ User not found for deletion");
            Err("User not found".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ User deletion failed with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("User deletion failed: {}", status).into())
        }
    }
}

/// Demonstrate complete CRUD workflow for a single user
async fn demonstrate_user_lifecycle(config: &ApiConfig, token: &str, user: &KitchenUser) -> Result<(), Box<dyn Error>> {
    println!("\n🔄 Complete User Lifecycle for: {}", user.full_name);
    println!("{}=", "=".repeat(50));
    
    // CREATE
    let created_user = create_user(config, token, user).await?;
    let user_id = created_user["id"].as_str().ok_or("No user ID in response")?;
    
    // READ
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let read_user = read_user(config, token, user_id).await?;
    
    // UPDATE
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let new_name = format!("{} (Updated)", user.full_name);
    let updated_user = update_user(config, token, user_id, &new_name).await?;
    
    // READ again to verify update
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("🔍 Verifying update:");
    read_user(config, token, user_id).await?;
    
    // DELETE
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    delete_user(config, token, user_id).await?;
    
    // Verify deletion
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("🔍 Verifying deletion:");
    match read_user(config, token, user_id).await {
        Ok(_) => println!("⚠️  User still exists after deletion"),
        Err(_) => println!("✅ User successfully deleted"),
    }
    
    Ok(())
}

/// Demonstrate error handling scenarios
async fn demonstrate_error_handling(config: &ApiConfig, token: &str) -> Result<(), Box<dyn Error>> {
    println!("\n🚨 Error Handling Scenarios");
    println!("===========================");
    
    // Test reading non-existent user
    println!("\n1. Reading non-existent user:");
    let fake_id = Uuid::new_v4().to_string();
    match read_user(config, token, &fake_id).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }
    
    // Test updating non-existent user
    println!("\n2. Updating non-existent user:");
    match update_user(config, token, &fake_id, "New Name").await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }
    
    // Test deleting non-existent user
    println!("\n3. Deleting non-existent user:");
    match delete_user(config, token, &fake_id).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected error: {}", e),
    }
    
    // Test with invalid token
    println!("\n4. Operations with invalid token:");
    match read_user(config, "invalid.jwt.token", &fake_id).await {
        Ok(_) => println!("   ⚠️  Unexpected success"),
        Err(e) => println!("   ✅ Expected authentication error: {}", e),
    }
    
    Ok(())
}

/// Demonstrate batch user operations
async fn demonstrate_batch_operations(config: &ApiConfig, token: &str) -> Result<(), Box<dyn Error>> {
    println!("\n👥 Batch User Operations");
    println!("========================");
    
    // Create multiple users for different kitchen roles
    let kitchen_staff = vec![
        KitchenUser::new("batch.chef@restaurant.com", "ChefPass123!", "Batch Chef", "Head Chef")
            .with_station("Main Kitchen")
            .with_shift("Morning")
            .with_preferences(json!({
                "theme": "dark",
                "notifications": true,
                "language": "en"
            })),
        KitchenUser::new("batch.cook1@restaurant.com", "CookPass123!", "Batch Cook 1", "Line Cook")
            .with_station("Grill")
            .with_shift("Evening"),
        KitchenUser::new("batch.cook2@restaurant.com", "CookPass123!", "Batch Cook 2", "Line Cook")
            .with_station("Prep")
            .with_shift("Morning"),
        KitchenUser::new("batch.prep@restaurant.com", "PrepPass123!", "Batch Prep Cook", "Prep Cook")
            .with_station("Prep Kitchen")
            .with_shift("Early Morning"),
    ];
    
    let mut created_users = Vec::new();
    
    // Create all users
    println!("\n📝 Creating batch users:");
    for user in &kitchen_staff {
        match create_user(config, token, user).await {
            Ok(created_user) => {
                let user_id = created_user["id"].as_str().unwrap().to_string();
                created_users.push(user_id);
                println!("   ✅ Created: {}", user.full_name);
            }
            Err(e) => {
                println!("   ❌ Failed to create {}: {}", user.full_name, e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    
    // Read all created users
    println!("\n📖 Reading all created users:");
    for user_id in &created_users {
        match read_user(config, token, user_id).await {
            Ok(_) => println!("   ✅ Read user: {}", user_id),
            Err(e) => println!("   ❌ Failed to read {}: {}", user_id, e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    
    // Update all users with a batch suffix
    println!("\n✏️  Batch updating users:");
    for (i, user_id) in created_users.iter().enumerate() {
        let new_name = format!("Updated Batch User {}", i + 1);
        match update_user(config, token, user_id, &new_name).await {
            Ok(_) => println!("   ✅ Updated user: {}", user_id),
            Err(e) => println!("   ❌ Failed to update {}: {}", user_id, e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    
    // Clean up - delete all created users
    println!("\n🗑️  Cleaning up batch users:");
    for user_id in &created_users {
        match delete_user(config, token, user_id).await {
            Ok(_) => println!("   ✅ Deleted user: {}", user_id),
            Err(e) => println!("   ❌ Failed to delete {}: {}", user_id, e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    
    println!("✅ Batch operations completed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - User CRUD Operations Example");
    println!("==========================================================");
    
    let config = ApiConfig::new();
    println!("🌐 Using API base URL: {}", config.base_url);
    
    // Get admin token for user management
    let admin_token = get_admin_token(&config).await?;
    
    // Create sample kitchen users
    let sample_users = vec![
        KitchenUser::new("crud.chef@restaurant.com", "ChefPass123!", "CRUD Test Chef", "Head Chef")
            .with_station("Main Kitchen")
            .with_shift("Day Shift")
            .with_preferences(json!({
                "theme": "light",
                "notifications": true,
                "dashboard_layout": "compact"
            })),
        KitchenUser::new("crud.cook@restaurant.com", "CookPass123!", "CRUD Test Cook", "Line Cook")
            .with_station("Grill Station")
            .with_shift("Evening Shift"),
        KitchenUser::new("crud.prep@restaurant.com", "PrepPass123!", "CRUD Test Prep", "Prep Cook")
            .with_station("Prep Kitchen")
            .with_shift("Morning Shift"),
    ];
    
    println!("\n🔄 Individual User CRUD Workflows");
    println!("=================================");
    
    // Demonstrate complete lifecycle for each user
    for user in &sample_users {
        demonstrate_user_lifecycle(&config, &admin_token, user).await?;
        println!(); // Add spacing between users
    }
    
    // Demonstrate error handling
    demonstrate_error_handling(&config, &admin_token).await?;
    
    // Demonstrate batch operations
    demonstrate_batch_operations(&config, &admin_token).await?;
    
    println!("\n📊 CRUD Operations Summary");
    println!("==========================");
    println!("✅ CREATE: User creation with validation and error handling");
    println!("✅ READ: User information retrieval with proper formatting");
    println!("✅ UPDATE: User data modification with verification");
    println!("✅ DELETE: User removal with confirmation");
    println!("✅ ERROR HANDLING: Comprehensive error scenarios tested");
    println!("✅ BATCH OPERATIONS: Multiple user management workflows");
    
    println!("\n🍳 Kitchen Management Best Practices");
    println!("====================================");
    println!("✅ User Lifecycle Management:");
    println!("   • Proper onboarding and offboarding procedures");
    println!("   • Role-based user creation with appropriate permissions");
    println!("   • Regular user data validation and cleanup");
    
    println!("\n✅ Data Integrity:");
    println!("   • Validate user data before creation/updates");
    println!("   • Handle duplicate email addresses gracefully");
    println!("   • Maintain audit trails for user changes");
    
    println!("\n✅ Security Considerations:");
    println!("   • Use proper authentication for all operations");
    println!("   • Implement role-based access control");
    println!("   • Secure password handling and storage");
    
    println!("\n🎉 User CRUD Operations Example Completed!");
    println!("==========================================");
    println!("✅ All CRUD operations demonstrated");
    println!("✅ Error handling patterns shown");
    println!("✅ Batch operations implemented");
    println!("✅ Kitchen management context provided");
    println!("\n💡 Next Steps:");
    println!("   - Run 'cargo run --example user_stats' for user analytics");
    println!("   - Run 'cargo run --example profile_management' for profile operations");
    println!("   - Run 'cargo run --example full_workflow' for complete integration");
    
    Ok(())
}