//! Documentation validation utilities
//! 
//! This module provides utilities for validating API documentation completeness,
//! correctness, and consistency across the codebase.

use std::collections::HashSet;
use serde_json::Value;
use thiserror::Error;

/// Documentation validation errors
#[derive(Debug, Error)]
pub enum DocumentationError {
    #[error("Missing documentation for public API: {api}")]
    MissingDocumentation { api: String },
    #[error("Invalid OpenAPI specification: {reason}")]
    InvalidOpenApiSpec { reason: String },
    #[error("Example validation failed: {example} - {error}")]
    ExampleValidationFailed { example: String, error: String },
    #[error("Documentation build failed: {error}")]
    BuildFailed { error: String },
    #[error("Schema validation failed: {schema} - {error}")]
    SchemaValidationFailed { schema: String, error: String },
}

/// Documentation validation results
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Validation success
    pub success: bool,
    /// Missing documentation items
    pub missing_docs: Vec<String>,
    /// Invalid examples
    pub invalid_examples: Vec<String>,
    /// Schema validation errors
    pub schema_errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Coverage statistics
    pub coverage: DocumentationCoverage,
}

/// Documentation coverage statistics
#[derive(Debug, Clone)]
pub struct DocumentationCoverage {
    /// Total number of public APIs
    pub total_apis: usize,
    /// Number of documented APIs
    pub documented_apis: usize,
    /// Documentation coverage percentage
    pub coverage_percentage: f64,
}

/// OpenAPI specification validator
pub struct OpenApiValidator {
    spec: utoipa::openapi::OpenApi,
}

impl OpenApiValidator {
    /// Create a new OpenAPI validator
    pub fn new(spec: utoipa::openapi::OpenApi) -> Self {
        Self { spec }
    }
    
    /// Validate the complete OpenAPI specification
    pub fn validate(&self) -> Result<ValidationResult, DocumentationError> {
        let mut result = ValidationResult {
            success: true,
            missing_docs: Vec::new(),
            invalid_examples: Vec::new(),
            schema_errors: Vec::new(),
            warnings: Vec::new(),
            coverage: DocumentationCoverage {
                total_apis: 0,
                documented_apis: 0,
                coverage_percentage: 0.0,
            },
        };
        
        // Validate basic structure
        self.validate_basic_structure(&mut result)?;
        
        // Validate paths and operations
        self.validate_paths(&mut result)?;
        
        // Validate schemas
        self.validate_schemas(&mut result)?;
        
        // Validate security schemes
        self.validate_security_schemes(&mut result)?;
        
        // Calculate coverage
        self.calculate_coverage(&mut result);
        
            // Ensure that if validation failed we report at least one schema error
            if !result.success && result.schema_errors.is_empty() {
                result.schema_errors.push("OpenAPI validation failed".to_string());
            }
        
        Ok(result)
    }
    
    fn validate_basic_structure(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        if self.spec.info.title.is_empty() {
            result.schema_errors.push("OpenAPI spec must have a title".to_string());
            result.success = false;
        }
        
        if self.spec.info.version.is_empty() {
            result.schema_errors.push("OpenAPI spec must have a version".to_string());
            result.success = false;
        }
        
        if self.spec.paths.paths.is_empty() {
            result.schema_errors.push("OpenAPI spec must have at least one path".to_string());
            result.success = false;
        }
        
        Ok(())
    }
    
    fn validate_paths(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        for (path, path_item) in &self.spec.paths.paths {
            // Check that each path has at least one operation
            let has_operations = !path_item.operations.is_empty();

            if !has_operations {
                result.schema_errors.push(format!("Path {} must have at least one HTTP operation", path));
                result.success = false;
            }

            // Validate individual operations
            self.validate_operations(path, path_item, result)?;
        }

        Ok(())
    }
    
    fn validate_operations(&self, path: &str, path_item: &utoipa::openapi::path::PathItem, result: &mut ValidationResult) -> Result<(), DocumentationError> {
    for (method, op) in &path_item.operations {
            // Validate that operation has summary or description
            if op.summary.is_none() && op.description.is_none() {
                result.missing_docs.push(format!("{} {} - missing summary or description", path_item_type_to_str(method), path));
                result.success = false;
            }

            // Validate that operation has responses
            if op.responses.responses.is_empty() {
                result.schema_errors.push(format!("{} {} - missing responses", path_item_type_to_str(method), path));
                result.success = false;
            }

            // Check for error responses on non-health endpoints
            if !path.contains("/health/") {
                let has_error_responses = op.responses.responses.keys().any(|status| {
                    status == "default" || status.starts_with('4') || status.starts_with('5')
                });

                if !has_error_responses {
                    result.warnings.push(format!("{} {} - consider adding error responses", path_item_type_to_str(method), path));
                }
            }
        }

        Ok(())
    }
    
