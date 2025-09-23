//! OpenAPI specification validation tests
//! 
//! This module contains comprehensive tests to validate the OpenAPI specification
//! is complete, valid, and follows best practices for API documentation.

use serde_json::{Value, Map};
use std::collections::HashSet;

#[cfg(test)]
mod openapi_validation_tests {
    use super::helpers::*;
    use server::docs::ApiDoc;
    use utoipa::OpenApi;

    /// Test OpenAPI specification compliance with OpenAPI 3.0 standard
    #[test]
    fn test_openapi_3_0_compliance() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        // Validate OpenAPI version
        assert_eq!(
            spec_json["openapi"].as_str().unwrap(),
            "3.0.3",
            "Should use OpenAPI 3.0.3 specification"
        );
        
        // Validate required top-level fields
        assert!(spec_json["info"].is_object(), "Must have info object");
        assert!(spec_json["paths"].is_object(), "Must have paths object");
        
        // Validate info object required fields
        let info = &spec_json["info"];
        assert!(info["title"].is_string(), "Info must have title");
        assert!(info["version"].is_string(), "Info must have version");
    }

    /// Test that all API endpoints have proper documentation
    #[test]
    fn test_all_endpoints_documented() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        let paths = spec_json["paths"].as_object().expect("Paths should be an object");
        
        for (path, path_item) in paths {
            let path_obj = path_item.as_object().expect("Path item should be an object");
            
            // Check each HTTP method
            for (method, operation) in path_obj {
                if is_http_method(method) {
                    let op = operation.as_object().expect("Operation should be an object");
                    
                    // Validate required operation fields
                    assert!(
                        op.contains_key("summary") || op.contains_key("description"),
                        "Operation {} {} must have summary or description",
                        method.to_uppercase(),
                        path
                    );
                    
                    // Validate responses
                    assert!(
                        op.contains_key("responses"),
                        "Operation {} {} must have responses",
                        method.to_uppercase(),
                        path
                    );
                    
                    validate_operation_responses(path, method, &op["responses"]);
                }
            }
        }
    }

    /// Test that all schemas are properly defined and referenced
    #[test]
    fn test_schemas_complete_and_valid() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        // Get all schema references used in the spec
        let mut referenced_schemas = HashSet::new();
        collect_schema_references(&spec_json, &mut referenced_schemas);
        
        // Get all defined schemas
        let mut defined_schemas = HashSet::new();
        if let Some(components) = spec_json["components"].as_object() {
            if let Some(schemas) = components["schemas"].as_object() {
                for schema_name in schemas.keys() {
                    defined_schemas.insert(schema_name.clone());
                }
            }
        }
        
        // Validate that all referenced schemas are defined
        for referenced in &referenced_schemas {
            assert!(
                defined_schemas.contains(referenced),
                "Referenced schema '{}' is not defined in components.schemas",
                referenced
            );
        }
        
        // Validate that defined schemas are actually used (optional warning)
        for defined in &defined_schemas {
            if !referenced_schemas.contains(defined) {
                println!("Warning: Schema '{}' is defined but not referenced", defined);
            }
        }
    }

    /// Test security schemes are properly defined
    #[test]
    fn test_security_schemes_valid() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        if let Some(components) = spec_json["components"].as_object() {
            if let Some(security_schemes) = components["securitySchemes"].as_object() {
                for (scheme_name, scheme) in security_schemes {
                    let scheme_obj = scheme.as_object().expect("Security scheme should be an object");
                    
                    // Validate required fields
                    assert!(
                        scheme_obj.contains_key("type"),
                        "Security scheme '{}' must have type",
                        scheme_name
                    );
                    
                    // Validate specific scheme types
                    match scheme_obj["type"].as_str().unwrap() {
                        "http" => {
                            assert!(
                                scheme_obj.contains_key("scheme"),
                                "HTTP security scheme '{}' must have scheme",
                                scheme_name
                            );
                        }
                        "apiKey" => {
                            assert!(
                                scheme_obj.contains_key("name") && scheme_obj.contains_key("in"),
                                "API key security scheme '{}' must have name and in",
                                scheme_name
                            );
                        }
                        _ => {} // Other types are valid
                    }
                }
            }
        }
    }

    /// Test that tags are properly used and defined
    #[test]
    fn test_tags_consistent() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        // Collect all tags used in operations
        let mut used_tags = HashSet::new();
        if let Some(paths) = spec_json["paths"].as_object() {
            for path_item in paths.values() {
                if let Some(path_obj) = path_item.as_object() {
                    for (method, operation) in path_obj {
                        if is_http_method(method) {
                            if let Some(op) = operation.as_object() {
                                if let Some(tags) = op["tags"].as_array() {
                                    for tag in tags {
                                        if let Some(tag_str) = tag.as_str() {
                                            used_tags.insert(tag_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Collect all defined tags
        let mut defined_tags = HashSet::new();
        if let Some(tags) = spec_json["tags"].as_array() {
            for tag in tags {
                if let Some(tag_obj) = tag.as_object() {
                    if let Some(name) = tag_obj["name"].as_str() {
                        defined_tags.insert(name.to_string());
                    }
                }
            }
        }
        
        // Validate that all used tags are defined
        for used_tag in &used_tags {
            assert!(
                defined_tags.contains(used_tag),
                "Tag '{}' is used in operations but not defined in tags array",
                used_tag
            );
        }
    }

    /// Test server configurations are valid
    #[test]
    fn test_servers_valid() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        if let Some(servers) = spec_json["servers"].as_array() {
            assert!(!servers.is_empty(), "Should have at least one server defined");
            
            for server in servers {
                let server_obj = server.as_object().expect("Server should be an object");
                
                // Validate required fields
                assert!(
                    server_obj.contains_key("url"),
                    "Server must have URL"
                );
                
                // Validate URL format (basic check)
                let url = server_obj["url"].as_str().unwrap();
                assert!(
                    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("/"),
                    "Server URL '{}' should be a valid URL or relative path",
                    url
                );
            }
        }
    }

    /// Test external documentation links are valid
    #[test]
    fn test_external_docs_valid() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        // Check top-level external docs
        if let Some(external_docs) = spec_json["externalDocs"].as_object() {
            assert!(
                external_docs.contains_key("url"),
                "External docs must have URL"
            );
        }
        
        // Check tag-level external docs
        if let Some(tags) = spec_json["tags"].as_array() {
            for tag in tags {
                if let Some(tag_obj) = tag.as_object() {
                    if let Some(external_docs) = tag_obj["externalDocs"].as_object() {
                        assert!(
                            external_docs.contains_key("url"),
                            "Tag external docs must have URL"
                        );
                    }
                }
            }
        }
    }

    /// Test that error responses are properly documented
    #[test]
    fn test_error_responses_documented() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_value(&spec).expect("Should serialize to JSON");
        
        let paths = spec_json["paths"].as_object().expect("Paths should be an object");
        
        for (path, path_item) in paths {
            let path_obj = path_item.as_object().expect("Path item should be an object");
            
            for (method, operation) in path_obj {
                if is_http_method(method) {
                    let op = operation.as_object().expect("Operation should be an object");
                    
                    if let Some(responses) = op["responses"].as_object() {
                        // Check for common error status codes
                        let has_error_responses = responses.contains_key("400") 
                            || responses.contains_key("401")
                            || responses.contains_key("403")
                            || responses.contains_key("404")
                            || responses.contains_key("500")
                            || responses.contains_key("default");
                        
                        // For non-health endpoints, we should have error responses
                        if !path.contains("/health/") {
                            assert!(
                                has_error_responses,
                                "Operation {} {} should document error responses",
                                method.to_uppercase(),
                                path
                            );
                        }
                    }
                }
            }
        }
    }
/// Helper functions for OpenAPI validation
#[cfg(test)]
mod helpers {
    use super::*;

    pub(crate) fn is_http_method(method: &str) -> bool {
        matches!(method.to_lowercase().as_str(),
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" | "trace"
        )
    }

    pub(crate) fn validate_operation_responses(path: &str, method: &str, responses: &Value) {
        let responses_obj = responses.as_object().expect("Responses should be an object");

        assert!(
            !responses_obj.is_empty(),
            "Operation {} {} must have at least one response",
            method.to_uppercase(),
            path
        );

        // Validate that we have at least one success response (2xx)
        let has_success_response = responses_obj.keys().any(|status_code| {
            status_code.starts_with('2') || status_code == "default"
        });

        assert!(
            has_success_response,
            "Operation {} {} must have at least one success response (2xx)",
            method.to_uppercase(),
            path
        );
    }

    pub(crate) fn collect_schema_references(value: &Value, references: &mut HashSet<String>) {
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

    pub(crate) fn extract_schema_name(ref_str: &str) -> Option<String> {
        // Extract schema name from reference like "#/components/schemas/SchemaName"
        if ref_str.starts_with("#/components/schemas/") {
            Some(ref_str.replace("#/components/schemas/", ""))
        } else {
            None
        }
    }
}

/// Performance and quality tests for OpenAPI spec
#[cfg(test)]
mod openapi_quality_tests {
    use super::helpers::*;
    use server::docs::ApiDoc;
    use utoipa::OpenApi;

    /// Test that OpenAPI spec is not excessively large
    #[test]
    fn test_spec_size_reasonable() {
        let spec = ApiDoc::openapi();
        let spec_json = serde_json::to_string(&spec).expect("Should serialize to JSON");
        
        // Arbitrary limit - adjust based on your needs
        const MAX_SPEC_SIZE: usize = 1024 * 1024; // 1MB
        
        assert!(
            spec_json.len() < MAX_SPEC_SIZE,
            "OpenAPI spec size ({} bytes) exceeds reasonable limit ({} bytes)",
            spec_json.len(),
            MAX_SPEC_SIZE
        );
    }

    /// Test that spec generation is fast
    #[test]
    fn test_spec_generation_performance() {
        use std::time::Instant;
        
        let start = Instant::now();
        let _spec = ApiDoc::openapi();
        let duration = start.elapsed();
        
        // Should generate spec quickly (adjust threshold as needed)
        assert!(
            duration.as_millis() < 1000,
            "OpenAPI spec generation took too long: {:?}",
            duration
        );
    }

    /// Test that spec serialization is consistent
    #[test]
    fn test_spec_serialization_consistent() {
        let spec1 = ApiDoc::openapi();
        let spec2 = ApiDoc::openapi();
        
        let json1 = serde_json::to_string(&spec1).expect("Should serialize");
        let json2 = serde_json::to_string(&spec2).expect("Should serialize");
        
        assert_eq!(
            json1, json2,
            "OpenAPI spec serialization should be consistent between calls"
        );
    }
}