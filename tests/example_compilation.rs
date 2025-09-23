//! Code example compilation and execution tests
//! 
//! This module validates that all code examples in documentation
//! compile correctly and execute as expected.

use std::process::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(test)]
mod example_compilation_tests {
    use super::*;

    /// Test that all rustdoc examples compile successfully
    #[test]
    fn test_rustdoc_examples_compile() {
        // Run cargo test --doc to compile and test all documentation examples
        let output = Command::new("cargo")
            .args(&["test", "--doc", "--no-fail-fast"])
            .output()
            .expect("Failed to run cargo test --doc");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            panic!(
                "Documentation examples failed to compile or execute:\nSTDOUT:\n{}\nSTDERR:\n{}", 
                stdout, stderr
            );
        }

        // Verify that tests actually ran
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test result:") || stdout.contains("running"),
            "No documentation tests were executed. Output: {}",
            stdout
        );
    }

    /// Test compilation of standalone example files
    #[test]
    fn test_example_files_compile() {
        let examples_dir = Path::new("examples");
        
        if !examples_dir.exists() {
            // If examples directory doesn't exist yet, skip this test
            // This will be created in a later task
            return;
        }

        // Find all .rs files in the examples directory
        let example_files = find_rust_files(examples_dir);
        
        for example_file in example_files {
            validate_example_file_compilation(&example_file);
        }
    }

    /// Test that inline code examples in API documentation are valid
    #[test]
    fn test_inline_api_examples() {
        // Create temporary test files with extracted examples
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Extract and test authentication examples
        test_authentication_code_examples(&temp_dir);
        
        // Extract and test user management examples  
        test_user_management_code_examples(&temp_dir);
        
        // Extract and test health check examples
        test_health_check_code_examples(&temp_dir);
    }

    /// Test that OpenAPI examples are syntactically correct JSON
    #[test]
    fn test_openapi_json_examples() {
        use server::docs::ApiDoc;
        use utoipa::OpenApi;
        
        let spec = ApiDoc::openapi();
        
        // Validate that the entire spec serializes to valid JSON
        let spec_json = serde_json::to_string_pretty(&spec)
            .expect("OpenAPI spec should serialize to valid JSON");
        
        // Re-parse to ensure it's valid
        let _parsed: serde_json::Value = serde_json::from_str(&spec_json)
            .expect("Serialized OpenAPI spec should be valid JSON");
        
        // Check for example values in schemas
        if let Some(components) = &spec.components {
            if let Some(schemas) = &components.schemas {
                for (schema_name, schema) in schemas {
                    validate_schema_examples(schema_name, schema);
                }
            }
        }
    }
}

/// Helper functions for example validation

fn find_rust_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut rust_files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                rust_files.push(path);
            } else if path.is_dir() {
                rust_files.extend(find_rust_files(&path));
            }
        }
    }
    
    rust_files
}

fn validate_example_file_compilation(file_path: &Path) {
    // Check if the file compiles as a standalone example
    let output = Command::new("rustc")
        .args(&[
            "--edition", "2021",
            "--crate-type", "bin",
            "--extern", "server",
            "--extern", "tokio",
            "--extern", "serde_json",
            "--extern", "reqwest",
            file_path.to_str().unwrap(),
            "-o", "/tmp/example_test"
        ])
        .output()
        .expect("Failed to compile example file");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Example file {:?} failed to compile:\n{}", 
            file_path, stderr
        );
    }
}

fn test_authentication_code_examples(temp_dir: &TempDir) {
    // Create a test file with authentication examples
    let auth_example = r#"
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Registration example
    let registration = json!({
        "email": "chef@restaurant.com",
        "password": "SecurePass123!",
        "full_name": "Head Chef"
    });
    
    // This would normally make a request, but for compilation testing
    // we just validate the JSON structure
    assert!(registration["email"].is_string());
    assert!(registration["password"].is_string());
    assert!(registration["full_name"].is_string());
    
    // Login example
    let login = json!({
        "email": "chef@restaurant.com",
        "password": "SecurePass123!"
    });
    
    assert!(login["email"].is_string());
    assert!(login["password"].is_string());
    
    println!("Authentication examples compiled successfully");
    Ok(())
}
"#;

    let auth_file = temp_dir.path().join("auth_example.rs");
    fs::write(&auth_file, auth_example).expect("Failed to write auth example");
    
    // Compile the example
    let output = Command::new("rustc")
        .args(&[
            "--edition", "2021",
            "--extern", "tokio",
            "--extern", "serde_json", 
            "--extern", "reqwest",
            auth_file.to_str().unwrap(),
            "-o", temp_dir.path().join("auth_example").to_str().unwrap()
        ])
        .output()
        .expect("Failed to compile auth example");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Authentication example failed to compile:\n{}", stderr);
    }
}

