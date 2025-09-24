//! Simple documentation test to verify the framework works

#[cfg(test)]
mod simple_tests {
    use server::docs::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_spec_generates() {
        let spec = ApiDoc::openapi();
        assert!(!spec.info.title.is_empty());
        assert!(!spec.info.version.is_empty());
        assert!(!spec.paths.paths.is_empty());
    }

    #[test]
    fn test_openapi_has_expected_endpoints() {
        let spec = ApiDoc::openapi();
        let paths = &spec.paths.paths;
        
        // Check for some key endpoints
        assert!(paths.contains_key("/health/live"));
        assert!(paths.contains_key("/api/v1/auth/register"));
        assert!(paths.contains_key("/api/v1/auth/login"));
    }
}