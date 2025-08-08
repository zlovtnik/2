//! Complete Kitchen Management Workflow Example
//!
//! This example demonstrates a comprehensive kitchen management workflow that
//! integrates multiple API endpoints to simulate real-world kitchen operations:
//! 1. Staff authentication and onboarding
//! 2. User profile and preference management
//! 3. System health monitoring
//! 4. Multi-user coordination workflows
//! 5. Error handling and recovery patterns
//!
//! # Kitchen Management Context
//!
//! This workflow simulates a complete day in a restaurant kitchen, from staff
//! login and setup through various operational tasks, demonstrating how all
//! API components work together in a real kitchen environment.

use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
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

/// Kitchen staff member with role and authentication
#[derive(Debug, Clone)]
struct KitchenStaff {
    email: String,
    password: String,
    full_name: String,
    role: String,
    station: String,
    shift: String,
    token: Option<String>,
    profile: Option<Value>,
    stats: Option<Value>,
}

impl KitchenStaff {
    fn new(email: &str, password: &str, full_name: &str, role: &str, station: &str, shift: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
            full_name: full_name.to_string(),
            role: role.to_string(),
            station: station.to_string(),
            shift: shift.to_string(),
            token: None,
            profile: None,
            stats: None,
        }
    }
    
    /// Authenticate staff member
    async fn authenticate(&mut self, config: &ApiConfig) -> Result<(), Box<dyn Error>> {
        println!("🔐 Authenticating {} ({})", self.full_name, self.role);
        
        // Try registration first
        let registration_data = json!({
            "email": self.email,
            "password": self.password,
            "full_name": self.full_name
        });
        
        let token = match config.client
            .post(&format!("{}/api/v1/auth/register", config.base_url))
            .json(&registration_data)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let token_response: Value = response.json().await?;
                println!("✅ {} registered successfully", self.full_name);
                token_response["token"].as_str().unwrap().to_string()
            }
            _ => {
                // Registration failed, try login
                let login_data = json!({
                    "email": self.email,
                    "password": self.password
                });
                
                let response = config.client
                    .post(&format!("{}/api/v1/auth/login", config.base_url))
                    .json(&login_data)
                    .send()
                    .await?;
                
                if response.status().is_success() {
                    let token_response: Value = response.json().await?;
                    println!("✅ {} logged in successfully", self.full_name);
                    token_response["token"].as_str().unwrap().to_string()
                } else {
                    return Err(format!("Authentication failed for {}", self.email).into());
                }
            }
        };
        
        self.token = Some(token);
        Ok(())
    }
    
    /// Load user profile
    async fn load_profile(&mut self, config: &ApiConfig) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = config.client
            .get(&format!("{}/api/v1/user/profile", config.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let profile: Value = response.json().await?;
            self.profile = Some(profile);
            println!("📋 Profile loaded for {}", self.full_name);
            Ok(())
        } else {
            Err(format!("Failed to load profile for {}", self.full_name).into())
        }
    }
    
    /// Load user statistics
    async fn load_statistics(&mut self, config: &ApiConfig) -> Result<(), Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = config.client
            .get(&format!("{}/api/v1/user/stats", config.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let stats: Value = response.json().await?;
            self.stats = Some(stats);
            println!("📊 Statistics loaded for {}", self.full_name);
            Ok(())
        } else {
            Err(format!("Failed to load statistics for {}", self.full_name).into())
        }
    }
    
    /// Check system health
    async fn check_system_health(&self, config: &ApiConfig) -> Result<Value, Box<dyn Error>> {
        let response = config.client
            .get(&format!("{}/health/ready", config.base_url))
            .send()
            .await?;
        
        let health: Value = response.json().await?;
        Ok(health)
    }
    
    /// Perform role-specific morning tasks
    async fn perform_morning_tasks(&self, config: &ApiConfig) -> Result<(), Box<dyn Error>> {
        println!("🌅 {} starting morning tasks at {}", self.full_name, self.station);
        
        match self.role.as_str() {
            "Head Chef" => {
                println!("   📋 Reviewing daily menu and specials");
                println!("   👥 Checking staff assignments");
                println!("   📊 Analyzing yesterday's performance");
                
                // Check system health as head chef
                match self.check_system_health(config).await {
                    Ok(health) => {
                        println!("   ✅ System health check: {}", health.get("status").unwrap_or(&json!("unknown")));
                    }
                    Err(e) => {
                        println!("   ⚠️  System health check failed: {}", e);
                    }
                }
            }
            "Sous Chef" => {
                println!("   📝 Organizing prep lists");
                println!("   🥘 Coordinating with line cooks");
                println!("   📦 Checking inventory deliveries");
            }
            "Line Cook" => {
                println!("   🔥 Setting up {} station", self.station);
                println!("   📋 Reviewing today's orders");
                println!("   🧄 Starting prep work");
            }
            "Prep Cook" => {
                println!("   🥕 Preparing vegetables for the day");
                println!("   🍖 Portioning proteins");
                println!("   📦 Organizing ingredients");
            }
            _ => {
                println!("   📋 Performing general morning setup");
            }
        }
        
        Ok(())
    }
    
    /// Display staff summary
    fn display_summary(&self) {
        println!("\n👤 Staff Summary: {}", self.full_name);
        println!("   📧 Email: {}", self.email);
        println!("   🎭 Role: {}", self.role);
        println!("   🍳 Station: {}", self.station);
        println!("   ⏰ Shift: {}", self.shift);
        println!("   🔐 Authenticated: {}", if self.token.is_some() { "Yes" } else { "No" });
        
        if let Some(profile) = &self.profile {
            if let Some(created_at) = profile.get("created_at") {
                println!("   📅 Account Created: {}", created_at);
            }
        }
        
        if let Some(stats) = &self.stats {
            if let Some(token_count) = stats.get("refresh_token_count") {
                println!("   🔄 Token Count: {}", token_count);
            }
        }
    }
}