    fn validate_schemas(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        if let Some(components) = &self.spec.components {
            let schemas = &components.schemas;
            if schemas.is_empty() {
                result.warnings.push("OpenAPI components present but no schemas defined; skipping schema reference checks".to_string());
            } else {
                let spec_json = serde_json::to_value(&self.spec)
                    .map_err(|e| DocumentationError::SchemaValidationFailed {
                        schema: "spec".to_string(),
                        error: e.to_string(),
                    })?;

                let mut referenced_schemas = HashSet::new();
                collect_schema_references(&spec_json, &mut referenced_schemas);

                // Check that all referenced schemas are defined
                for referenced in &referenced_schemas {
                    if !schemas.contains_key(referenced) {
                        result.schema_errors.push(format!("Referenced schema '{}' is not defined", referenced));
                        result.success = false;
                    }
                }

                // Warn about unused schemas
                for (schema_name, _) in schemas {
                    if !referenced_schemas.contains(schema_name) {
                        result.warnings.push(format!("Schema '{}' is defined but not referenced", schema_name));
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_security_schemes(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        if let Some(components) = &self.spec.components {
            let security_schemes = &components.security_schemes;
            for (scheme_name, scheme) in security_schemes {
                // Validate scheme description if present
                if let Some(description) = &scheme.description {
                    if description.trim().is_empty() {
                        result.schema_errors.push(format!("Security scheme '{}' has empty description", scheme_name));
                        result.success = false;
                    }
                }
                
                // Additional validation could be added here for scheme-specific fields
                // based on the scheme type (bearer, apiKey, oauth2, etc.)
            }
        }

        Ok(())
    }
    
    fn calculate_coverage(&self, result: &mut ValidationResult) {
        // Calculate basic coverage metrics
        let total_paths = self.spec.paths.paths.len();
        let documented_paths = self.spec.paths.paths.iter()
            .filter(|(_, path_item)| self.path_has_documentation(path_item))
            .count();
        
        result.coverage.total_apis = total_paths;
        result.coverage.documented_apis = documented_paths;
        result.coverage.coverage_percentage = if total_paths > 0 {
            (documented_paths as f64 / total_paths as f64) * 100.0
        } else {
            0.0
        };
    }
    
    fn path_has_documentation(&self, path_item: &utoipa::openapi::path::PathItem) -> bool {
        path_item.operations.values().any(|op| op.summary.is_some() || op.description.is_some())
    }
}

/// Collect all schema references from a JSON value
fn collect_schema_references(value: &Value, references: &mut HashSet<String>) {
    match value {
        Value::Object(obj) => {
            // Check for $ref
            if let Some(ref_value) = obj.get("$ref") {
                if let Some(ref_str) = ref_value.as_str() {
                    if let Some(schema_name) = extract_schema_name(ref_str) {
                        references.insert(schema_name);
                    }
                }
            }
            
            // Recursively check all object values
            for val in obj.values() {
                collect_schema_references(val, references);
            }
        }
        Value::Array(arr) => {
            // Recursively check all array elements
            for val in arr {
                collect_schema_references(val, references);
            }
        }
        _ => {} // Primitive values don't contain references
    }
}

/// Extract schema name from a reference string
fn extract_schema_name(ref_str: &str) -> Option<String> {
    if ref_str.starts_with("#/components/schemas/") {
        Some(ref_str.replace("#/components/schemas/", ""))
    } else {
        None
    }
}

/// Convert a PathItem key to a readable HTTP method string
fn path_item_type_to_str(method: &utoipa::openapi::path::PathItemType) -> &'static str {
    // PathItemType variants correspond to HTTP methods; match to human-readable strings
    match method {
        utoipa::openapi::path::PathItemType::Get => "GET",
        utoipa::openapi::path::PathItemType::Post => "POST",
        utoipa::openapi::path::PathItemType::Put => "PUT",
        utoipa::openapi::path::PathItemType::Delete => "DELETE",
        utoipa::openapi::path::PathItemType::Patch => "PATCH",
        utoipa::openapi::path::PathItemType::Head => "HEAD",
        utoipa::openapi::path::PathItemType::Options => "OPTIONS",
        utoipa::openapi::path::PathItemType::Trace => "TRACE",
        _ => "UNKNOWN",
    }
}

/// Example validator for code examples in documentation
pub struct ExampleValidator {
    base_url: String,
}

impl ExampleValidator {
    /// Create a new example validator
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
        }
    }
    
    /// Validate all examples
    pub async fn validate_examples(&self) -> Result<ValidationResult, DocumentationError> {
        let mut result = ValidationResult {
            success: true,
            missing_docs: Vec::new(),
            invalid_examples: Vec::new(),
            schema_errors: Vec::new(),
            warnings: Vec::new(),
            coverage: DocumentationCoverage {
                total_apis: 0,
                documented_apis: 0,
                coverage_percentage: 0.0,
            },
        };
        
        // Validate authentication examples
        self.validate_auth_examples(&mut result).await?;
        
        // Validate user management examples
        self.validate_user_examples(&mut result).await?;
        
        // Validate health check examples
        self.validate_health_examples(&mut result).await?;
        
        Ok(result)
    }
    
    async fn validate_auth_examples(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        // Validate registration example JSON structure
        let registration_example = serde_json::json!({
            "email": "chef@restaurant.com",
            "password": "SecurePass123!",
            "full_name": "Head Chef"
        });
        
        if !registration_example["email"].is_string() {
            result.invalid_examples.push("Registration example - email should be string".to_string());
            result.success = false;
        }
        
        // Static validation only - no HTTP requests during build
        result.warnings.push("HTTP-based example validation disabled during build".to_string());
        
        Ok(())
    }
    
    async fn validate_user_examples(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        use crate::core::user::User;
        use uuid::Uuid;
        use chrono::Utc;
        
        // Validate user management example structures
        let user_example = serde_json::json!({
            "id": Uuid::new_v4(),
            "email": "kitchen.manager@restaurant.com",
            "password_hash": "hashed_password_example",
            "full_name": "Kitchen Manager",
            "preferences": null,
            "created_at": Utc::now(),
            "updated_at": Utc::now()
        });
        
        // Validate the example can be deserialized into User struct
        match serde_json::from_value::<User>(user_example.clone()) {
            Ok(user) => {
                // Validate email format
                if !User::is_valid_email(&user.email) {
                    result.schema_errors.push("User example has invalid email format".to_string());
                    result.success = false;
                }
                
                // Validate full_name is not empty
                if user.full_name.trim().is_empty() {
                    result.schema_errors.push("User example has empty full_name".to_string());
                    result.success = false;
                }
                
                // Validate password_hash is not empty
                if user.password_hash.trim().is_empty() {
                    result.schema_errors.push("User example has empty password_hash".to_string());
                    result.success = false;
                }
            },
            Err(e) => {
                result.schema_errors.push(format!("User example failed deserialization: {}", e));
                result.success = false;
            }
        }
        
        Ok(())
    }
    
    async fn validate_health_examples(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        // Static validation only - no HTTP requests during build
        result.warnings.push("HTTP-based health check validation disabled during build".to_string());
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::ApiDoc;
    use utoipa::OpenApi;
    #[test]
    fn test_openapi_validator_creation() {
        let spec = ApiDoc::openapi();
        let validator = OpenApiValidator::new(spec);
        let result = validator.validate().expect("Validation should succeed");
        
        // Basic validation should pass for a valid API spec
        assert!(result.success);
        assert!(result.schema_errors.is_empty());
    }

    #[test]
    fn test_example_validator_creation() {
        let validator = ExampleValidator::new("http://localhost:3000".to_string());
        assert_eq!(validator.base_url, "http://localhost:3000");
    }

    #[test]
    fn test_schema_reference_extraction() {
        assert_eq!(
            extract_schema_name("#/components/schemas/User"),
            Some("User".to_string())
        );
        assert_eq!(
            extract_schema_name("#/components/schemas/RegisterRequest"),
            Some("RegisterRequest".to_string())
        );
        assert_eq!(
            extract_schema_name("invalid_reference"),
            None
        );
    }
}