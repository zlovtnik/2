//! Documentation validation tests
//! 
//! This module contains comprehensive tests to validate that all public APIs
//! have proper documentation, OpenAPI specifications are valid and complete,
//! and code examples compile and execute correctly.

// HashSet removed; imports are scoped per-test modules
use std::process::Command;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use reqwest::Client;
use axum::http::StatusCode;

/// Test suite for documentation validation
#[cfg(test)]
mod documentation_tests {
    use super::*;
    use server::docs::ApiDoc;
    use utoipa::OpenApi;

    /// Validate that all public APIs have documentation
    #[test]
    fn test_all_public_apis_documented() {
        let missing_docs = find_undocumented_apis();
        assert!(
            missing_docs.is_empty(),
            "Missing documentation for public APIs: {:?}",
            missing_docs
        );
    }

    /// Validate OpenAPI specification structure and completeness
    #[test]
    fn test_openapi_spec_valid() {
        let spec = ApiDoc::openapi();
        validate_openapi_spec(&spec).expect("OpenAPI spec should be valid");
    }

    /// Test that OpenAPI spec contains all required endpoints
    #[test]
    fn test_openapi_endpoints_complete() {
        let spec = ApiDoc::openapi();
        let paths = spec.paths.paths;
        
        // Expected endpoints based on the current API structure
        let expected_endpoints = vec![
            "/api/v1/auth/register",
            "/api/v1/auth/login", 
            "/api/v1/auth/refresh",
            "/api/v1/users",
            "/api/v1/users/me",
            "/api/v1/users/me/stats",
            "/api/v1/users/{id}",
            "/api/v1/refresh_tokens",
            "/api/v1/refresh_tokens/{id}",
            "/health/live",
            "/health/ready",
        ];

        for endpoint in expected_endpoints {
            assert!(
                paths.contains_key(endpoint),
                "Missing endpoint in OpenAPI spec: {}",
                endpoint
            );
        }
    }

    /// Test that OpenAPI spec has proper metadata
    #[test]
    fn test_openapi_metadata_complete() {
        let spec = ApiDoc::openapi();
        
        // Validate info section
        assert_eq!(spec.info.title, "Kitchen Management API");
        assert_eq!(spec.info.version, "1.0.0");
        assert!(spec.info.description.is_some());
        assert!(spec.info.contact.is_some());
        assert!(spec.info.license.is_some());
        
    // Validate servers (servers may be optional)
    assert!(spec.servers.as_ref().map(|s| !s.is_empty()).unwrap_or(false), "OpenAPI spec should have server configurations");
        
    // Validate tags (tags may be optional)
    assert!(spec.tags.as_ref().map(|t| !t.is_empty()).unwrap_or(false), "OpenAPI spec should have tags for organization");
        
        // Validate external docs
        assert!(spec.external_docs.is_some(), "OpenAPI spec should have external documentation links");
    }

    /// Test that all OpenAPI schemas are properly defined
    #[test]
    fn test_openapi_schemas_complete() {
        let spec = ApiDoc::openapi();
        
        // Inspect serialized spec JSON for components.schemas
        let spec_json = serde_json::to_value(&spec).expect("Should serialize spec");
        if let Some(components) = spec_json.get("components") {
            if let Some(schemas) = components.get("schemas") {
                // Expected core schemas
                let expected_schemas = vec![
                    "RegisterRequest",
                    "LoginRequest", 
                    "TokenResponse",
                    "ErrorResponse",
                    "User",
                    "UserInfoWithStats",
                    "HealthStatus",
                    "RefreshToken",
                    "ValidationErrorResponse",
                ];

                for schema_name in expected_schemas {
                    let present = schemas.as_object().map(|m| m.contains_key(schema_name)).unwrap_or(false);
                    assert!(
                        present,
                        "Missing schema in OpenAPI spec: {}",
                        schema_name
                    );
                }
            } else {
                panic!("OpenAPI spec should have component schemas defined");
            }
        } else {
            panic!("OpenAPI spec should have components section");
        }
    }
}

/// Integration tests for documentation endpoints
#[cfg(test)]
mod documentation_endpoints_tests {
    use super::*;
    use std::env;

