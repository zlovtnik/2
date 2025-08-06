use crate::middleware::rate_limit::{RateLimitConfig, RateLimitMiddleware, create_ip_rate_limiter, create_user_rate_limiter, create_global_rate_limiter};
use std::time::Duration;

/// Pre-configured rate limiters for different use cases
pub struct RateLimitConfigs;

impl RateLimitConfigs {
    /// Rate limiter for authentication endpoints (stricter)
    pub fn auth_endpoints() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 5,                               // 5 requests per minute
            window_duration: Duration::from_secs(60),      // 1 minute window
            burst_allowance: 2,                            // Allow 2 extra for occasional bursts
            use_redis: false,
        };
        create_ip_rate_limiter(config)
    }

    /// Rate limiter for API endpoints (moderate)
    pub fn api_endpoints() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 100,                             // 100 requests per minute
            window_duration: Duration::from_secs(60),      // 1 minute window
            burst_allowance: 20,                           // Allow 20 extra for bursts
            use_redis: false,
        };
        create_user_rate_limiter(config)
    }

    /// Rate limiter for public endpoints (lenient)
    pub fn public_endpoints() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 1000,                            // 1000 requests per minute
            window_duration: Duration::from_secs(60),      // 1 minute window
            burst_allowance: 100,                          // Allow 100 extra for bursts
            use_redis: false,
        };
        create_ip_rate_limiter(config)
    }

    /// Rate limiter for file uploads (very strict)
    pub fn upload_endpoints() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 10,                              // 10 uploads per hour
            window_duration: Duration::from_secs(3600),    // 1 hour window
            burst_allowance: 2,                            // Allow 2 extra uploads
            use_redis: false,
        };
        create_user_rate_limiter(config)
    }

    /// Rate limiter for admin endpoints (strict)
    pub fn admin_endpoints() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 50,                              // 50 requests per minute
            window_duration: Duration::from_secs(60),      // 1 minute window
            burst_allowance: 5,                            // Allow 5 extra for admin tasks
            use_redis: false,
        };
        create_user_rate_limiter(config)
    }

    /// Global rate limiter for entire application
    pub fn global_limit() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 10000,                           // 10k requests per minute globally
            window_duration: Duration::from_secs(60),      // 1 minute window
            burst_allowance: 1000,                         // Allow 1k extra for traffic spikes
            use_redis: false,
        };
        create_global_rate_limiter(config)
    }

    /// Rate limiter for password reset endpoints (very strict)
    pub fn password_reset() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 3,                               // 3 attempts per hour
            window_duration: Duration::from_secs(3600),    // 1 hour window
            burst_allowance: 0,                            // No burst allowance for security
            use_redis: false,
        };
        create_ip_rate_limiter(config)
    }

    /// Rate limiter for registration endpoints (strict)
    pub fn registration() -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests: 3,                               // 3 registrations per hour per IP
            window_duration: Duration::from_secs(3600),    // 1 hour window
            burst_allowance: 1,                            // Allow 1 extra registration
            use_redis: false,
        };
        create_ip_rate_limiter(config)
    }

    /// Custom rate limiter with user-defined parameters
    pub fn custom(max_requests: u32, window_secs: u64, burst_allowance: u32, use_user_based: bool) -> RateLimitMiddleware {
        let config = RateLimitConfig {
            max_requests,
            window_duration: Duration::from_secs(window_secs),
            burst_allowance,
            use_redis: false,
        };
        
        if use_user_based {
            create_user_rate_limiter(config)
        } else {
            create_ip_rate_limiter(config)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_endpoints_config() {
        let _limiter = RateLimitConfigs::auth_endpoints();
        // Just ensure it can be created without panicking
    }

    #[test]
    fn test_api_endpoints_config() {
        let _limiter = RateLimitConfigs::api_endpoints();
    }

    #[test]
    fn test_public_endpoints_config() {
        let _limiter = RateLimitConfigs::public_endpoints();
    }

    #[test]
    fn test_upload_endpoints_config() {
        let _limiter = RateLimitConfigs::upload_endpoints();
    }

    #[test]
    fn test_admin_endpoints_config() {
        let _limiter = RateLimitConfigs::admin_endpoints();
    }

    #[test]
    fn test_global_limit_config() {
        let _limiter = RateLimitConfigs::global_limit();
    }

    #[test]
    fn test_password_reset_config() {
        let _limiter = RateLimitConfigs::password_reset();
    }

    #[test]
    fn test_registration_config() {
        let _limiter = RateLimitConfigs::registration();
    }

    #[test]
    fn test_custom_config() {
        let _limiter = RateLimitConfigs::custom(50, 60, 5, true);
        let _limiter2 = RateLimitConfigs::custom(100, 120, 10, false);
    }
}
