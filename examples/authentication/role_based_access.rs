//! Role-Based Access Control Example
//!
//! This example demonstrates role-based access patterns for kitchen management:
//! 1. Different user roles and their permissions
//! 2. Role-based API access patterns
//! 3. Permission validation and error handling
//! 4. Kitchen hierarchy and access control
//!
//! # Kitchen Management Roles
//!
//! - **Head Chef**: Full access to all kitchen operations
//! - **Sous Chef**: Management access, limited administrative functions
//! - **Line Cook**: Operational access to assigned stations
//! - **Prep Cook**: Limited access to preparation tasks
//! - **Kitchen Manager**: Administrative access, staff management

use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::env;
use std::collections::HashMap;

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

/// Kitchen staff role definitions
#[derive(Debug, Clone)]
enum KitchenRole {
    HeadChef,
    SousChef,
    LineCook,
    PrepCook,
    KitchenManager,
}

impl KitchenRole {
    fn as_str(&self) -> &'static str {
        match self {
            KitchenRole::HeadChef => "Head Chef",
            KitchenRole::SousChef => "Sous Chef",
            KitchenRole::LineCook => "Line Cook",
            KitchenRole::PrepCook => "Prep Cook",
            KitchenRole::KitchenManager => "Kitchen Manager",
        }
    }
    
    /// Get permissions for this role
    fn permissions(&self) -> Vec<&'static str> {
        match self {
            KitchenRole::HeadChef => vec![
                "menu:read", "menu:write", "menu:delete",
                "orders:read", "orders:write", "orders:manage",
                "inventory:read", "inventory:write", "inventory:manage",
                "staff:read", "staff:write", "staff:manage",
                "reports:read", "reports:generate",
                "system:admin"
            ],
            KitchenRole::SousChef => vec![
                "menu:read", "menu:write",
                "orders:read", "orders:write", "orders:manage",
                "inventory:read", "inventory:write",
                "staff:read", "staff:write",
                "reports:read"
            ],
            KitchenRole::LineCook => vec![
                "menu:read",
                "orders:read", "orders:write",
                "inventory:read",
                "staff:read"
            ],
            KitchenRole::PrepCook => vec![
                "menu:read",
                "orders:read",
                "inventory:read"
            ],
            KitchenRole::KitchenManager => vec![
                "menu:read",
                "orders:read", "orders:manage",
                "inventory:read", "inventory:manage",
                "staff:read", "staff:write", "staff:manage",
                "reports:read", "reports:generate",
                "system:admin"
            ],
        }
    }
    
    /// Check if role has specific permission
    fn has_permission(&self, permission: &str) -> bool {
        self.permissions().contains(&permission)
    }
}

/// Kitchen staff member with role and authentication
#[derive(Debug)]
struct KitchenStaff {
    email: String,
    password: String,
    full_name: String,
    role: KitchenRole,
    token: Option<String>,
}

impl KitchenStaff {
    fn new(email: &str, password: &str, full_name: &str, role: KitchenRole) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
            full_name: full_name.to_string(),
            role,
            token: None,
        }
    }
    
    /// Authenticate and store token
    async fn authenticate(&mut self, config: &ApiConfig) -> Result<(), Box<dyn Error>> {
        println!("🔐 Authenticating {} ({})", self.full_name, self.role.as_str());
        
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
                println!("✅ Registered new staff member: {}", self.full_name);
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
                    println!("✅ Logged in existing staff member: {}", self.full_name);
                    token_response["token"].as_str().unwrap().to_string()
                } else {
                    return Err(format!("Authentication failed for {}", self.email).into());
                }
            }
        };
        
        self.token = Some(token);
        Ok(())
    }
    
    /// Make authenticated request
    async fn make_request(&self, config: &ApiConfig, method: &str, endpoint: &str, body: Option<Value>) -> Result<Value, Box<dyn Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let mut request = match method.to_uppercase().as_str() {
            "GET" => config.client.get(&format!("{}{}", config.base_url, endpoint)),
            "POST" => config.client.post(&format!("{}{}", config.base_url, endpoint)),
            "PUT" => config.client.put(&format!("{}{}", config.base_url, endpoint)),
            "DELETE" => config.client.delete(&format!("{}{}", config.base_url, endpoint)),
            _ => return Err("Unsupported HTTP method".into()),
        };
        
        request = request.header("Authorization", format!("Bearer {}", token));
        
        if let Some(json_body) = body {
            request = request.json(&json_body);
        }
        
        let response = request.send().await?;
        let status = response.status();
        
        if status.is_success() {
            if status == reqwest::StatusCode::NO_CONTENT {
                Ok(json!({"status": "success"}))
            } else {
                let data: Value = response.json().await?;
                Ok(data)
            }
        } else {
            let error_text = response.text().await?;
            Err(format!("Request failed ({}): {}", status, error_text).into())
        }
    }
}

