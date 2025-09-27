use std::process::Command;

/// RAII guard for DOC_VALIDATION_IN_PROGRESS environment variable
struct DocValidationGuard;

impl DocValidationGuard {
    fn new() -> Self {
        std::env::set_var("DOC_VALIDATION_IN_PROGRESS", "1");
        Self
    }
}

impl Drop for DocValidationGuard {
    fn drop(&mut self) {
        std::env::remove_var("DOC_VALIDATION_IN_PROGRESS");
    }
}
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

    // Create RAII guard that sets the environment variable and ensures cleanup
    let _guard = DocValidationGuard::new();

    // Validate that documentation can be generated
    validate_rustdoc_generation()?;

    // Placeholder OpenAPI validation (currently just a stub)
    placeholder_openapi_validation()?;

    // Run documentation tests if available
    run_documentation_tests()?;

    Ok(())
}

/// Validate that rustdoc can be generated successfully
fn validate_rustdoc_generation() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Validating rustdoc generation...");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut command = create_cargo_command(&cargo, &["doc", "--no-deps", "--quiet"]);
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

/// Validate OpenAPI specification generation and compilation
fn validate_openapi_generation() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Validating OpenAPI specification generation...");

    // Try to compile the docs module which contains OpenAPI generation
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut command = create_cargo_command(&cargo, &["check", "--lib", "--features", "docs"]);
    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("cargo:warning=OpenAPI specification generation failed: {}", stderr);
        println!("cargo:warning=cargo check stdout: {}", stdout);
        return Err("OpenAPI specification generation failed".into());
    }

    println!("cargo:warning=OpenAPI specification validation completed successfully");
    Ok(())
}

/// OpenAPI validation with feature flag support
fn placeholder_openapi_validation() -> Result<(), Box<dyn std::error::Error>> {
    // Check if openapi-validate feature is enabled
    if cfg!(feature = "openapi-validate") {
        println!("cargo:warning=OpenAPI validation enabled via feature flag");
        return validate_openapi_generation();
    }

    // Check if explicitly enabled via environment variable
    if std::env::var("ENABLE_OPENAPI_VALIDATION").is_ok() {
        println!("cargo:warning=OpenAPI validation enabled via environment variable");
        return validate_openapi_generation();
    }

    // Default behavior: warn but don't fail
    println!("cargo:warning=OpenAPI validation is disabled - use 'openapi-validate' feature or ENABLE_OPENAPI_VALIDATION env var to enable");
    
    // Return the result from a no-op validation to maintain proper error propagation
    validate_openapi_generation_noop()
}

/// No-op OpenAPI validation that always succeeds but maintains Result type
fn validate_openapi_generation_noop() -> Result<(), Box<dyn std::error::Error>> {
    // This is a no-op that always succeeds, but maintains the Result type
    // for proper error propagation when the feature is enabled
    Ok(())
}

/// Create a cargo Command with common environment and flags used for
/// documentation validation. This centralizes the setup used by multiple
/// functions so behavior stays consistent.
fn create_cargo_command<C: AsRef<str>>(cargo_executable: C, args: &[&str]) -> Command {
    let cargo = cargo_executable.as_ref();
    let mut command = Command::new(cargo);
    command
        .args(args)
        .env("DOC_VALIDATION_IN_PROGRESS", "1")
        .env_remove("CARGO_PRIMARY_PACKAGE")
        .env_remove("CARGO_TARGET_DIR");

    if let Ok(value) = std::env::var("RUSTDOCFLAGS") {
        command.env("RUSTDOCFLAGS", value);
    }
    if let Ok(value) = std::env::var("RUSTFLAGS") {
        command.env("RUSTFLAGS", value);
    }
    if let Ok(value) = std::env::var("OUT_DIR") {
        command.env("OUT_DIR", value);
    }

    command
}

/// Run documentation-specific tests during build
fn run_documentation_tests() -> Result<(), Box<dyn std::error::Error>> {
    // Only run doc tests during build validation
    // Skip integration tests to avoid requiring database setup during build

    println!("cargo:warning=Running documentation tests...");

    let mut command = create_cargo_command("cargo", &["test", "--doc", "--quiet"]);
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