    async fn create_test_app() -> axum::Router {
        use server::app;
        use sqlx::PgPool;
        
        // Set up test environment
        env::set_var("APP_DATABASE_URL", "postgres://user:pass@localhost/postgres");
        env::set_var("JWT_SECRET", "your-super-secret-jwt-key-here");
        
        let db_url = env::var("APP_DATABASE_URL").unwrap();
        let pool = PgPool::connect_lazy(&db_url).unwrap();
        
        app(pool)
    }

    /// Test that documentation endpoints are accessible
    #[tokio::test]
    async fn test_documentation_endpoints_accessible() {
        let app = create_test_app().await;
        
        // Start the app in the background
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let app_service = app.into_make_service();
        tokio::spawn(async move {
            axum::serve(listener, app_service).await.unwrap();
        });
        
        sleep(Duration::from_millis(100)).await;
        let client = Client::new();

        // Note: Swagger UI endpoints are currently commented out in lib.rs
        // This test validates the current state and can be updated when Swagger UI is re-enabled
        
        // Test that the app is running and health endpoints work
        let response = client
            .get(format!("http://{}/health/live", local_addr))
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test OpenAPI specification generation
    #[test]
    fn test_openapi_spec_generation() {
        use server::docs::ApiDoc;
        use utoipa::OpenApi;
        
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_string(&spec).expect("Should serialize OpenAPI spec to JSON");
        
        // Validate that the JSON is valid
        let parsed: Value = serde_json::from_str(&spec_json).expect("Generated OpenAPI spec should be valid JSON");
        
        // Basic structure validation
        assert!(parsed["openapi"].is_string());
        assert!(parsed["info"].is_object());
        assert!(parsed["paths"].is_object());
        assert!(parsed["components"].is_object());
        assert!(parsed["tags"].is_array());
    }
}

/// Example validation tests
#[cfg(test)]
mod example_validation_tests {
    use super::*;
    use std::env;
    use serde_json::json;

    /// Test that authentication examples work correctly
    #[tokio::test]
    async fn test_authentication_examples() {
        // Set up test environment
        env::set_var("APP_DATABASE_URL", "postgres://user:pass@localhost/postgres");
        env::set_var("JWT_SECRET", "your-super-secret-jwt-key-here");
        
        let app = create_test_app().await;
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let app_service = app.into_make_service();
        tokio::spawn(async move {
            axum::serve(listener, app_service).await.unwrap();
        });
        
        sleep(Duration::from_millis(100)).await;
        let client = Client::new();

        // Test registration example
        let registration_example = json!({
            "email": "test.chef@restaurant.com",
            "password": "SecurePass123!",
            "full_name": "Test Chef"
        });

        let response = client
            .post(format!("http://{}/api/v1/auth/register", local_addr))
            .json(&registration_example)
            .send()
            .await
            .unwrap();

        // Should either succeed or fail due to existing user
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
            "Registration example should return 200 or 400, got: {}",
            status
        );

        // If registration succeeded, test login example
        if status == StatusCode::OK {
            let login_example = json!({
                "email": "test.chef@restaurant.com",
                "password": "SecurePass123!"
            });

            let login_response = client
                .post(format!("http://{}/api/v1/auth/login", local_addr))
                .json(&login_example)
                .send()
                .await
                .unwrap();

            assert_eq!(login_response.status(), StatusCode::OK);
            
            // Validate response structure
            let token_response: Value = login_response.json().await.unwrap();
            assert!(token_response["token"].is_string());
        }
    }

    /// Test that health check examples work correctly
    #[tokio::test]
    async fn test_health_check_examples() {
        let app = create_test_app().await;
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let app_service = app.into_make_service();
        tokio::spawn(async move {
            axum::serve(listener, app_service).await.unwrap();
        });
        
        sleep(Duration::from_millis(100)).await;
        let client = Client::new();

        // Test live health check example
        let live_response = client
            .get(format!("http://{}/health/live", local_addr))
            .send()
            .await
            .unwrap();

        assert_eq!(live_response.status(), StatusCode::OK);
        
        let health_data: Value = live_response.json().await.unwrap();
        assert!(health_data["status"].is_string());

        // Test ready health check example
        let ready_response = client
            .get(format!("http://{}/health/ready", local_addr))
            .send()
            .await
            .unwrap();

        assert_eq!(ready_response.status(), StatusCode::OK);
    }

