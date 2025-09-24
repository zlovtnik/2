# Kitchen Management API Examples

This directory contains comprehensive code examples demonstrating how to use the Kitchen Management API effectively. Each example is a complete, runnable Rust program that showcases specific API functionality and common usage patterns.

## Directory Structure

- **`authentication/`** - User authentication workflows (register, login, token refresh)
- **`user_management/`** - User CRUD operations, profiles, and statistics
- **`integration/`** - Complete workflows combining multiple API endpoints
- **`error_handling/`** - Error handling patterns and best practices

## Prerequisites

Before running these examples, ensure you have:

1. **Rust installed** (1.70 or later)
2. **Kitchen Management API running** on `http://localhost:3000`
3. **Database configured** with proper migrations applied
4. **Environment variables set** (see `.env.example`)

## Running Examples

Each example can be run independently:

```bash
# Run authentication examples
cargo run --example basic_auth
cargo run --example jwt_refresh
cargo run --example role_based_access

# Run user management examples
cargo run --example user_crud
cargo run --example user_stats
cargo run --example profile_management

# Run integration examples
cargo run --example full_workflow
cargo run --example error_handling
cargo run --example rate_limiting
```

## Example Categories

### Authentication Examples
- **Basic Authentication** - Registration and login flows
- **JWT Token Management** - Token creation, validation, and refresh
- **Role-Based Access** - Permission-based API access patterns

### User Management Examples
- **User CRUD** - Create, read, update, delete user operations
- **User Statistics** - Retrieving user stats and analytics
- **Profile Management** - User profile operations and preferences

### Integration Examples
- **Full Workflow** - Complete kitchen management workflows
- **Error Handling** - Comprehensive error handling strategies
- **Rate Limiting** - Working with API rate limits

## Kitchen Management Context

These examples are designed specifically for restaurant kitchen management scenarios:

- **Staff Onboarding** - Creating accounts for kitchen personnel
- **Shift Management** - Authentication for different shifts
- **Order Processing** - User roles in order workflows
- **Inventory Tracking** - User permissions for inventory operations

## API Base URL

All examples use `http://localhost:3000` as the base URL. To use a different URL, set the `API_BASE_URL` environment variable:

```bash
export API_BASE_URL=https://your-api-domain.com
cargo run --example basic_auth
```

## Error Handling

Each example demonstrates proper error handling patterns:
- Network errors and timeouts
- Authentication failures
- Validation errors
- Rate limiting responses
- Server errors

## Security Considerations

These examples follow security best practices:
- Secure token storage and transmission
- Input validation and sanitization
- Proper error message handling
- HTTPS usage in production examples