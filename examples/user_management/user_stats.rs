//! User Statistics and Analytics Example
//!
//! This example demonstrates user statistics and analytics functionality:
//! 1. Retrieving user statistics via PostgreSQL procedures
//! 2. Analyzing user activity and engagement metrics
//! 3. Generating reports for kitchen management
//! 4. Monitoring user behavior patterns
//!
//! # Kitchen Management Context
//!
//! User statistics help kitchen managers understand staff engagement,
//! system usage patterns, and optimize kitchen operations based on data.

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

/// User statistics data structure
#[derive(Debug)]
struct UserStats {
    user_id: String,
    email: String,
    full_name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    refresh_token_count: i64,
    last_login: Option<DateTime<Utc>>,
}

impl UserStats {
    fn from_json(data: &Value) -> Result<Self, Box<dyn Error>> {
        Ok(UserStats {
            user_id: data["user_id"].as_str().ok_or("Missing user_id")?.to_string(),
            email: data["email"].as_str().ok_or("Missing email")?.to_string(),
            full_name: data["full_name"].as_str().ok_or("Missing full_name")?.to_string(),
            created_at: data["created_at"].as_str()
                .ok_or("Missing created_at")?
                .parse()?,
            updated_at: data["updated_at"].as_str()
                .ok_or("Missing updated_at")?
                .parse()?,
            refresh_token_count: data["refresh_token_count"].as_i64().unwrap_or(0),
            last_login: data["last_login"].as_str()
                .and_then(|s| s.parse().ok()),
        })
    }
    
    /// Calculate account age in days
    fn account_age_days(&self) -> i64 {
        let now = Utc::now();
        (now - self.created_at).num_days()
    }
    
    /// Calculate days since last update
    fn days_since_update(&self) -> i64 {
        let now = Utc::now();
        (now - self.updated_at).num_days()
    }
    
    /// Calculate days since last login (if available)
    fn days_since_last_login(&self) -> Option<i64> {
        self.last_login.map(|login| {
            let now = Utc::now();
            (now - login).num_days()
        })
    }
    
    /// Determine user activity level
    fn activity_level(&self) -> &'static str {
        match self.refresh_token_count {
            0 => "Inactive",
            1..=5 => "Low",
            6..=20 => "Moderate",
            21..=50 => "High",
            _ => "Very High",
        }
    }
    
    /// Check if user needs attention (inactive or issues)
    fn needs_attention(&self) -> bool {
        self.refresh_token_count == 0 || 
        self.days_since_update() > 30 ||
        self.days_since_last_login().unwrap_or(0) > 7
    }
}

