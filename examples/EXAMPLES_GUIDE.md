# Kitchen Management API Examples Guide

This comprehensive guide provides detailed examples for using the Kitchen Management API effectively. Each example demonstrates real-world usage patterns and best practices for restaurant kitchen operations.

## Quick Start

1. **Start the API server**:
   ```bash
   cargo run
   ```

2. **Navigate to examples directory**:
   ```bash
   cd examples
   ```

3. **Run any example**:
   ```bash
   cargo run --example basic_auth
   ```

## Example Categories

### 🔐 Authentication Examples

#### Basic Authentication (`basic_auth.rs`)
Demonstrates fundamental authentication workflows for kitchen staff:
- User registration with validation
- Login with credential verification
- Using JWT tokens for API requests
- Error handling for authentication failures

**Kitchen Context**: Staff onboarding and daily login procedures.

```bash
cargo run --example basic_auth
```

#### JWT Token Refresh (`jwt_refresh.rs`)
Shows advanced token management for long-running kitchen operations:
- Token parsing and expiration checking
- Automatic token refresh strategies
- Token security best practices
- Long-shift token management

**Kitchen Context**: Managing authentication during 8-12 hour kitchen shifts.

```bash
cargo run --example jwt_refresh
```

#### Role-Based Access Control (`role_based_access.rs`)
Implements kitchen hierarchy and permission systems:
- Different roles (Head Chef, Sous Chef, Line Cook, Prep Cook)
- Permission matrices and access control
- Role-based workflow demonstrations
- Access violation handling

**Kitchen Context**: Kitchen hierarchy and station-based permissions.

```bash
cargo run --example role_based_access
```

### 👥 User Management Examples

#### User CRUD Operations (`user_crud.rs`)
Complete user lifecycle management:
- Creating users with validation
- Reading user information
- Updating user data
- Deleting users and cleanup
- Batch operations for multiple users

**Kitchen Context**: Staff management and personnel record keeping.

```bash
cargo run --example user_crud
```

#### User Statistics (`user_stats.rs`)
Analytics and reporting for kitchen staff:
- Retrieving user activity statistics
- Analyzing engagement metrics
- Generating staff reports
- Monitoring user behavior patterns

**Kitchen Context**: Staff performance monitoring and system usage analytics.

```bash
cargo run --example user_stats
```

#### Profile Management (`profile_management.rs`)
Personalization and preference management:
- User profile customization
- Kitchen-specific preferences
- Role-based profile templates
- Profile validation and monitoring

**Kitchen Context**: Personalizing the kitchen management experience.

```bash
cargo run --example profile_management
```

### 🔄 Integration Examples

#### Complete Workflow (`full_workflow.rs`)
End-to-end kitchen management simulation:
- Multi-user authentication and coordination
- System health monitoring
- Operational workflow simulation
- Comprehensive reporting

**Kitchen Context**: Complete day-in-the-life kitchen operations.

```bash
cargo run --example full_workflow
```

#### Error Handling (`error_handling.rs`)
Robust error handling patterns:
- Network errors and timeouts
- Authentication failures
- Validation errors
- Server errors and recovery
- Retry mechanisms and circuit breakers

**Kitchen Context**: Maintaining operations during system issues.

```bash
cargo run --example error_handling
```

#### Rate Limiting (`rate_limiting.rs`)
API throttling and queue management:
- Request rate limiting
- Priority-based queuing
- Adaptive throttling
- Rate limit monitoring

**Kitchen Context**: Managing API usage during busy kitchen periods.

```bash
cargo run --example rate_limiting
```

## Configuration

### Environment Variables

Set these environment variables to customize example behavior:

```bash
# API base URL (default: http://localhost:3000)
export API_BASE_URL=https://your-api-domain.com

# Enable debug logging
export RUST_LOG=debug
```

### Example Configuration Files

Create `.env` file in the examples directory:

```env
API_BASE_URL=http://localhost:3000
RUST_LOG=info
```

## Kitchen Management Scenarios

### Morning Shift Startup
```bash
# 1. Staff authentication
cargo run --example basic_auth

# 2. System health checks
cargo run --example full_workflow

# 3. Load user profiles and preferences
cargo run --example profile_management
```

### Busy Service Period
```bash
# 1. Handle high API load
cargo run --example rate_limiting

# 2. Manage multiple concurrent operations
cargo run --example full_workflow

# 3. Handle system errors gracefully
cargo run --example error_handling
```

### End-of-Day Reporting
```bash
# 1. Generate staff activity reports
cargo run --example user_stats

# 2. Review system performance
cargo run --example full_workflow

# 3. Update user profiles and preferences
cargo run --example profile_management
```

## Best Practices Demonstrated

### Security
- ✅ Secure password handling and validation
- ✅ JWT token management and refresh
- ✅ Role-based access control
- ✅ Input validation and sanitization
- ✅ Error message security

### Performance
- ✅ Request rate limiting and throttling
- ✅ Connection pooling and reuse
- ✅ Efficient error handling and retries
- ✅ Batch operations for bulk data
- ✅ Caching strategies

### Reliability
- ✅ Comprehensive error handling
- ✅ Graceful degradation patterns
- ✅ Circuit breaker implementations
- ✅ Health monitoring and checks
- ✅ Recovery and retry mechanisms

### Kitchen Operations
- ✅ Role-based workflow management
- ✅ Shift-based access patterns
- ✅ Station assignment and coordination
- ✅ Real-time operational monitoring
- ✅ Staff performance analytics

## Troubleshooting

### Common Issues

#### Connection Refused
```
Error: Connection refused (os error 61)
```
**Solution**: Ensure the API server is running on the correct port.

#### Authentication Failed
```
Error: Authentication Error: Invalid or expired authentication token
```
**Solution**: Check that user credentials are correct and tokens haven't expired.

#### Rate Limited
```
Error: Rate Limited: Too many requests (retry after 60 seconds)
```
**Solution**: Implement proper rate limiting as shown in the rate_limiting example.

### Debug Mode

Run examples with debug logging:
```bash
RUST_LOG=debug cargo run --example basic_auth
```

### API Server Issues

Check API server health:
```bash
curl http://localhost:3000/health/live
curl http://localhost:3000/health/ready
```

## Advanced Usage

### Custom API Client

Create your own API client based on the examples:

```rust
use reqwest::Client;
use serde_json::{json, Value};

struct KitchenApiClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl KitchenApiClient {
    fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token: None,
        }
    }
    
    async fn authenticate(&mut self, email: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation based on basic_auth example
        // ...
    }
}
```

### Integration Testing

Use examples as integration test templates:

```rust
#[tokio::test]
async fn test_kitchen_workflow() {
    // Based on full_workflow example
    // ...
}
```

### Production Deployment

Adapt examples for production use:

1. **Environment Configuration**:
   - Use proper environment variable management
   - Implement secure credential storage
   - Configure appropriate timeouts and retries

2. **Monitoring and Logging**:
   - Add structured logging
   - Implement metrics collection
   - Set up alerting for critical errors

3. **Security Hardening**:
   - Use HTTPS for all communications
   - Implement proper certificate validation
   - Add request signing for sensitive operations

## Contributing

To add new examples:

1. Create the example file in the appropriate category directory
2. Add the example to `Cargo.toml`
3. Update this guide with documentation
4. Include kitchen management context and use cases
5. Add error handling and best practices

## Support

For questions about these examples:

1. Check the inline documentation in each example
2. Review the API documentation
3. Run examples with debug logging
4. Check the troubleshooting section above

## License

These examples are provided under the same license as the Kitchen Management API project.