/// Kitchen management system coordinator
struct KitchenSystem {
    config: ApiConfig,
    staff: Vec<KitchenStaff>,
    system_status: HashMap<String, String>,
}

impl KitchenSystem {
    fn new() -> Self {
        Self {
            config: ApiConfig::new(),
            staff: Vec::new(),
            system_status: HashMap::new(),
        }
    }
    
    /// Add staff member to the system
    fn add_staff(&mut self, staff: KitchenStaff) {
        self.staff.push(staff);
    }
    
    /// Initialize all staff members
    async fn initialize_staff(&mut self) -> Result<(), Box<dyn Error>> {
        println!("👥 Initializing Kitchen Staff");
        println!("=============================");
        
        for staff in &mut self.staff {
            // Authenticate
            if let Err(e) = staff.authenticate(&self.config).await {
                println!("❌ Failed to authenticate {}: {}", staff.full_name, e);
                continue;
            }
            
            sleep(Duration::from_millis(200)).await;
            
            // Load profile
            if let Err(e) = staff.load_profile(&self.config).await {
                println!("⚠️  Failed to load profile for {}: {}", staff.full_name, e);
            }
            
            sleep(Duration::from_millis(200)).await;
            
            // Load statistics
            if let Err(e) = staff.load_statistics(&self.config).await {
                println!("⚠️  Failed to load statistics for {}: {}", staff.full_name, e);
            }
            
            sleep(Duration::from_millis(200)).await;
        }
        
        println!("✅ Staff initialization completed");
        Ok(())
    }
    
    /// Perform system health checks
    async fn perform_system_checks(&mut self) -> Result<(), Box<dyn Error>> {
        println!("\n🏥 System Health Checks");
        println!("=======================");
        
        // Check liveness
        println!("🔍 Checking system liveness...");
        let liveness_response = self.config.client
            .get(&format!("{}/health/live", self.config.base_url))
            .send()
            .await?;
        
        if liveness_response.status().is_success() {
            let liveness_text = liveness_response.text().await?;
            println!("✅ Liveness check: {}", liveness_text);
            self.system_status.insert("liveness".to_string(), "ok".to_string());
        } else {
            println!("❌ Liveness check failed");
            self.system_status.insert("liveness".to_string(), "error".to_string());
        }
        
        sleep(Duration::from_millis(500)).await;
        
        // Check readiness
        println!("🔍 Checking system readiness...");
        let readiness_response = self.config.client
            .get(&format!("{}/health/ready", self.config.base_url))
            .send()
            .await?;
        
        if readiness_response.status().is_success() {
            let readiness_data: Value = readiness_response.json().await?;
            println!("✅ Readiness check: {}", readiness_data.get("status").unwrap_or(&json!("unknown")));
            println!("   Database: {}", readiness_data.get("database").unwrap_or(&json!("unknown")));
            self.system_status.insert("readiness".to_string(), "ok".to_string());
        } else {
            println!("❌ Readiness check failed");
            self.system_status.insert("readiness".to_string(), "error".to_string());
        }
        
        Ok(())
    }
    
    /// Simulate morning shift startup
    async fn simulate_morning_startup(&mut self) -> Result<(), Box<dyn Error>> {
        println!("\n🌅 Morning Shift Startup Simulation");
        println!("===================================");
        
        // Staff arrive and perform morning tasks
        for staff in &self.staff {
            if staff.shift == "Morning" || staff.shift == "All Day" {
                staff.perform_morning_tasks(&self.config).await?;
                sleep(Duration::from_millis(300)).await;
            }
        }
        
        Ok(())
    }
    