/// Create and authenticate a test user
async fn create_test_user(config: &ApiConfig, email: &str, password: &str, full_name: &str) -> Result<String, Box<dyn Error>> {
    println!("👤 Creating test user: {}", full_name);
    
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

/// Get user statistics from the API
async fn get_user_statistics(config: &ApiConfig, token: &str) -> Result<UserStats, Box<dyn Error>> {
    println!("📊 Retrieving user statistics");
    
    let response = config.client
        .get(&format!("{}/api/v1/user/stats", config.base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    match response.status() {
        reqwest::StatusCode::OK => {
            let stats_data: Value = response.json().await?;
            println!("✅ User statistics retrieved successfully");
            
            let stats = UserStats::from_json(&stats_data)?;
            Ok(stats)
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            println!("❌ Unauthorized - Invalid token");
            Err("Authentication failed".into())
        }
        reqwest::StatusCode::NOT_FOUND => {
            println!("❌ User not found");
            Err("User not found".into())
        }
        status => {
            let error_text = response.text().await?;
            println!("❌ Failed to get statistics with status: {}", status);
            println!("   Error: {}", error_text);
            Err(format!("Statistics request failed: {}", status).into())
        }
    }
}

/// Display detailed user statistics
fn display_user_statistics(stats: &UserStats) {
    println!("\n📈 Detailed User Statistics");
    println!("===========================");
    
    println!("👤 User Information:");
    println!("   ID: {}", stats.user_id);
    println!("   Email: {}", stats.email);
    println!("   Full Name: {}", stats.full_name);
    
    println!("\n📅 Account Timeline:");
    println!("   Created: {}", stats.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("   Last Updated: {}", stats.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("   Account Age: {} days", stats.account_age_days());
    println!("   Days Since Update: {}", stats.days_since_update());
    
    if let Some(last_login) = stats.last_login {
        println!("   Last Login: {}", last_login.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(days) = stats.days_since_last_login() {
            println!("   Days Since Login: {}", days);
        }
    } else {
        println!("   Last Login: Never recorded");
    }
    
    println!("\n🔄 Activity Metrics:");
    println!("   Refresh Token Count: {}", stats.refresh_token_count);
    println!("   Activity Level: {}", stats.activity_level());
    println!("   Needs Attention: {}", if stats.needs_attention() { "Yes" } else { "No" });
    
    // Kitchen-specific insights
    println!("\n🍳 Kitchen Management Insights:");
    match stats.activity_level() {
        "Inactive" => {
            println!("   • User has not been active - may need onboarding support");
            println!("   • Consider reaching out for training or system access issues");
        }
        "Low" => {
            println!("   • User shows minimal system engagement");
            println!("   • May benefit from additional training or role clarification");
        }
        "Moderate" => {
            println!("   • User shows healthy system engagement");
            println!("   • Good balance of system usage for their role");
        }
        "High" => {
            println!("   • User is very active in the system");
            println!("   • May be a power user or in a high-responsibility role");
        }
        "Very High" => {
            println!("   • Extremely high system usage");
            println!("   • May indicate heavy workload or system dependency");
        }
        _ => {}
    }
    
    if stats.needs_attention() {
        println!("\n⚠️  Attention Required:");
        if stats.refresh_token_count == 0 {
            println!("   • No refresh tokens - user may not be actively using the system");
        }
        if stats.days_since_update() > 30 {
            println!("   • Profile not updated in over 30 days - may need data refresh");
        }
        if let Some(days) = stats.days_since_last_login() {
            if days > 7 {
                println!("   • No login in {} days - user may be inactive", days);
            }
        }
    }
}

/// Generate user activity report
fn generate_activity_report(all_stats: &[UserStats]) {
    println!("\n📋 Kitchen Staff Activity Report");
    println!("================================");
    
    let total_users = all_stats.len();
    let active_users = all_stats.iter().filter(|s| s.refresh_token_count > 0).count();
    let inactive_users = total_users - active_users;
    let users_needing_attention = all_stats.iter().filter(|s| s.needs_attention()).count();
    
    println!("📊 Summary Statistics:");
    println!("   Total Users: {}", total_users);
    println!("   Active Users: {} ({:.1}%)", active_users, (active_users as f64 / total_users as f64) * 100.0);
    println!("   Inactive Users: {} ({:.1}%)", inactive_users, (inactive_users as f64 / total_users as f64) * 100.0);
    println!("   Users Needing Attention: {} ({:.1}%)", users_needing_attention, (users_needing_attention as f64 / total_users as f64) * 100.0);
    
    // Activity level distribution
    let mut activity_counts = std::collections::HashMap::new();
    for stats in all_stats {
        *activity_counts.entry(stats.activity_level()).or_insert(0) += 1;
    }
    
    println!("\n📈 Activity Level Distribution:");
    for (level, count) in &activity_counts {
        let percentage = (*count as f64 / total_users as f64) * 100.0;
        println!("   {}: {} users ({:.1}%)", level, count, percentage);
    }
    
    // Average metrics
    let avg_age = all_stats.iter().map(|s| s.account_age_days()).sum::<i64>() as f64 / total_users as f64;
    let avg_tokens = all_stats.iter().map(|s| s.refresh_token_count).sum::<i64>() as f64 / total_users as f64;
    
    println!("\n📊 Average Metrics:");
    println!("   Average Account Age: {:.1} days", avg_age);
    println!("   Average Refresh Tokens: {:.1}", avg_tokens);
    
    // Users needing attention
    if users_needing_attention > 0 {
        println!("\n⚠️  Users Requiring Attention:");
        for stats in all_stats.iter().filter(|s| s.needs_attention()) {
            println!("   • {} ({})", stats.full_name, stats.email);
            if stats.refresh_token_count == 0 {
                print!("     - No activity");
            }
            if stats.days_since_update() > 30 {
                print!("     - Stale profile");
            }
            if let Some(days) = stats.days_since_last_login() {
                if days > 7 {
                    print!("     - {} days since login", days);
                }
            }
            println!();
        }
    }
}

/// Demonstrate statistics comparison between users
async fn demonstrate_statistics_comparison(config: &ApiConfig) -> Result<(), Box<dyn Error>> {
    println!("\n👥 Multi-User Statistics Comparison");
    println!("===================================");
    
    // Create multiple test users with different activity patterns
    let test_users = vec![
        ("stats.chef@restaurant.com", "ChefPass123!", "Statistics Chef"),
        ("stats.cook@restaurant.com", "CookPass123!", "Statistics Cook"),
        ("stats.manager@restaurant.com", "ManagerPass123!", "Statistics Manager"),
    ];
    
    let mut all_stats = Vec::new();
    
    for (email, password, name) in &test_users {
        match create_test_user(config, email, password, name).await {
            Ok(token) => {
                match get_user_statistics(config, &token).await {
                    Ok(stats) => {
                        println!("\n📊 Statistics for {}:", name);
                        display_user_statistics(&stats);
                        all_stats.push(stats);
                    }
                    Err(e) => {
                        println!("❌ Failed to get statistics for {}: {}", name, e);
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
    
    if !all_stats.is_empty() {
        generate_activity_report(&all_stats);
    }
    
    Ok(())
}

/// Demonstrate real-time statistics monitoring
async fn demonstrate_statistics_monitoring(config: &ApiConfig, token: &str) -> Result<(), Box<dyn Error>> {
    println!("\n⏱️  Real-time Statistics Monitoring");
    println!("==================================");
    
    println!("📊 Monitoring user statistics over time...");
    
    for i in 1..=3 {
        println!("\n🔄 Monitoring cycle {} of 3:", i);
        
        match get_user_statistics(config, token).await {
            Ok(stats) => {
                println!("   ✅ Statistics retrieved successfully");
                println!("   📈 Refresh Token Count: {}", stats.refresh_token_count);
                println!("   🕐 Last Updated: {}", stats.updated_at.format("%H:%M:%S"));
                println!("   📊 Activity Level: {}", stats.activity_level());
                
                if stats.needs_attention() {
                    println!("   ⚠️  User needs attention");
                } else {
                    println!("   ✅ User status normal");
                }
            }
            Err(e) => {
                println!("   ❌ Failed to retrieve statistics: {}", e);
            }
        }
        
        if i < 3 {
            println!("   ⏳ Waiting 2 seconds before next check...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
    
    println!("✅ Monitoring cycle completed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - User Statistics Example");
    println!("====================================================");
    
    let config = ApiConfig::new();
    println!("🌐 Using API base URL: {}", config.base_url);
    
    // Create a primary test user for detailed statistics
    let primary_user_token = create_test_user(
        &config,
        "primary.stats@restaurant.com",
        "PrimaryStatsPass123!",
        "Primary Statistics User"
    ).await?;
    
    println!("\n📊 Primary User Statistics Analysis");
    println!("===================================");
    
    // Get and display detailed statistics for primary user
    let primary_stats = get_user_statistics(&config, &primary_user_token).await?;
    display_user_statistics(&primary_stats);
    
    // Demonstrate statistics comparison across multiple users
    demonstrate_statistics_comparison(&config).await?;
    
    // Demonstrate real-time monitoring
    demonstrate_statistics_monitoring(&config, &primary_user_token).await?;
    
    println!("\n📈 Statistics API Features Demonstrated");
    println!("=======================================");
    println!("✅ User statistics retrieval via PostgreSQL procedures");
    println!("✅ Comprehensive activity metrics analysis");
    println!("✅ Multi-user statistics comparison");
    println!("✅ Real-time statistics monitoring");
    println!("✅ Kitchen management insights generation");
    
    println!("\n🍳 Kitchen Management Applications");
    println!("==================================");
    println!("✅ Staff Performance Monitoring:");
    println!("   • Track system engagement and usage patterns");
    println!("   • Identify users needing additional training");
    println!("   • Monitor staff productivity and system adoption");
    
    println!("\n✅ Operational Insights:");
    println!("   • Understand peak usage times and patterns");
    println!("   • Identify system bottlenecks and user pain points");
    println!("   • Optimize kitchen workflows based on usage data");
    
    println!("\n✅ Management Reporting:");
    println!("   • Generate staff activity reports for management");
    println!("   • Track user onboarding success and engagement");
    println!("   • Provide data-driven insights for kitchen operations");
    
    println!("\n📊 Advanced Analytics Recommendations");
    println!("====================================");
    println!("💡 Future Enhancements:");
    println!("   • Add time-series data for trend analysis");
    println!("   • Implement user behavior pattern recognition");
    println!("   • Create predictive analytics for staff needs");
    println!("   • Add role-based performance benchmarking");
    println!("   • Implement automated alerting for inactive users");
    
    println!("\n🎉 User Statistics Example Completed!");
    println!("=====================================");
    println!("✅ Statistics retrieval and analysis demonstrated");
    println!("✅ Multi-user comparison workflows shown");
    println!("✅ Real-time monitoring capabilities tested");
    println!("✅ Kitchen management insights provided");
    println!("\n💡 Next Steps:");
    println!("   - Run 'cargo run --example profile_management' for profile operations");
    println!("   - Run 'cargo run --example full_workflow' for complete integration");
    println!("   - Implement custom analytics dashboards");
    println!("   - Add automated reporting and alerting systems");
    
    Ok(())
}