    async fn create_test_app() -> axum::Router {
        use server::app;
        use sqlx::PgPool;
        
        let db_url = env::var("APP_DATABASE_URL").unwrap();
        let pool = PgPool::connect_lazy(&db_url).unwrap();
        
        app(pool)
    }
}

/// Helper functions for documentation validation
fn find_undocumented_apis() -> Vec<String> {
    let mut missing_docs = Vec::new();
    
    // Run cargo doc with missing_docs lint to find undocumented public APIs
    let output = Command::new("cargo")
        .args(&["doc", "--no-deps", "--", "-D", "missing_docs"])
        .output()
        .expect("Failed to run cargo doc");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Use a regex to robustly extract backticked API names from warnings,
        // e.g. "missing documentation for ... `crate::api::foo::bar`".
        let re = regex::Regex::new(r"missing documentation for.*?`([^`]+)`").unwrap();
        for cap in re.captures_iter(&stderr) {
            if let Some(m) = cap.get(1) {
                missing_docs.push(m.as_str().to_string());
            }
        }
    }
    
    missing_docs
}

fn validate_openapi_spec(spec: &utoipa::openapi::OpenApi) -> Result<(), String> {
    // Serialize to JSON and validate the common OpenAPI shape to avoid relying on utoipa internals
    let spec_json = serde_json::to_value(spec).map_err(|e| format!("Failed to serialize spec: {}", e))?;

    // Basic checks
    if spec_json["info"]["title"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
        return Err("OpenAPI spec must have a title".to_string());
    }

    if spec_json["info"]["version"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
        return Err("OpenAPI spec must have a version".to_string());
    }

    if !spec_json["paths"].is_object() || spec_json["paths"].as_object().unwrap().is_empty() {
        return Err("OpenAPI spec must have at least one path".to_string());
    }

    // Validate that each path has at least one HTTP operation and that operations have responses
    if let Some(paths_obj) = spec_json["paths"].as_object() {
        for (path, path_item) in paths_obj {
            if let Some(path_obj) = path_item.as_object() {
                let mut found_op = false;
                for (method, operation) in path_obj {
                    if matches!(method.as_str(), "get" | "post" | "put" | "delete" | "patch" | "head" | "options" | "trace") {
                        found_op = true;
                        if !operation.get("responses").is_some() {
                            return Err(format!("Operation {} {} must have responses", method.to_uppercase(), path));
                        }
                    }
                }

                if !found_op {
                    return Err(format!("Path {} must have at least one HTTP operation", path));
                }
            }
        }
    }

    // Validate components.schemas presence
    if let Some(components) = spec_json.get("components") {
        if let Some(schemas) = components.get("schemas") {
            if !schemas.is_object() || schemas.as_object().unwrap().is_empty() {
                return Err("If components.schemas is present, it should not be empty".to_string());
            }
        } else {
            return Err("OpenAPI spec should have component schemas defined".to_string());
        }
    } else {
        return Err("OpenAPI spec should have components section".to_string());
    }

    Ok(())
}

/// Documentation coverage analysis
#[cfg(test)]
mod documentation_coverage_tests {
    use super::*;

    /// Test documentation coverage for API modules
    #[test]
    fn test_api_modules_documentation_coverage() {
        let modules_to_check = vec![
            "src/api/auth.rs",
            "src/api/user.rs", 
            "src/api/health.rs",
            "src/api/refresh_token.rs",
        ];
        
        for module in modules_to_check {
            validate_module_documentation(module);
        }
    }

    /// Test documentation coverage for core modules
    #[test]
    fn test_core_modules_documentation_coverage() {
        let modules_to_check = vec![
            "src/core/auth.rs",
            "src/core/user.rs",
            "src/core/refresh_token.rs",
        ];
        
        for module in modules_to_check {
            validate_module_documentation(module);
        }
    }
}

fn validate_module_documentation(module_path: &str) {
    // This is a simplified validation - in a real implementation,
    // you might use syn or other parsing libraries to analyze the AST
    let output = Command::new("cargo")
        .args(&["doc", "--no-deps", "--document-private-items", "--", "-D", "missing_docs"])
        .env("RUSTDOCFLAGS", format!("--document-private-items -D missing_docs"))
        .output()
        .expect("Failed to run cargo doc for module validation");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains(module_path) && stderr.contains("missing documentation") {
            panic!("Module {} has missing documentation: {}", module_path, stderr);
        }
    }
}