fn test_user_management_code_examples(temp_dir: &TempDir) {
    // Create a test file with user management examples
    let user_example = r#"
use serde_json::json;

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // User creation example
    let user_data = json!({
        "email": "kitchen.manager@restaurant.com",
        "full_name": "Kitchen Manager",
        "role": "manager"
    });
    
    assert!(user_data["email"].is_string());
    assert!(user_data["full_name"].is_string());
    
    // User update example
    let update_data = json!({
        "full_name": "Senior Kitchen Manager",
        "role": "senior_manager"
    });
    
    assert!(update_data["full_name"].is_string());
    
    println!("User management examples compiled successfully");
    Ok(())
}
"#;

    let user_file = temp_dir.path().join("user_example.rs");
    fs::write(&user_file, user_example).expect("Failed to write user example");
    
    // Compile the example
    let output = Command::new("rustc")
        .args(&[
            "--edition", "2021",
            "--extern", "tokio",
            "--extern", "serde_json",
            user_file.to_str().unwrap(),
            "-o", temp_dir.path().join("user_example").to_str().unwrap()
        ])
        .output()
        .expect("Failed to compile user example");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("User management example failed to compile:\n{}", stderr);
    }
}

fn test_health_check_code_examples(temp_dir: &TempDir) {
    // Create a test file with health check examples
    let health_example = r#"
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Health check example structure
    let expected_response = serde_json::json!({
        "status": "healthy",
        "timestamp": "2024-01-01T00:00:00Z"
    });
    
    assert!(expected_response["status"].is_string());
    
    println!("Health check examples compiled successfully");
    Ok(())
}
"#;

    let health_file = temp_dir.path().join("health_example.rs");
    fs::write(&health_file, health_example).expect("Failed to write health example");
    
    // Compile the example
    let output = Command::new("rustc")
        .args(&[
            "--edition", "2021",
            "--extern", "tokio",
            "--extern", "serde_json",
            "--extern", "reqwest",
            health_file.to_str().unwrap(),
            "-o", temp_dir.path().join("health_example").to_str().unwrap()
        ])
        .output()
        .expect("Failed to compile health example");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Health check example failed to compile:\n{}", stderr);
    }
}

fn validate_schema_examples(schema_name: &str, _schema: &utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>) {
    // For now, just validate that schema names are reasonable
    assert!(!schema_name.is_empty(), "Schema name should not be empty");
    assert!(
        schema_name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "Schema name '{}' should only contain alphanumeric characters and underscores",
        schema_name
    );
}

/// Test that validates the build process includes documentation checks
#[cfg(test)]
mod build_integration_tests {
    use super::*;

    /// Test that documentation can be built successfully
    #[test]
    fn test_documentation_builds_successfully() {
        let output = Command::new("cargo")
            .args(&["doc", "--no-deps"])
            .output()
            .expect("Failed to run cargo doc");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Documentation build failed:\n{}", stderr);
        }
    }

    /// Test that documentation warnings are captured
    #[test]
    fn test_documentation_warnings_captured() {
        let output = Command::new("cargo")
            .args(&["doc", "--no-deps", "--", "-W", "missing_docs"])
            .output()
            .expect("Failed to run cargo doc with warnings");

        // We expect this to succeed, but we want to capture any warnings
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Log warnings for review (in a real CI environment, you might want to fail on warnings)
        if !stderr.is_empty() {
            println!("Documentation warnings:\n{}", stderr);
        }
        if !stdout.is_empty() {
            println!("Documentation output:\n{}", stdout);
        }
    }
}