    /// Simulate operational workflow
    async fn simulate_operational_workflow(&mut self) -> Result<(), Box<dyn Error>> {
        println!("\n🍽️  Operational Workflow Simulation");
        println!("===================================");
        
        // Simulate various kitchen operations
        let operations = vec![
            "Order received - Grilled Salmon with vegetables",
            "Inventory check - Low on salmon, ordering more",
            "Quality control - Checking food temperatures",
            "Staff coordination - Adjusting station assignments",
            "Customer feedback - Positive review on pasta dish",
        ];
        
        for (i, operation) in operations.iter().enumerate() {
            println!("\n🔄 Operation {}: {}", i + 1, operation);
            
            // Assign operation to appropriate staff
            match i % 4 {
                0 => {
                    // Order handling - Line Cook
                    if let Some(line_cook) = self.staff.iter().find(|s| s.role == "Line Cook") {
                        println!("   👨‍🍳 Assigned to: {} at {}", line_cook.full_name, line_cook.station);
                        println!("   📋 Status: Processing order");
                    }
                }
                1 => {
                    // Inventory - Sous Chef
                    if let Some(sous_chef) = self.staff.iter().find(|s| s.role == "Sous Chef") {
                        println!("   👩‍🍳 Assigned to: {} at {}", sous_chef.full_name, sous_chef.station);
                        println!("   📦 Status: Managing inventory");
                    }
                }
                2 => {
                    // Quality control - Head Chef
                    if let Some(head_chef) = self.staff.iter().find(|s| s.role == "Head Chef") {
                        println!("   👨‍🍳 Assigned to: {} at {}", head_chef.full_name, head_chef.station);
                        println!("   🌡️  Status: Quality inspection");
                    }
                }
                3 => {
                    // Staff coordination - Head Chef
                    if let Some(head_chef) = self.staff.iter().find(|s| s.role == "Head Chef") {
                        println!("   👩‍🍳 Assigned to: {} at {}", head_chef.full_name, head_chef.station);
                        println!("   👥 Status: Staff management");
                    }
                }
                _ => {}
            }
            
            println!("   ✅ Operation completed successfully");
            sleep(Duration::from_secs(1)).await;
        }
        
        Ok(())
    }
    
