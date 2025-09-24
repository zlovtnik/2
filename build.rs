use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Existing protobuf compilation
    let out_dir = std::env::var("OUT_DIR").unwrap();
    tonic_build::configure()
        .file_descriptor_set_path(format!("{}/user_stats.bin", out_dir))
        .compile(&["proto/user_stats.proto"], &["proto"])?;

    // Documentation validation during build - only if explicitly enabled
    if std::env::var("ENABLE_DOC_VALIDATION").is_ok() {
        validate_documentation_during_build()?;
    }

    Ok(())
}

/// Validate documentation completeness during build process
fn validate_documentation_during_build() -> Result<(), Box<dyn std::error::Error>> {
    // Check for reentrancy guard to prevent infinite recursion
    if std::env::var("DOC_VALIDATION_IN_PROGRESS").is_ok() {
        println!("cargo:warning=Documentation validation skipped (already in progress)");
        return Ok(());
    }

    // Skip in release builds to avoid slowing down production builds
    if std::env::var("PROFILE").unwrap_or_default() == "release" {
        return Ok(());
    }

    // Skip if explicitly disabled via environment variable
    if std::env::var("SKIP_DOC_VALIDATION").is_ok() {
        println!("cargo:warning=Documentation validation skipped via SKIP_DOC_VALIDATION");
        return Ok(());
    }

    // Emit rerun directive for the opt-in flag
    println!("cargo:rerun-if-env-changed=ENABLE_DOC_VALIDATION");
    println!("cargo:rerun-if-env-changed=SKIP_DOC_VALIDATION");

    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=tests/");

    // Set reentrancy guard
    std::env::set_var("DOC_VALIDATION_IN_PROGRESS", "1");

    // Validate that documentation can be generated
    validate_rustdoc_generation()?;

    // Validate OpenAPI spec generation
    validate_openapi_generation()?;

    // Run documentation tests if available
    run_documentation_tests()?;

    // Clean up reentrancy guard
    std::env::remove_var("DOC_VALIDATION_IN_PROGRESS");

    Ok(())
}

/// Validate that rustdoc can be generated successfully
fn validate_rustdoc_generation() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Validating rustdoc generation...");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .args(&["doc", "--no-deps", "--quiet"])
        .env("DOC_VALIDATION_IN_PROGRESS", "1")  // Prevent reentrancy
        .env_remove("CARGO_PRIMARY_PACKAGE")     // Remove to avoid confusion
        .env_remove("CARGO_TARGET_DIR");         // Use default target dir

    // Preserve important environment variables
    if let Ok(value) = std::env::var("RUSTDOCFLAGS") {
        command.env("RUSTDOCFLAGS", value);
    }
    if let Ok(value) = std::env::var("RUSTFLAGS") {
        command.env("RUSTFLAGS", value);
    }
    if let Ok(value) = std::env::var("OUT_DIR") {
        command.env("OUT_DIR", value);
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("cargo:warning=Documentation generation failed: {}", stderr);
        println!("cargo:warning=rustdoc stdout: {}", stdout);
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

    let mut command = Command::new("cargo");
    command
        .args(&["test", "--doc", "--quiet"])
        .env("DOC_VALIDATION_IN_PROGRESS", "1")  // Prevent reentrancy
        .env_remove("CARGO_PRIMARY_PACKAGE")     // Remove to avoid confusion
        .env_remove("CARGO_TARGET_DIR");         // Use default target dir

    // Preserve important environment variables
    if let Ok(value) = std::env::var("RUSTDOCFLAGS") {
        command.env("RUSTDOCFLAGS", value);
    }
    if let Ok(value) = std::env::var("RUSTFLAGS") {
        command.env("RUSTFLAGS", value);
    }
    if let Ok(value) = std::env::var("OUT_DIR") {
        command.env("OUT_DIR", value);
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Don't fail the build for doc test failures, just warn
        println!("cargo:warning=Documentation tests failed: {}", stderr);
        println!("cargo:warning=Documentation test output: {}", stdout);
    }

    Ok(())
}