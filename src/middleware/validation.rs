use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{error, warn, debug};
use utoipa::ToSchema;
use validator::{Validate, ValidationErrors};

/// Standard validation error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub message: String,
    pub validation_errors: HashMap<String, Vec<String>>,
}

impl ValidationErrorResponse {
    pub fn new(errors: ValidationErrors) -> Self {
        let mut validation_errors = HashMap::new();
        
        for (field, field_errors) in errors.field_errors() {
            let error_messages: Vec<String> = field_errors
                .iter()
                .map(|error| {
                    error.message
                        .as_ref()
                        .map(|msg| msg.to_string())
                        .unwrap_or_else(|| format!("Invalid value for field '{}'", field))
                })
                .collect();
            validation_errors.insert(field.to_string(), error_messages);
        }
        
        Self {
            error: "VALIDATION_ERROR".to_string(),
            message: "Request validation failed".to_string(),
            validation_errors,
        }
    }
    
    pub fn from_json_error(error: &str) -> Self {
        let mut validation_errors = HashMap::new();
        validation_errors.insert("json".to_string(), vec![error.to_string()]);
        
        Self {
            error: "JSON_PARSE_ERROR".to_string(),
            message: "Invalid JSON format".to_string(),
            validation_errors,
        }
    }
    
    pub fn from_content_type_error() -> Self {
        let mut validation_errors = HashMap::new();
        validation_errors.insert("content_type".to_string(), vec!["Expected application/json".to_string()]);
        
        Self {
            error: "INVALID_CONTENT_TYPE".to_string(),
            message: "Invalid content type".to_string(),
            validation_errors,
        }
    }
}

impl IntoResponse for ValidationErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

/// Validation trait for request validation
pub trait ValidatedRequest: for<'de> Deserialize<'de> + Validate + Send + 'static {
    fn validate_request(&self) -> Result<(), ValidationErrors> {
        self.validate()
    }
}

/// Middleware function for request validation
pub async fn validate_json_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ValidationErrorResponse> {
    let (parts, body) = request.into_parts();
    
    // Only validate POST, PUT, PATCH requests with JSON body
    let should_validate = matches!(
        parts.method.as_str(),
        "POST" | "PUT" | "PATCH"
    );
    
    if !should_validate {
        let request = Request::from_parts(parts, body);
        return Ok(next.run(request).await);
    }
    
    // Check content type for JSON requests
    let content_type = parts.headers
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("");
    
    if !content_type.starts_with("application/json") && !parts.uri.path().contains("/health") {
        warn!(path = %parts.uri.path(), content_type = %content_type, "Invalid content type for validation");
        return Err(ValidationErrorResponse::from_content_type_error());
    }
    
    // Read body bytes
    let body_bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return Err(ValidationErrorResponse::from_json_error("Failed to read request body"));
        }
    };
    
    // Validate JSON syntax if body is not empty
    if !body_bytes.is_empty() {
        match serde_json::from_slice::<Value>(&body_bytes) {
            Ok(_) => {
                debug!(path = %parts.uri.path(), "JSON syntax validation passed");
            }
            Err(e) => {
                warn!(path = %parts.uri.path(), error = %e, "JSON syntax validation failed");
                return Err(ValidationErrorResponse::from_json_error(&e.to_string()));
            }
        }
    }
    
    // Reconstruct request with the same body
    let body = Body::from(body_bytes);
    let request = Request::from_parts(parts, body);
    
    debug!("Request validation middleware passed, proceeding to handler");
    Ok(next.run(request).await)
}

/// Input sanitization utilities
pub struct InputSanitizer;

impl InputSanitizer {
    /// Sanitize email input
    pub fn sanitize_email(email: &str) -> String {
        email.trim().to_lowercase()
    }
    
    /// Sanitize general text input (remove potential XSS patterns)
    pub fn sanitize_text(text: &str) -> String {
        text.trim()
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#x27;")
    }
    
    /// Sanitize SQL input (basic protection)
    pub fn sanitize_sql_input(input: &str) -> String {
        input.trim()
            .replace("'", "''")  // Escape single quotes
            .replace("--", "")   // Remove SQL comments
            .replace(";", "")    // Remove statement terminators
    }
    
    /// Validate and sanitize password (check for common patterns)
    pub fn validate_password_strength(password: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        if password.len() < 8 {
            errors.push("Password must be at least 8 characters long".to_string());
        }
        
        if password.len() > 128 {
            errors.push("Password must be less than 128 characters long".to_string());
        }
        
        if !password.chars().any(|c| c.is_ascii_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }
        
        if !password.chars().any(|c| c.is_ascii_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }
        
        if !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one number".to_string());
        }
        
        if !password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
            errors.push("Password must contain at least one special character".to_string());
        }
        
        // Check for common weak patterns
        let lower_password = password.to_lowercase();
        let weak_patterns = ["password", "123456", "qwerty", "admin", "user"];
        for pattern in &weak_patterns {
            if lower_password.contains(pattern) {
                errors.push(format!("Password cannot contain common patterns like '{}'", pattern));
                break;
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::post,
        Router,
    };
    use tower::ServiceExt;

    async fn dummy_handler() -> &'static str {
        "OK"
    }

    fn create_test_app() -> Router {
        Router::new()
            .route("/test", post(dummy_handler))
            .layer(middleware::from_fn(validate_json_middleware))
    }

    #[tokio::test]
    async fn test_valid_json_request() {
        let app = create_test_app();
        
        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"test": "value"}"#))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_json_request() {
        let app = create_test_app();
        
        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"test": invalid}"#))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_missing_content_type() {
        let app = create_test_app();
        
        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .body(Body::from(r#"{"test": "value"}"#))
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_request_passes_through() {
        let app = create_test_app();
        
        let request = Request::builder()
            .method("GET")
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        
        // Note: This will fail because our handler only accepts POST,
        // but the middleware should let it through
        let response = app.oneshot(request).await.unwrap();
        // The 405 Method Not Allowed is from Axum, not our validation middleware
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_sanitize_email() {
        assert_eq!(InputSanitizer::sanitize_email("  Test@Example.COM  "), "test@example.com");
    }

    #[test]
    fn test_sanitize_text() {
        let input = "<script>alert('xss')</script>";
        let expected = "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;";
        assert_eq!(InputSanitizer::sanitize_text(input), expected);
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid password
        assert!(InputSanitizer::validate_password_strength("StrongPass123!").is_ok());
        
        // Too short
        assert!(InputSanitizer::validate_password_strength("Short1!").is_err());
        
        // Missing uppercase
        assert!(InputSanitizer::validate_password_strength("lowercase123!").is_err());
        
        // Missing special character
        assert!(InputSanitizer::validate_password_strength("Password123").is_err());
        
        // Contains weak pattern
        assert!(InputSanitizer::validate_password_strength("password123!").is_err());
    }

    #[test]
    fn test_validation_error_response_creation() {
        use validator::ValidationErrors;
        
        let mut errors = ValidationErrors::new();
        errors.add("email", validator::ValidationError::new("email"));
        errors.add("password", validator::ValidationError::new("length"));
        
        let response = ValidationErrorResponse::new(errors);
        assert_eq!(response.error, "VALIDATION_ERROR");
        assert!(response.validation_errors.contains_key("email"));
        assert!(response.validation_errors.contains_key("password"));
    }
}
