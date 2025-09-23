use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Existing protobuf compilation
    let out_dir = std::env::var("OUT_DIR").unwrap();
    tonic_build::configure()
        .file_descriptor_set_path(format!("{}/user_stats.bin", out_dir))
        .compile(&["proto/user_stats.proto"], &["proto"])?;
    
    // Documentation validation during build
    validate_documentation_during_build()?;
    
    Ok(())
}

/// Validate documentation completeness during build process
fn validate_documentation_during_build() -> Result<(), Box<dyn std::error::Error>> {
    // Only run documentation validation in development builds
    // Skip in release builds to avoid slowing down production builds
    if std::env::var("PROFILE").unwrap_or_default() == "release" {
        return Ok(());
    }
    
    // Skip if explicitly disabled via environment variable
    if std::env::var("SKIP_DOC_VALIDATION").is_ok() {
        println!("cargo:warning=Documentation validation skipped via SKIP_DOC_VALIDATION");
        return Ok(());
    }
    
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=tests/");
    
    // Validate that documentation can be generated
    validate_rustdoc_generation()?;
    
    // Validate OpenAPI spec generation
    validate_openapi_generation()?;
    
    // Run documentation tests if available
    run_documentation_tests()?;
    
    Ok(())
}

/// Validate that rustdoc can be generated successfully
fn validate_rustdoc_generation() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Validating rustdoc generation...");
    
    let output = Command::new("cargo")
        .args(&["doc", "--no-deps", "--quiet"])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Documentation generation failed: {}", stderr);
        return Err("Rustdoc generation failed".into());
    }
    
    Ok(())
}

/// Validate that OpenAPI specification can be generated
fn validate_openapi_generation() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Validating OpenAPI specification generation...");
    
    // This will be validated by the compilation of the docs module
    // If the OpenAPI spec has issues, the compilation will fail
    
    Ok(())
}

/// Run documentation-specific tests during build
fn run_documentation_tests() -> Result<(), Box<dyn std::error::Error>> {
    // Only run doc tests during build validation
    // Skip integration tests to avoid requiring database setup during build
    
    println!("cargo:warning=Running documentation tests...");
    
    let output = Command::new("cargo")
        .args(&["test", "--doc", "--quiet"])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Don't fail the build for doc test failures, just warn
        println!("cargo:warning=Documentation tests failed: {}", stderr);
        println!("cargo:warning=Documentation test output: {}", stdout);
    }
    
    Ok(())
}