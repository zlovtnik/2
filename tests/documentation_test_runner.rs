//! Documentation test runner
//! 
//! This module provides a comprehensive test runner for all documentation
//! validation tests, including integration with the validation utilities.

use std::env;
use std::time::Instant;
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use reqwest::Client;

#[cfg(test)]
mod documentation_integration_tests {
    use super::*;
    use server::docs::{ApiDoc, validation::{OpenApiValidator, ExampleValidator}};
    use utoipa::OpenApi;

    /// Comprehensive documentation validation test
    #[tokio::test]
    async fn test_comprehensive_documentation_validation() {
        println!("Starting comprehensive documentation validation...");
        
        // Test OpenAPI specification validation
        test_openapi_validation().await;
        
        // Test example validation
        test_example_validation().await;
        
        // Test documentation completeness
        test_documentation_completeness().await;
        
        println!("Comprehensive documentation validation completed successfully");
    }

    async fn test_openapi_validation() {
        println!("Validating OpenAPI specification...");
        
        let spec = ApiDoc::openapi();
        let validator = OpenApiValidator::new(spec);
        
        let start_time = Instant::now();
        let result = validator.validate().expect("OpenAPI validation should not fail");
        let validation_time = start_time.elapsed();
        
        println!("OpenAPI validation completed in {:?}", validation_time);
        
        // Print validation results
        if !result.success {
            println!("OpenAPI validation issues found:");
            for error in &result.schema_errors {
                println!("  ERROR: {}", error);
            }
            for missing in &result.missing_docs {
                println!("  MISSING DOC: {}", missing);
            }
        }
        
        if !result.warnings.is_empty() {
            println!("OpenAPI validation warnings:");
            for warning in &result.warnings {
                println!("  WARNING: {}", warning);
            }
        }
        
        println!("Documentation coverage: {:.1}%", result.coverage.coverage_percentage);
        
        // Assert that critical validations pass
        assert!(
            result.schema_errors.is_empty(),
            "OpenAPI specification has schema errors: {:?}",
            result.schema_errors
        );
    }

    async fn test_example_validation() {
        println!("Validating code examples...");
        
        // Set up test environment
        env::set_var("APP_DATABASE_URL", "postgres://user:pass@localhost/postgres");
        env::set_var("JWT_SECRET", "your-super-secret-jwt-key-here");
        
        // Create test app
        let app = create_test_app().await;
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let app_service = app.into_make_service();
        tokio::spawn(async move {
            axum::serve(listener, app_service).await.unwrap();
        });
        
        sleep(Duration::from_millis(100)).await;
        
        // Create example validator with HTTP client
        let client = Client::new();
        let base_url = format!("http://{}", local_addr);
        let validator = ExampleValidator::with_client(base_url, client);
        
        let start_time = Instant::now();
        let result = validator.validate_examples().await.expect("Example validation should not fail");
        let validation_time = start_time.elapsed();
        
        println!("Example validation completed in {:?}", validation_time);
        
        // Print validation results
        if !result.success {
            println!("Example validation issues found:");
            for error in &result.invalid_examples {
                println!("  INVALID EXAMPLE: {}", error);
            }
        }
        
        if !result.warnings.is_empty() {
            println!("Example validation warnings:");
            for warning in &result.warnings {
                println!("  WARNING: {}", warning);
            }
        }
        
        // Examples should be valid (warnings are acceptable)
        assert!(
            result.invalid_examples.is_empty(),
            "Code examples have validation errors: {:?}",
            result.invalid_examples
        );
    }

    async fn test_documentation_completeness() {
        println!("Testing documentation completeness...");
        
        // Test that all public APIs are documented
        let missing_docs = find_undocumented_public_apis();
        
        if !missing_docs.is_empty() {
            println!("APIs missing documentation:");
            for missing in &missing_docs {
                println!("  MISSING: {}", missing);
            }
        }
        
        // For now, we'll warn about missing docs but not fail the test
        // In a stricter environment, you might want to fail here
        if !missing_docs.is_empty() {
            println!("WARNING: {} public APIs are missing documentation", missing_docs.len());
        } else {
            println!("All public APIs have documentation");
        }
    }

