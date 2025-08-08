//! Profile Management Example
//!
//! This example demonstrates user profile management functionality:
//! 1. Retrieving current user profile information
//! 2. Managing user preferences and settings
//! 3. Profile validation and error handling
//! 4. Kitchen-specific profile customization
//!
//! # Kitchen Management Context
//!
//! Profile management is essential for personalizing the kitchen management
//! experience, storing user preferences, shift information, and role-specific
//! settings that improve workflow efficiency.

use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use chrono::{DateTime, Utc};

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

/// User profile data structure
#[derive(Debug, Clone)]
struct UserProfile {
    id: String,
    email: String,
    full_name: String,
    preferences: Option<Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl UserProfile {
    fn from_json(data: &Value) -> Result<Self, Box<dyn Error>> {
        Ok(UserProfile {
            id: data["id"].as_str().ok_or("Missing id")?.to_string(),
            email: data["email"].as_str().ok_or("Missing email")?.to_string(),
            full_name: data["full_name"].as_str().ok_or("Missing full_name")?.to_string(),
            preferences: data.get("preferences").cloned(),
            created_at: data["created_at"].as_str()
                .ok_or("Missing created_at")?
                .parse()?,
            updated_at: data["updated_at"].as_str()
                .ok_or("Missing updated_at")?
                .parse()?,
        })
    }
    
    /// Get preference value by key
    fn get_preference(&self, key: &str) -> Option<&Value> {
        self.preferences.as_ref()?.get(key)
    }
    
    /// Check if user has specific preference set
    fn has_preference(&self, key: &str) -> bool {
        self.get_preference(key).is_some()
    }
    
    /// Get theme preference
    fn theme(&self) -> String {
        self.get_preference("theme")
            .and_then(|v| v.as_str())
            .unwrap_or("light")
            .to_string()
    }
    
    /// Get notification preference
    fn notifications_enabled(&self) -> bool {
        self.get_preference("notifications")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }
    
    /// Get language preference
    fn language(&self) -> String {
        self.get_preference("language")
            .and_then(|v| v.as_str())
            .unwrap_or("en")
            .to_string()
    }
    
    /// Get kitchen-specific preferences
    fn kitchen_station(&self) -> Option<String> {
        self.get_preference("kitchen_station")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    
    fn shift_preference(&self) -> Option<String> {
        self.get_preference("shift")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    
    fn dashboard_layout(&self) -> String {
        self.get_preference("dashboard_layout")
            .and_then(|v| v.as_str())
            .unwrap_or("standard")
            .to_string()
    }
}

/// Kitchen-specific user preferences
#[derive(Debug, Clone)]
struct KitchenPreferences {
    theme: String,
    notifications: bool,
    language: String,
    kitchen_station: Option<String>,
    shift: Option<String>,
    dashboard_layout: String,
    quick_actions: Vec<String>,
    temperature_unit: String,
    time_format: String,
}

impl KitchenPreferences {
    fn new() -> Self {
        Self {
            theme: "light".to_string(),
            notifications: true,
            language: "en".to_string(),
            kitchen_station: None,
            shift: None,
            dashboard_layout: "standard".to_string(),
            quick_actions: vec!["orders".to_string(), "inventory".to_string()],
            temperature_unit: "fahrenheit".to_string(),
            time_format: "12h".to_string(),
        }
    }
    
    fn for_head_chef() -> Self {
        Self {
            theme: "dark".to_string(),
            notifications: true,
            language: "en".to_string(),
            kitchen_station: Some("Main Kitchen".to_string()),
            shift: Some("All Day".to_string()),
            dashboard_layout: "executive".to_string(),
            quick_actions: vec![
                "orders".to_string(),
                "inventory".to_string(),
                "staff".to_string(),
                "reports".to_string()
            ],
            temperature_unit: "fahrenheit".to_string(),
            time_format: "24h".to_string(),
        }
    }
    
    fn for_line_cook() -> Self {
        Self {
            theme: "light".to_string(),
            notifications: true,
            language: "en".to_string(),
            kitchen_station: Some("Grill Station".to_string()),
            shift: Some("Evening".to_string()),
            dashboard_layout: "compact".to_string(),
            quick_actions: vec!["orders".to_string(), "recipes".to_string()],
            temperature_unit: "fahrenheit".to_string(),
            time_format: "12h".to_string(),
        }
    }
    
    fn for_prep_cook() -> Self {
        Self {
            theme: "light".to_string(),
            notifications: false,
            language: "en".to_string(),
            kitchen_station: Some("Prep Kitchen".to_string()),
            shift: Some("Morning".to_string()),
            dashboard_layout: "minimal".to_string(),
            quick_actions: vec!["prep_lists".to_string(), "inventory".to_string()],
            temperature_unit: "fahrenheit".to_string(),
            time_format: "12h".to_string(),
        }
    }
    
    fn to_json(&self) -> Value {
        json!({
            "theme": self.theme,
            "notifications": self.notifications,
            "language": self.language,
            "kitchen_station": self.kitchen_station,
            "shift": self.shift,
            "dashboard_layout": self.dashboard_layout,
            "quick_actions": self.quick_actions,
            "temperature_unit": self.temperature_unit,
            "time_format": self.time_format
        })
    }
}

/// Create and authenticate a test user
async fn create_test_user(config: &ApiConfig, email: &str, password: &str, full_name: &str) -> Result<String, Box<dyn Error>> {
    println!("👤 Creating/authenticating user: {}", full_name);
    
    let registration_data = json!({
        "email": email,
        "password": password,
        "full_name": full_name
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
            println!("✅ User registered successfully");
            Ok(token_response["token"].as_str().unwrap().to_string())
        }
        _ => {
            // Registration failed, try login
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
                println!("✅ User logged in successfully");
                Ok(token_response["token"].as_str().unwrap().to_string())
            } else {
                Err(format!("Failed to authenticate user: {}", email).into())
            }
        }
    }
}

/// Get current user profile
async fn get_user_profile(config: &ApiConfig, token: &str) -> Result<UserProfile, Box<dyn Error>> {
    println!("👤 Retrieving user profile");
    
    let response = config.client
        .get(&format!("{}/api/v1/user/profile", config.base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let profile_data: Value = response.json().await?;
            println!("✅ Profile retrieved successfully");
            
            let profile = UserProfile::from_json(&profile_data)?;
            Ok(profile)
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("❌ Unauthorized - Invalid token");
            Err("Authentication failed".into())
        }
        reqwest::StatusCode::NOT_FOUND => {
            println!("❌ Profile not found");
            Err("Profile not found".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ Failed to get profile with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("Profile request failed: {}", status).into())
        }
    }
}

/// Display detailed profile information
fn display_profile_information(profile: &UserProfile) {
    println!("\n📋 User Profile Information");
    println!("===========================");
    
    println!("👤 Basic Information:");
    println!("   ID: {}", profile.id);
    println!("   Email: {}", profile.email);
    println!("   Full Name: {}", profile.full_name);
    println!("   Created: {}", profile.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("   Updated: {}", profile.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
    
    println!("\n⚙️  User Preferences:");
    if let Some(preferences) = &profile.preferences {
        if preferences.is_null() {
            println!("   No preferences set");
        } else {
            println!("   Theme: {}", profile.theme());
            println!("   Notifications: {}", if profile.notifications_enabled() { "Enabled" } else { "Disabled" });
            println!("   Language: {}", profile.language());
            println!("   Dashboard Layout: {}", profile.dashboard_layout());
            
            if let Some(station) = profile.kitchen_station() {
                println!("   Kitchen Station: {}", station);
            }
            
            if let Some(shift) = profile.shift_preference() {
                println!("   Preferred Shift: {}", shift);
            }
            
            // Display raw preferences for debugging
            println!("   Raw Preferences: {}", serde_json::to_string_pretty(preferences).unwrap_or_else(|_| "Invalid JSON".to_string()));
        }
    } else {
        println!("   No preferences configured");
    }
    
    println!("\n🍳 Kitchen Management Context:");
    match profile.kitchen_station() {
        Some(station) => {
            println!("   Assigned Station: {}", station);
            match station.as_str() {
                "Main Kitchen" => println!("   Role: Likely Head Chef or Kitchen Manager"),
                "Grill Station" => println!("   Role: Grill Cook or Line Cook"),
                "Prep Kitchen" => println!("   Role: Prep Cook or Kitchen Assistant"),
                "Pastry Station" => println!("   Role: Pastry Chef or Baker"),
                _ => println!("   Role: Kitchen Staff"),
            }
        }
        None => println!("   No station assignment - may need role configuration"),
    }
    
    if let Some(shift) = profile.shift_preference() {
        println!("   Preferred Shift: {}", shift);
        match shift.as_str() {
            "Morning" => println!("   Typical Hours: 6:00 AM - 2:00 PM"),
            "Evening" => println!("   Typical Hours: 2:00 PM - 10:00 PM"),
            "Night" => println!("   Typical Hours: 10:00 PM - 6:00 AM"),
            "All Day" => println!("   Typical Hours: Management/Supervisory role"),
            _ => println!("   Custom shift schedule"),
        }
    }
}

/// Demonstrate profile customization for different kitchen roles
async fn demonstrate_role_based_profiles(config: &ApiConfig) -> Result<(), Box<dyn Error>> {
    println!("\n👥 Role-Based Profile Customization");
    println!("===================================");
    
    let kitchen_roles = vec![
        ("head.chef.profile@restaurant.com", "HeadChefProfile123!", "Head Chef Profile", KitchenPreferences::for_head_chef()),
        ("line.cook.profile@restaurant.com", "LineCookProfile123!", "Line Cook Profile", KitchenPreferences::for_line_cook()),
        ("prep.cook.profile@restaurant.com", "PrepCookProfile123!", "Prep Cook Profile", KitchenPreferences::for_prep_cook()),
    ];
    
    for (email, password, name, preferences) in kitchen_roles {
        println!("\n🔧 Setting up profile for: {}", name);
        
        match create_test_user(config, email, password, name).await {
            Ok(token) => {
                // Get initial profile
                match get_user_profile(config, &token).await {
                    Ok(profile) => {
                        println!("📋 Initial profile for {}:", name);
                        display_profile_information(&profile);
                        
                        // Note: In a real implementation, you would update the user's preferences
                        // via a PUT request to /api/v1/user/profile or similar endpoint
                        println!("\n💡 Recommended preferences for {}:", name);
                        println!("{}", serde_json::to_string_pretty(&preferences.to_json()).unwrap());
                    }
                    Err(e) => {
                        println!("❌ Failed to get profile for {}: {}", name, e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to create/authenticate {}: {}", name, e);
            }
        }
        
        // Small delay between users
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    Ok(())
}

/// Demonstrate profile validation and error handling
async fn demonstrate_profile_validation(config: &ApiConfig, token: &str) -> Result<(), Box<dyn Error>> {
    println!("\n🔍 Profile Validation and Error Handling");
    println!("========================================");
    
    // Test with valid token
    println!("\n1. Valid profile request:");
    match get_user_profile(config, token).await {
        Ok(profile) => {
            println!("   ✅ Profile retrieved successfully");
            println!("   📧 Email: {}", profile.email);
            println!("   👤 Name: {}", profile.full_name);
        }
        Err(e) => {
            println!("   ❌ Unexpected error: {}", e);
        }
    }
    
    // Test with invalid token
    println!("\n2. Invalid token request:");
    match get_user_profile(config, "invalid.jwt.token").await {
        Ok(_) => {
            println!("   ⚠️  Unexpected success with invalid token");
        }
        Err(e) => {
            println!("   ✅ Expected authentication error: {}", e);
        }
    }
    
    // Test with malformed token
    println!("\n3. Malformed token request:");
    match get_user_profile(config, "not.a.token").await {
        Ok(_) => {
            println!("   ⚠️  Unexpected success with malformed token");
        }
        Err(e) => {
            println!("   ✅ Expected token error: {}", e);
        }
    }
    
    // Test with empty token
    println!("\n4. Empty token request:");
    match get_user_profile(config, "").await {
        Ok(_) => {
            println!("   ⚠️  Unexpected success with empty token");
        }
        Err(e) => {
            println!("   ✅ Expected authentication error: {}", e);
        }
    }
    
    Ok(())
}

/// Demonstrate profile monitoring and analytics
async fn demonstrate_profile_monitoring(config: &ApiConfig, token: &str) -> Result<(), Box<dyn Error>> {
    println!("\n📊 Profile Monitoring and Analytics");
    println!("===================================");
    
    println!("🔄 Monitoring profile changes over time...");
    
    for i in 1..=3 {
        println!("\n📈 Monitoring cycle {} of 3:", i);
        
        match get_user_profile(config, token).await {
            Ok(profile) => {
                println!("   ✅ Profile retrieved successfully");
                println!("   📧 Email: {}", profile.email);
                println!("   🕐 Last Updated: {}", profile.updated_at.format("%H:%M:%S"));
                println!("   🎨 Theme: {}", profile.theme());
                println!("   🔔 Notifications: {}", if profile.notifications_enabled() { "On" } else { "Off" });
                
                if let Some(station) = profile.kitchen_station() {
                    println!("   🍳 Station: {}", station);
                }
                
                // Calculate profile completeness
                let mut completeness_score = 0;
                let mut total_fields = 5;
                
                if profile.has_preference("theme") { completeness_score += 1; }
                if profile.has_preference("notifications") { completeness_score += 1; }
                if profile.has_preference("language") { completeness_score += 1; }
                if profile.has_preference("kitchen_station") { completeness_score += 1; }
                if profile.has_preference("dashboard_layout") { completeness_score += 1; }
                
                let completeness_percentage = (completeness_score as f64 / total_fields as f64) * 100.0;
                println!("   📊 Profile Completeness: {:.1}% ({}/{})", completeness_percentage, completeness_score, total_fields);
                
                if completeness_percentage < 80.0 {
                    println!("   ⚠️  Profile needs more configuration");
                } else {
                    println!("   ✅ Profile well configured");
                }
            }
            Err(e) => {
                println!("   ❌ Failed to retrieve profile: {}", e);
            }
        }
        
        if i < 3 {
            println!("   ⏳ Waiting 2 seconds before next check...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
    
    println!("✅ Profile monitoring completed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - Profile Management Example");
    println!("========================================================");
    
    let config = ApiConfig::new();
    println!("🌐 Using API base URL: {}", config.base_url);
    
    // Create a primary test user for profile management
    let primary_user_token = create_test_user(
        &config,
        "profile.manager@restaurant.com",
        "ProfileManagerPass123!",
        "Profile Manager User"
    ).await?;
    
    println!("\n👤 Primary User Profile Analysis");
    println!("================================");
    
    // Get and display detailed profile for primary user
    let primary_profile = get_user_profile(&config, &primary_user_token).await?;
    display_profile_information(&primary_profile);
    
    // Demonstrate role-based profile customization
    demonstrate_role_based_profiles(&config).await?;
    
    // Demonstrate profile validation and error handling
    demonstrate_profile_validation(&config, &primary_user_token).await?;
    
    // Demonstrate profile monitoring
    demonstrate_profile_monitoring(&config, &primary_user_token).await?;
    
    println!("\n🔧 Profile Management Best Practices");
    println!("====================================");
    println!("✅ Profile Customization:");
    println!("   • Provide role-based default preferences");
    println!("   • Allow users to customize their workspace");
    println!("   • Store kitchen-specific settings (station, shift)");
    println!("   • Enable theme and accessibility options");
    
    println!("\n✅ Data Management:");
    println!("   • Validate profile data before storage");
    println!("   • Provide sensible defaults for missing preferences");
    println!("   • Track profile completeness and encourage completion");
    println!("   • Maintain audit trail of profile changes");
    
    println!("\n✅ Kitchen-Specific Features:");
    println!("   • Station assignment and preferences");
    println!("   • Shift scheduling integration");
    println!("   • Role-based dashboard layouts");
    println!("   • Quick action customization");
    println!("   • Temperature and time format preferences");
    
    println!("\n📊 Profile Analytics Insights");
    println!("=============================");
    println!("💡 Monitoring Opportunities:");
    println!("   • Track profile completion rates");
    println!("   • Analyze preference patterns by role");
    println!("   • Monitor user engagement with customization");
    println!("   • Identify popular features and settings");
    
    println!("\n🔮 Future Enhancements");
    println!("======================");
    println!("💡 Advanced Features:");
    println!("   • Profile import/export functionality");
    println!("   • Team-based preference sharing");
    println!("   • Automated preference suggestions");
    println!("   • Integration with external systems");
    println!("   • Mobile app profile synchronization");
    
    println!("\n🎉 Profile Management Example Completed!");
    println!("========================================");
    println!("✅ Profile retrieval and analysis demonstrated");
    println!("✅ Role-based customization patterns shown");
    println!("✅ Validation and error handling tested");
    println!("✅ Profile monitoring capabilities demonstrated");
    println!("\n💡 Next Steps:");
    println!("   - Implement profile update endpoints");
    println!("   - Add preference validation and defaults");
    println!("   - Run 'cargo run --example full_workflow' for complete integration");
    println!("   - Create role-based preference templates");
    
    Ok(())
}