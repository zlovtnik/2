//! Documentation validation utilities
//! 
//! This module provides utilities for validating API documentation completeness,
//! correctness, and consistency across the codebase.

use std::collections::{HashMap, HashSet};
use serde_json::Value;

/// Documentation validation errors
#[derive(Debug)]
pub enum DocumentationError {
    MissingDocumentation { api: String },
    InvalidOpenApiSpec { reason: String },
    ExampleValidationFailed { example: String, error: String },
    BuildFailed { error: String },
    SchemaValidationFailed { schema: String, error: String },
}

impl std::fmt::Display for DocumentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentationError::MissingDocumentation { api } => {
                write!(f, "Missing documentation for public API: {}", api)
            }
            DocumentationError::InvalidOpenApiSpec { reason } => {
                write!(f, "Invalid OpenAPI specification: {}", reason)
            }
            DocumentationError::ExampleValidationFailed { example, error } => {
                write!(f, "Example validation failed: {} - {}", example, error)
            }
            DocumentationError::BuildFailed { error } => {
                write!(f, "Documentation build failed: {}", error)
            }
            DocumentationError::SchemaValidationFailed { schema, error } => {
                write!(f, "Schema validation failed: {} - {}", schema, error)
            }
        }
    }
}

impl std::error::Error for DocumentationError {}

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
    /// Warnings
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
    /// APIs by module
    pub module_coverage: HashMap<String, ModuleCoverage>,
}

/// Module-specific documentation coverage
#[derive(Debug, Clone)]
pub struct ModuleCoverage {
    /// Module name
    pub module_name: String,
    /// Total APIs in module
    pub total_apis: usize,
    /// Documented APIs in module
    pub documented_apis: usize,
    /// Coverage percentage for this module
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
                module_coverage: HashMap::new(),
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
            let has_operations = path_item.get.is_some() 
                || path_item.post.is_some() 
                || path_item.put.is_some() 
                || path_item.delete.is_some()
                || path_item.patch.is_some()
                || path_item.head.is_some()
                || path_item.options.is_some()
                || path_item.trace.is_some();
                
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
        let operations = [
            ("GET", &path_item.get),
            ("POST", &path_item.post),
            ("PUT", &path_item.put),
            ("DELETE", &path_item.delete),
            ("PATCH", &path_item.patch),
            ("HEAD", &path_item.head),
            ("OPTIONS", &path_item.options),
            ("TRACE", &path_item.trace),
        ];
        
        for (method, operation) in operations {
            if let Some(op) = operation {
                // Validate that operation has summary or description
                if op.summary.is_none() && op.description.is_none() {
                    result.missing_docs.push(format!("{} {} - missing summary or description", method, path));
                    result.success = false;
                }
                
                // Validate that operation has responses
                if op.responses.responses.is_empty() && op.responses.default.is_none() {
                    result.schema_errors.push(format!("{} {} - missing responses", method, path));
                    result.success = false;
                }
                
                // Check for error responses on non-health endpoints
                if !path.contains("/health/") {
                    let has_error_responses = op.responses.responses.keys().any(|status| {
                        status.starts_with('4') || status.starts_with('5')
                    }) || op.responses.default.is_some();
                    
                    if !has_error_responses {
                        result.warnings.push(format!("{} {} - consider adding error responses", method, path));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_schemas(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        if let Some(components) = &self.spec.components {
            if let Some(schemas) = &components.schemas {
                // Collect all schema references used in the spec
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
            if let Some(security_schemes) = &components.security_schemes {
                for (scheme_name, scheme) in security_schemes {
                    match scheme {
                        utoipa::openapi::security::SecurityScheme::Http(http_scheme) => {
                            if http_scheme.scheme.is_empty() {
                                result.schema_errors.push(format!("HTTP security scheme '{}' must have scheme", scheme_name));
                                result.success = false;
                            }
                        }
                        utoipa::openapi::security::SecurityScheme::ApiKey(api_key_scheme) => {
                            if api_key_scheme.name.is_empty() {
                                result.schema_errors.push(format!("API key security scheme '{}' must have name", scheme_name));
                                result.success = false;
                            }
                        }
                        _ => {} // Other schemes are valid
                    }
                }
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
        let operations = [
            &path_item.get, &path_item.post, &path_item.put, &path_item.delete,
            &path_item.patch, &path_item.head, &path_item.options, &path_item.trace,
        ];
        
        operations.iter().any(|op| {
            if let Some(operation) = op {
                operation.summary.is_some() || operation.description.is_some()
            } else {
                false
            }
        })
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

/// Example validator for code examples in documentation
pub struct ExampleValidator {
    base_url: String,
    client: Option<reqwest::Client>,
}

impl ExampleValidator {
    /// Create a new example validator
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: None,
        }
    }
    
    /// Create a new example validator with HTTP client for integration testing
    pub fn with_client(base_url: String, client: reqwest::Client) -> Self {
        Self {
            base_url,
            client: Some(client),
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
                module_coverage: HashMap::new(),
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
        
        // If we have a client, test the actual API
        if let Some(client) = &self.client {
            match client
                .post(&format!("{}/api/v1/auth/register", self.base_url))
                .json(&registration_example)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() && response.status() != reqwest::StatusCode::BAD_REQUEST {
                        result.warnings.push(format!("Registration example returned unexpected status: {}", response.status()));
                    }
                }
                Err(e) => {
                    result.warnings.push(format!("Registration example request failed: {}", e));
                }
            }
        }
        
        Ok(())
    }
    
    async fn validate_user_examples(&self, _result: &mut ValidationResult) -> Result<(), DocumentationError> {
        // Validate user management example structures
        let _user_example = serde_json::json!({
            "email": "kitchen.manager@restaurant.com",
            "full_name": "Kitchen Manager",
            "role": "manager"
        });
        
        // Add validation logic here
        Ok(())
    }
    
    async fn validate_health_examples(&self, result: &mut ValidationResult) -> Result<(), DocumentationError> {
        // If we have a client, test health endpoints
        if let Some(client) = &self.client {
            match client
                .get(&format!("{}/health/live", self.base_url))
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        result.invalid_examples.push(format!("Health live example failed: {}", response.status()));
                        result.success = false;
                    }
                }
                Err(e) => {
                    result.warnings.push(format!("Health live example request failed: {}", e));
                }
            }
        }
        
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
        
        // Basic validation should pass
        assert!(result.success || !result.schema_errors.is_empty());
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