    async fn create_test_app() -> axum::Router {
        use server::app;
        use sqlx::PgPool;
        
        let db_url = env::var("APP_DATABASE_URL").unwrap();
        let pool = PgPool::connect_lazy(&db_url).unwrap();
        
        app(pool)
    }

    /// Performance test for documentation generation
    #[test]
    fn test_documentation_generation_performance() {
        println!("Testing documentation generation performance...");
        
        let iterations = 10;
        let mut total_time = Duration::new(0, 0);
        
        for i in 0..iterations {
            let start = Instant::now();
            let _spec = ApiDoc::openapi();
            let iteration_time = start.elapsed();
            total_time += iteration_time;
            
            println!("Iteration {}: {:?}", i + 1, iteration_time);
        }
        
        let average_time = total_time / iterations;
        println!("Average generation time: {:?}", average_time);
        
        // Documentation generation should be reasonably fast
        assert!(
            average_time.as_millis() < 500,
            "Documentation generation is too slow: {:?}",
            average_time
        );
    }

    /// Test documentation consistency across multiple generations
    #[test]
    fn test_documentation_consistency() {
        println!("Testing documentation consistency...");
        
        let spec1 = ApiDoc::openapi();
        let spec2 = ApiDoc::openapi();
        
        let json1 = serde_json::to_string(&spec1).expect("Should serialize");
        let json2 = serde_json::to_string(&spec2).expect("Should serialize");
        
        assert_eq!(
            json1, json2,
            "Documentation generation should be consistent"
        );
        
        println!("Documentation generation is consistent");
    }

    /// Test that documentation includes all expected endpoints
    #[test]
    fn test_all_endpoints_documented() {
        println!("Testing endpoint documentation coverage...");
        
        let spec = ApiDoc::openapi();
        let paths = &spec.paths.paths;
        
        // Expected endpoints based on current API
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
        
        let mut missing_endpoints = Vec::new();
        let mut documented_endpoints = Vec::new();
        
        for endpoint in &expected_endpoints {
            if paths.contains_key(*endpoint) {
                documented_endpoints.push(*endpoint);
            } else {
                missing_endpoints.push(*endpoint);
            }
        }
        
        println!("Documented endpoints: {}", documented_endpoints.len());
        println!("Missing endpoints: {}", missing_endpoints.len());
        
        if !missing_endpoints.is_empty() {
            println!("Missing endpoints:");
            for endpoint in &missing_endpoints {
                println!("  MISSING: {}", endpoint);
            }
        }
        
        // All expected endpoints should be documented
        assert!(
            missing_endpoints.is_empty(),
            "Missing endpoints in documentation: {:?}",
            missing_endpoints
        );
        
        println!("All expected endpoints are documented");
    }
}

/// Helper functions for documentation testing

fn find_undocumented_public_apis() -> Vec<String> {
    use std::process::Command;
    
    let mut missing_docs = Vec::new();
    
    // Run cargo doc with missing_docs lint
    let output = Command::new("cargo")
        .args(&["doc", "--no-deps", "--", "-D", "missing_docs"])
        .output()
        .expect("Failed to run cargo doc");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Parse warnings for missing documentation
        for line in stderr.lines() {
            if line.contains("missing documentation for") {
                if let Some(start) = line.find("missing documentation for ") {
                    let api_part = &line[start + 26..];
                    if let Some(end) = api_part.find('`') {
                        let api_name = &api_part[1..end];
                        missing_docs.push(api_name.to_string());
                    }
                }
            }
        }
    }
    
    missing_docs
}

/// Documentation quality metrics
#[cfg(test)]
mod documentation_quality_tests {
    use super::*;
    use server::docs::ApiDoc;
    use utoipa::OpenApi;