/// Test access to different API endpoints based on role
async fn test_role_based_access(config: &ApiConfig, staff: &KitchenStaff) -> Result<(), Box<dyn Error>> {
    println!("\n🔍 Testing API Access for {} ({})", staff.full_name, staff.role.as_str());
    println!("{}=", "=".repeat(50));
    
    // Define test endpoints with their required permissions
    let test_endpoints = vec![
        ("GET", "/api/v1/user/profile", None, "staff:read", "View own profile"),
        ("GET", "/api/v1/user/stats", None, "staff:read", "View own statistics"),
        ("GET", "/health/ready", None, "system:read", "Check system health"),
        ("GET", "/health/live", None, "system:read", "Check system liveness"),
    ];
    
    for (method, endpoint, body, required_permission, description) in test_endpoints {
        let has_permission = staff.role.has_permission(required_permission);
        
        print!("📡 {} {}: {} - ", method, endpoint, description);
        
        match staff.make_request(config, method, endpoint, body).await {
            Ok(data) => {
                println!("✅ SUCCESS");
                
                // Show relevant response data
                match endpoint {
                    "/api/v1/user/profile" => {
                        if let Some(email) = data.get("email") {
                            println!("     Email: {}", email);
                        }
                    }
                    "/api/v1/user/stats" => {
                        if let Some(count) = data.get("refresh_token_count") {
                            println!("     Refresh Token Count: {}", count);
                        }
                    }
                    "/health/ready" => {
                        if let Some(status) = data.get("status") {
                            println!("     System Status: {}", status);
                        }
                    }
                    "/health/live" => {
                        println!("     System: Live");
                    }
                    _ => {}
                }
                
                if !has_permission {
                    println!("     ⚠️  Access granted despite missing permission: {}", required_permission);
                }
            }
            Err(e) => {
                if has_permission {
                    println!("❌ FAILED (unexpected): {}", e);
                } else {
                    println!("🚫 DENIED (expected): {}", e);
                }
            }
        }
    }
    
    Ok(())
}

/// Demonstrate permission checking logic
fn demonstrate_permission_system() {
    println!("\n🔐 Kitchen Role Permission Matrix");
    println!("=================================");
    
    let roles = vec![
        KitchenRole::HeadChef,
        KitchenRole::SousChef,
        KitchenRole::LineCook,
        KitchenRole::PrepCook,
        KitchenRole::KitchenManager,
    ];
    
    let permissions = vec![
        "menu:read", "menu:write", "menu:delete",
        "orders:read", "orders:write", "orders:manage",
        "inventory:read", "inventory:write", "inventory:manage",
        "staff:read", "staff:write", "staff:manage",
        "reports:read", "reports:generate",
        "system:admin"
    ];
    
    // Print header
    print!("{:<15}", "Role");
    for permission in &permissions {
        print!("{:<12}", permission.split(':').next().unwrap_or(permission));
    }
    println!();
    
    print!("{:<15}", "");
    for permission in &permissions {
        let action = permission.split(':').nth(1).unwrap_or("");
        print!("{:<12}", action);
    }
    println!();
    
    println!("{}", "-".repeat(15 + permissions.len() * 12));
    
    // Print permission matrix
    for role in &roles {
        print!("{:<15}", role.as_str());
        for permission in &permissions {
            let has_perm = role.has_permission(permission);
            print!("{:<12}", if has_perm { "✅" } else { "❌" });
        }
        println!();
    }
}

/// Simulate kitchen workflow with different roles
async fn simulate_kitchen_workflow(config: &ApiConfig, staff_members: &[KitchenStaff]) -> Result<(), Box<dyn Error>> {
    println!("\n🍳 Kitchen Workflow Simulation");
    println!("==============================");
    
    // Simulate morning prep workflow
    println!("\n📅 Morning Prep Workflow:");
    println!("-------------------------");
    
    for staff in staff_members {
        println!("\n👤 {} ({}) starting morning tasks:", staff.full_name, staff.role.as_str());
        
        // Each role has different morning responsibilities
        match staff.role {
            KitchenRole::HeadChef => {
                println!("   🔍 Reviewing daily menu and specials");
                println!("   📊 Checking inventory levels");
                println!("   👥 Assigning staff to stations");
                println!("   📈 Reviewing yesterday's performance reports");
            }
            KitchenRole::SousChef => {
                println!("   📋 Checking prep lists");
                println!("   🥘 Coordinating with line cooks");
                println!("   📦 Verifying inventory deliveries");
            }
            KitchenRole::LineCook => {
                println!("   🔥 Setting up cooking station");
                println!("   📝 Reviewing today's orders");
                println!("   🧄 Starting prep work");
            }
            KitchenRole::PrepCook => {
                println!("   🥕 Preparing vegetables");
                println!("   🍖 Portioning proteins");
                println!("   📦 Organizing ingredients");
            }
            KitchenRole::KitchenManager => {
                println!("   📊 Reviewing staff schedules");
                println!("   💰 Checking cost reports");
                println!("   📞 Coordinating with suppliers");
            }
        }
        
        // Test profile access (all roles should have this)
        match staff.make_request(config, "GET", "/api/v1/user/profile", None).await {
            Ok(_) => println!("   ✅ Accessed personal profile"),
            Err(e) => println!("   ❌ Failed to access profile: {}", e),
        }
    }
    
    Ok(())
}