    /// Generate comprehensive system report
    fn generate_system_report(&self) {
        println!("\n📊 Kitchen Management System Report");
        println!("===================================");
        
        println!("🕐 Report Generated: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        
        // System status
        println!("\n🏥 System Status:");
        for (component, status) in &self.system_status {
            let status_icon = if status == "ok" { "✅" } else { "❌" };
            println!("   {} {}: {}", status_icon, component, status);
        }
        
        // Staff summary
        println!("\n👥 Staff Summary:");
        println!("   Total Staff: {}", self.staff.len());
        
        let authenticated_count = self.staff.iter().filter(|s| s.token.is_some()).count();
        println!("   Authenticated: {} ({:.1}%)", 
                authenticated_count, 
                (authenticated_count as f64 / self.staff.len() as f64) * 100.0);
        
        let profile_loaded_count = self.staff.iter().filter(|s| s.profile.is_some()).count();
        println!("   Profiles Loaded: {} ({:.1}%)", 
                profile_loaded_count, 
                (profile_loaded_count as f64 / self.staff.len() as f64) * 100.0);
        
        let stats_loaded_count = self.staff.iter().filter(|s| s.stats.is_some()).count();
        println!("   Statistics Loaded: {} ({:.1}%)", 
                stats_loaded_count, 
                (stats_loaded_count as f64 / self.staff.len() as f64) * 100.0);
        
        // Role distribution
        let mut role_counts = HashMap::new();
        for staff in &self.staff {
            *role_counts.entry(&staff.role).or_insert(0) += 1;
        }
        
        println!("\n🎭 Role Distribution:");
        for (role, count) in &role_counts {
            println!("   {}: {} staff", role, count);
        }
        
        // Shift distribution
        let mut shift_counts = HashMap::new();
        for staff in &self.staff {
            *shift_counts.entry(&staff.shift).or_insert(0) += 1;
        }
        
        println!("\n⏰ Shift Distribution:");
        for (shift, count) in &shift_counts {
            println!("   {}: {} staff", shift, count);
        }
        
        // Station assignments
        let mut station_counts = HashMap::new();
        for staff in &self.staff {
            *station_counts.entry(&staff.station).or_insert(0) += 1;
        }
        
        println!("\n🍳 Station Assignments:");
        for (station, count) in &station_counts {
            println!("   {}: {} staff", station, count);
        }
    }
    
    /// Display detailed staff information
    fn display_staff_details(&self) {
        println!("\n👥 Detailed Staff Information");
        println!("=============================");
        
        for staff in &self.staff {
            staff.display_summary();
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - Complete Workflow Example");
    println!("=======================================================");
    
    let mut kitchen_system = KitchenSystem::new();
    println!("🌐 Using API base URL: {}", kitchen_system.config.base_url);
    
    // Add kitchen staff with different roles and shifts
    kitchen_system.add_staff(KitchenStaff::new(
        "head.chef.workflow@restaurant.com",
        "HeadChefWorkflow123!",
        "Maria Rodriguez",
        "Head Chef",
        "Main Kitchen",
        "All Day"
    ));
    
    kitchen_system.add_staff(KitchenStaff::new(
        "sous.chef.workflow@restaurant.com",
        "SousChefWorkflow123!",
        "James Wilson",
        "Sous Chef",
        "Main Kitchen",
        "Morning"
    ));
    
    kitchen_system.add_staff(KitchenStaff::new(
        "line.cook.workflow@restaurant.com",
        "LineCookWorkflow123!",
        "Sarah Chen",
        "Line Cook",
        "Grill Station",
        "Evening"
    ));
    
    kitchen_system.add_staff(KitchenStaff::new(
        "prep.cook.workflow@restaurant.com",
        "PrepCookWorkflow123!",
        "Miguel Santos",
        "Prep Cook",
        "Prep Kitchen",
        "Morning"
    ));
    
    kitchen_system.add_staff(KitchenStaff::new(
        "pastry.chef.workflow@restaurant.com",
        "PastryChefWorkflow123!",
        "Emma Thompson",
        "Pastry Chef",
        "Pastry Station",
        "Morning"
    ));
    
    // Initialize the kitchen system
    println!("\n🚀 Kitchen System Initialization");
    println!("================================");
    
    // Perform system health checks
    kitchen_system.perform_system_checks().await?;
    
    // Initialize all staff
    kitchen_system.initialize_staff().await?;
    
    // Simulate morning startup
    kitchen_system.simulate_morning_startup().await?;
    
    // Simulate operational workflow
    kitchen_system.simulate_operational_workflow().await?;
    
    // Display detailed staff information
    kitchen_system.display_staff_details();
    
    // Generate comprehensive system report
    kitchen_system.generate_system_report();
    
    println!("\n🎯 Workflow Integration Summary");
    println!("===============================");
    println!("✅ Authentication: Multi-user login and registration");
    println!("✅ Profile Management: User profile loading and analysis");
    println!("✅ Statistics: User activity and engagement metrics");
    println!("✅ Health Monitoring: System liveness and readiness checks");
    println!("✅ Role-Based Operations: Different tasks by kitchen role");
    println!("✅ Shift Management: Morning startup and operational workflows");
    println!("✅ Error Handling: Graceful handling of API failures");
    println!("✅ Reporting: Comprehensive system and staff analytics");
    
    println!("\n🍳 Kitchen Management Best Practices Demonstrated");
    println!("=================================================");
    println!("✅ Staff Coordination:");
    println!("   • Role-based task assignment and workflow");
    println!("   • Shift-based operations and scheduling");
    println!("   • Station management and resource allocation");
    
    println!("\n✅ System Integration:");
    println!("   • Health monitoring for operational reliability");
    println!("   • User authentication and session management");
    println!("   • Profile and preference synchronization");
    
    println!("\n✅ Operational Excellence:");
    println!("   • Real-time workflow coordination");
    println!("   • Comprehensive reporting and analytics");
    println!("   • Error handling and system resilience");
    
    println!("\n🚀 Production Deployment Considerations");
    println!("=======================================");
    println!("💡 Scalability:");
    println!("   • Implement connection pooling for database operations");
    println!("   • Add caching layers for frequently accessed data");
    println!("   • Use load balancing for high-availability deployments");
    
    println!("\n💡 Security:");
    println!("   • Implement rate limiting and request throttling");
    println!("   • Add comprehensive audit logging");
    println!("   • Use HTTPS for all API communications");
    
    println!("\n💡 Monitoring:");
    println!("   • Set up application performance monitoring");
    println!("   • Implement alerting for system health issues");
    println!("   • Add business metrics tracking and dashboards");
    
    println!("\n🎉 Complete Workflow Example Finished!");
    println!("======================================");
    println!("✅ Full kitchen management workflow demonstrated");
    println!("✅ Multi-user coordination patterns shown");
    println!("✅ System integration best practices applied");
    println!("✅ Production-ready patterns and considerations outlined");
    println!("\n💡 Next Steps:");
    println!("   - Implement additional kitchen-specific endpoints");
    println!("   - Add real-time communication features (WebSockets)");
    println!("   - Create automated testing suites for workflows");
    println!("   - Deploy to production with monitoring and alerting");
    
    Ok(())
}