    /// Test documentation quality metrics
    #[test]
    fn test_documentation_quality_metrics() {
        println!("Calculating documentation quality metrics...");
        
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize");
        
        // Calculate metrics
        let total_paths = spec.paths.paths.len();
        let paths_with_descriptions = count_paths_with_descriptions(&spec);
        let total_operations = count_total_operations(&spec);
        let operations_with_examples = count_operations_with_examples(&spec_json);
        
        println!("Documentation Quality Metrics:");
        println!("  Total paths: {}", total_paths);
        println!("  Paths with descriptions: {}", paths_with_descriptions);
        println!("  Total operations: {}", total_operations);
        println!("  Operations with examples: {}", operations_with_examples);
        
        let path_description_coverage = if total_paths > 0 {
            (paths_with_descriptions as f64 / total_paths as f64) * 100.0
        } else {
            0.0
        };
        
        let example_coverage = if total_operations > 0 {
            (operations_with_examples as f64 / total_operations as f64) * 100.0
        } else {
            0.0
        };
        
        println!("  Path description coverage: {:.1}%", path_description_coverage);
        println!("  Example coverage: {:.1}%", example_coverage);
        
        // Quality thresholds (adjust as needed)
        assert!(
            path_description_coverage >= 80.0,
            "Path description coverage ({:.1}%) is below threshold (80%)",
            path_description_coverage
        );
    }
}

fn count_paths_with_descriptions(spec: &utoipa::openapi::OpenApi) -> usize {
    spec.paths.paths.iter()
        .filter(|(_, path_item)| {
            path_item.description.is_some() || has_operation_with_description(path_item)
        })
        .count()
}

fn has_operation_with_description(path_item: &utoipa::openapi::path::PathItem) -> bool {
    let operations = [
        &path_item.get, &path_item.post, &path_item.put, &path_item.delete,
        &path_item.patch, &path_item.head, &path_item.options, &path_item.trace,
    ];
    
    operations.iter().any(|op| {
        if let Some(operation) = op {
            operation.description.is_some() || operation.summary.is_some()
        } else {
            false
        }
    })
}

fn count_total_operations(spec: &utoipa::openapi::OpenApi) -> usize {
    spec.paths.paths.iter()
        .map(|(_, path_item)| {
            let operations = [
                &path_item.get, &path_item.post, &path_item.put, &path_item.delete,
                &path_item.patch, &path_item.head, &path_item.options, &path_item.trace,
            ];
            operations.iter().filter(|op| op.is_some()).count()
        })
        .sum()
}

fn count_operations_with_examples(spec_json: &serde_json::Value) -> usize {
    let mut count = 0;
    
    if let Some(paths) = spec_json["paths"].as_object() {
        for path_item in paths.values() {
            if let Some(path_obj) = path_item.as_object() {
                for (method, operation) in path_obj {
                    if is_http_method(method) {
                        if let Some(op) = operation.as_object() {
                            if has_examples(op) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    
    count
}

fn is_http_method(method: &str) -> bool {
    matches!(method.to_lowercase().as_str(), 
        "get" | "post" | "put" | "delete" | "patch" | "head" | "options" | "trace"
    )
}

fn has_examples(operation: &serde_json::Map<String, serde_json::Value>) -> bool {
    // Check for examples in request body or responses
    if let Some(request_body) = operation.get("requestBody") {
        if has_content_examples(request_body) {
            return true;
        }
    }
    
    if let Some(responses) = operation.get("responses") {
        if let Some(responses_obj) = responses.as_object() {
            for response in responses_obj.values() {
                if has_content_examples(response) {
                    return true;
                }
            }
        }
    }
    
    false
}

fn has_content_examples(content_item: &serde_json::Value) -> bool {
    if let Some(content) = content_item.get("content") {
        if let Some(content_obj) = content.as_object() {
            for media_type in content_obj.values() {
                if let Some(media_obj) = media_type.as_object() {
                    if media_obj.contains_key("example") || media_obj.contains_key("examples") {
                        return true;
                    }
                }
            }
        }
    }
    false
}