/// Demonstrate access control violations and proper error handling
async fn demonstrate_access_violations(config: &ApiConfig, staff: &KitchenStaff) -> Result<(), Box<dyn Error>> {
    println!("\n🚨 Access Control Violation Testing");
    println!("===================================");
    println!("Testing with {} ({})", staff.full_name, staff.role.as_str());
    
    // Test endpoints that might require higher permissions
    let restricted_tests = vec![
        ("Attempting to access system admin functions", "GET", "/api/admin/system", "system:admin"),
        ("Attempting to manage all staff", "GET", "/api/admin/staff", "staff:manage"),
        ("Attempting to generate reports", "GET", "/api/reports/daily", "reports:generate"),
    ];
    
    for (description, method, endpoint, required_permission) in restricted_tests {
        println!("\n🔍 {}", description);
        let has_permission = staff.role.has_permission(required_permission);
        
        println!("   Required permission: {}", required_permission);
        println!("   Role has permission: {}", has_permission);
        
        // Note: These endpoints don't exist in the current API, so they'll return 404
        // In a real implementation, they would return 403 Forbidden for unauthorized access
        match staff.make_request(config, method, endpoint, None).await {
            Ok(_) => {
                if has_permission {
                    println!("   ✅ Access granted (authorized)");
                } else {
                    println!("   ⚠️  Access granted (should be denied!)");
                }
            }
            Err(e) => {
                if has_permission {
                    println!("   ❌ Access denied (should be granted): {}", e);
                } else {
                    println!("   ✅ Access properly denied: {}", e);
                }
            }
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🍽️  Kitchen Management API - Role-Based Access Control Example");
    println!("===============================================================");
    
    let config = ApiConfig::new();
    println!("🌐 Using API base URL: {}", config.base_url);
    
    // Create staff members with different roles
    let mut staff_members = vec![
        KitchenStaff::new(
            "head.chef@restaurant.com",
            "HeadChefPass123!",
            "Maria Rodriguez",
            KitchenRole::HeadChef
        ),
        KitchenStaff::new(
            "sous.chef@restaurant.com",
            "SousChefPass123!",
            "James Wilson",
            KitchenRole::SousChef
        ),
        KitchenStaff::new(
            "line.cook@restaurant.com",
            "LineCookPass123!",
            "Sarah Chen",
            KitchenRole::LineCook
        ),
        KitchenStaff::new(
            "prep.cook@restaurant.com",
            "PrepCookPass123!",
            "Miguel Santos",
            KitchenRole::PrepCook
        ),
        KitchenStaff::new(
            "kitchen.manager@restaurant.com",
            "ManagerPass123!",
            "Lisa Thompson",
            KitchenRole::KitchenManager
        ),
    ];
    
    println!("\n👥 Authenticating Kitchen Staff");
    println!("===============================");
    
    // Authenticate all staff members
    for staff in &mut staff_members {
        if let Err(e) = staff.authenticate(&config).await {
            println!("❌ Failed to authenticate {}: {}", staff.full_name, e);
            continue;
        }
    }
    
    // Show permission matrix
    demonstrate_permission_system();
    
    // Test role-based access for each staff member
    for staff in &staff_members {
        if staff.token.is_some() {
            test_role_based_access(&config, staff).await?;
        }
    }
    
    // Simulate kitchen workflow
    simulate_kitchen_workflow(&config, &staff_members).await?;
    
    // Demonstrate access violations with a lower-privilege user
    if let Some(prep_cook) = staff_members.iter().find(|s| matches!(s.role, KitchenRole::PrepCook)) {
        if prep_cook.token.is_some() {
            demonstrate_access_violations(&config, prep_cook).await?;
        }
    }
    
    println!("\n🔒 Role-Based Access Control Best Practices");
    println!("===========================================");
    println!("✅ Principle of Least Privilege:");
    println!("   • Grant minimum permissions needed for job function");
    println!("   • Regularly review and audit role permissions");
    println!("   • Use role hierarchies to simplify management");
    
    println!("\n✅ Kitchen-Specific Considerations:");
    println!("   • Station-based permissions (grill, prep, pastry)");
    println!("   • Shift-based access controls");
    println!("   • Emergency override procedures");
    println!("   • Cross-training permission escalation");
    
    println!("\n✅ Implementation Recommendations:");
    println!("   • Store roles and permissions in database");
    println!("   • Implement middleware for permission checking");
    println!("   • Use JWT claims for role information");
    println!("   • Log all access attempts for audit trails");
    
    println!("\n🎉 Role-Based Access Control Example Completed!");
    println!("===============================================");
    println!("✅ Multiple kitchen roles demonstrated");
    println!("✅ Permission matrix analyzed");
    println!("✅ Access control testing performed");
    println!("✅ Kitchen workflow simulation completed");
    println!("\n💡 Next Steps:");
    println!("   - Implement role-based middleware in the API");
    println!("   - Add permission checking to all endpoints");
    println!("   - Run 'cargo run --example user_crud' for user management");
    println!("   - Run 'cargo run --example full_workflow' for complete integration");
    
    Ok(())
}