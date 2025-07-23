# Enterprise Rust JWT Backend

[![CI/CD](https://github.com/company/rust-jwt-backend/workflows/CI/badge.svg)](https://github.com/company/rust-jwt-backend/actions)
[![Security Audit](https://github.com/company/rust-jwt-backend/workflows/Security%20Audit/badge.svg)](https://github.com/company/rust-jwt-backend/actions)
[![Coverage](https://codecov.io/gh/company/rust-jwt-backend/branch/main/graph/badge.svg)](https://codecov.io/gh/company/rust-jwt-backend)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A production-grade, enterprise-ready JWT authentication backend built with Rust, featuring comprehensive security, observability, and scalability patterns.

## 🚀 Features

### Core Functionality
- **JWT Authentication** - Secure token-based authentication with refresh tokens
- **User Management** - Registration, login, profile management with role-based access
- **PostgreSQL Integration** - Type-safe database operations with connection pooling
- **Password Security** - Argon2 hashing with configurable parameters

### Enterprise Features
- **Distributed Tracing** - OpenTelemetry integration with Jaeger/Zipkin
- **Metrics & Monitoring** - Prometheus metrics with Grafana dashboards
- **Health Checks** - Kubernetes-ready liveness and readiness probes
- **Rate Limiting** - Token bucket algorithm with Redis backend
- **Audit Logging** - Comprehensive security event logging
- **Configuration Management** - Environment-aware configuration with validation

### Security & Compliance
- **OWASP Compliance** - Protection against top 10 web vulnerabilities
- **PII Encryption** - Field-level encryption for sensitive data
- **Account Lockout** - Brute force protection with configurable policies
- **Session Management** - Secure session handling with automatic cleanup
- **CORS Policy** - Configurable cross-origin resource sharing
- **Input Validation** - Comprehensive request validation and sanitization

### Operational Excellence
- **Docker Support** - Multi-stage builds with distroless images
- **Kubernetes Ready** - Helm charts and deployment manifests
- **CI/CD Pipeline** - GitHub Actions with security scanning
- **Load Testing** - K6 performance test suite
- **Database Migrations** - Version-controlled schema management
- **Backup Strategy** - Automated PostgreSQL backup procedures

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer │────│  Rust Backend   │────│   PostgreSQL    │
│   (nginx/envoy) │    │    (Axum)       │    │   (Primary)     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │                        │
                       ┌─────────────────┐    ┌─────────────────┐
                       │      Redis      │    │   PostgreSQL    │
                       │   (Sessions)    │    │   (Replica)     │
                       └─────────────────┘    └─────────────────┘
                                │
                       ┌─────────────────┐
                       │  Observability  │
                       │ (Prometheus +   │
                       │  Jaeger +       │
                       │  Grafana)       │
                       └─────────────────┘
```

## 🛠️ Tech Stack

### Core Technologies
- **Runtime**: Tokio (async runtime)
- **Web Framework**: Axum (type-safe, performant)
- **Database**: PostgreSQL 15+ with SQLx
- **Authentication**: JWT with RS256 signing
- **Caching**: Redis for sessions and rate limiting

### Observability
- **Logging**: tracing + tracing-subscriber
- **Metrics**: Prometheus with custom business metrics
- **Tracing**: OpenTelemetry with Jaeger backend
- **Health Checks**: Custom health check framework

### Security
- **Password Hashing**: Argon2id (OWASP recommended)
- **Encryption**: AES-256-GCM for PII data
- **Rate Limiting**: Token bucket with Redis
- **Input Validation**: Custom validation framework

## 📋 Prerequisites

- **Rust**: 1.75+ (MSRV policy: latest stable - 2 versions)
- **PostgreSQL**: 15+
- **Redis**: 7+
- **Docker**: 24+ (for development)
- **Kubernetes**: 1.28+ (for production deployment)

## 🚀 Quick Start

### Development Environment

```bash
# Clone the repository
git clone https://github.com/company/rust-jwt-backend.git
cd rust-jwt-backend

# Start dependencies
docker-compose up -d postgres redis

# Install dependencies and run migrations
cargo install sqlx-cli
sqlx migrate run

# Copy environment configuration
cp .env.example .env.local

# Run the application
cargo run
```

### Production Deployment

```bash
# Build optimized container
docker build -t rust-jwt-backend:latest .

# Deploy to Kubernetes
helm upgrade --install jwt-backend ./helm/jwt-backend \
  --namespace production \
  --values ./helm/jwt-backend/values.prod.yaml
```

## 📁 Project Structure

```
.
├── src/
│   ├── api/                    # HTTP handlers and routing
│   │   ├── auth/              # Authentication endpoints
│   │   ├── users/             # User management endpoints
│   │   └── health/            # Health check endpoints
│   ├── core/                  # Business logic layer
│   │   ├── auth/              # Authentication services
│   │   ├── users/             # User services
│   │   └── security/          # Security utilities
│   ├── infrastructure/        # External integrations
│   │   ├── database/          # Database repositories
│   │   ├── cache/             # Redis integration
│   │   └── observability/     # Metrics and tracing
│   ├── config/                # Configuration management
│   ├── middleware/            # HTTP middleware
│   └── main.rs                # Application entry point
├── migrations/                # Database migrations
├── tests/                     # Integration tests
│   ├── api/                   # API endpoint tests
│   ├── performance/           # Load testing scripts
│   └── security/              # Security testing
├── helm/                      # Kubernetes deployment
├── .github/                   # CI/CD workflows
├── docker/                    # Docker configurations
└── docs/                      # Documentation
    ├── api/                   # OpenAPI specifications
    ├── deployment/            # Deployment guides
    └── security/              # Security documentation
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `APP_SERVER__HOST` | Server bind address | `0.0.0.0` | No |
| `APP_SERVER__PORT` | Server port | `3000` | No |
| `APP_DATABASE__URL` | PostgreSQL connection string | - | Yes |
| `APP_AUTH__JWT_SECRET` | JWT signing secret (min 32 chars) | - | Yes |
| `APP_REDIS__URL` | Redis connection string | - | Yes |
| `APP_LOGGING__LEVEL` | Log level (trace, debug, info, warn, error) | `info` | No |

### Configuration Files

```yaml
# config/production.yaml
server:
  host: "0.0.0.0"
  port: 3000
  request_timeout_secs: 30
  max_request_size: 2097152

database:
  max_connections: 50
  min_connections: 5
  acquire_timeout_secs: 10

auth:
  jwt_expiration_hours: 24
  password_hash_cost: 12
  max_login_attempts: 5
  lockout_duration_minutes: 15

observability:
  tracing:
    jaeger_endpoint: "http://jaeger:14268/api/traces"
  metrics:
    prometheus_endpoint: "0.0.0.0:9090"
```

## 📊 API Documentation

### Authentication Endpoints

#### Register User
```http
POST /api/v1/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "full_name": "John Doe"
}
```

#### Login
```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```

#### Refresh Token
```http
POST /api/v1/auth/refresh
Authorization: Bearer <refresh_token>
```

### User Management

#### Get Current User
```http
GET /api/v1/users/me
Authorization: Bearer <access_token>
```

#### Update Profile
```http
PUT /api/v1/users/me
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "full_name": "Jane Doe",
  "preferences": {
    "theme": "dark",
    "notifications": true
  }
}
```

### Health Checks

#### Liveness Probe
```http
GET /health/live
```

#### Readiness Probe
```http
GET /health/ready
```

#### Detailed Health
```http
GET /health
Authorization: Bearer <admin_token>
```

## 🧪 Testing

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests
```bash
cargo test --test integration
```

### Load Testing
```bash
# Install k6
brew install k6  # macOS
# or
sudo apt install k6  # Ubuntu

# Run performance tests
k6 run tests/performance/load_test.js
```

### Security Testing
```bash
# Run security audit
cargo audit

# Check for vulnerabilities
cargo deny check

# Static analysis
cargo clippy -- -D warnings
```

## 📈 Monitoring & Observability

### Metrics

The application exposes the following Prometheus metrics:

- `http_requests_total` - Total HTTP requests by method and status
- `http_request_duration_seconds` - HTTP request duration histogram
- `auth_attempts_total` - Authentication attempts by outcome
- `database_connections_active` - Active database connections
- `jwt_tokens_issued_total` - Total JWT tokens issued
- `rate_limit_exceeded_total` - Rate limit violations

### Tracing

Distributed tracing is implemented using OpenTelemetry:

- Request correlation IDs for end-to-end tracing
- Database query tracing with performance metrics
- External service call instrumentation
- Custom business logic spans

### Dashboards

Grafana dashboards are provided for:

- Application performance metrics
- Database performance and health
- Authentication and security events
- Infrastructure metrics
- Business KPIs

## 🔒 Security

### Security Measures

- **Password Policy**: Enforced complexity requirements
- **Rate Limiting**: Per-IP and per-user rate limits
- **Account Lockout**: Temporary lockout after failed attempts
- **JWT Security**: Short-lived access tokens + refresh tokens
- **Input Validation**: Comprehensive request validation
- **SQL Injection Protection**: Parameterized queries only
- **XSS Protection**: Content Security Policy headers
- **CSRF Protection**: Double-submit cookie pattern

### Compliance

- **GDPR**: Personal data encryption and deletion capabilities
- **SOX**: Comprehensive audit logging
- **PCI DSS**: Secure handling of sensitive data
- **OWASP**: Protection against top 10 vulnerabilities

### Security Headers

```http
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'
```

## 🚀 Deployment

### Docker

```dockerfile
# Multi-stage build for minimal production image
FROM rust:1.75-alpine AS builder
# ... build steps ...

FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/rust-jwt-backend /
EXPOSE 3000
ENTRYPOINT ["/rust-jwt-backend"]
```

### Kubernetes

```bash
# Deploy using Helm
helm upgrade --install jwt-backend ./helm/jwt-backend \
  --namespace production \
  --set image.tag=v1.2.3 \
  --set replicaCount=3 \
  --set resources.requests.memory=256Mi \
  --set resources.limits.memory=512Mi
```

### Production Checklist

- [ ] TLS certificates configured
- [ ] Database backups scheduled
- [ ] Monitoring alerts configured
- [ ] Log aggregation setup
- [ ] Secrets management implemented
- [ ] Network policies applied
- [ ] Resource limits set
- [ ] Auto-scaling configured

## 🤝 Contributing

### Development Workflow

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Implement** your changes with tests
4. **Run** the test suite (`make test`)
5. **Commit** your changes (`git commit -m 'Add amazing feature'`)
6. **Push** to the branch (`git push origin feature/amazing-feature`)
7. **Open** a Pull Request

### Code Standards

- **Formatting**: `cargo fmt` (enforced in CI)
- **Linting**: `cargo clippy` (no warnings allowed)
- **Testing**: Minimum 80% code coverage
- **Documentation**: All public APIs must be documented
- **Commits**: Conventional commit format

### Review Process

- All PRs require 2 approvals
- Automated security scanning must pass
- Performance benchmarks must not regress
- Integration tests must pass in staging environment

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

### Getting Help

- **Documentation**: [docs.company.com/rust-jwt-backend](https://docs.company.com/rust-jwt-backend)
- **Issues**: [GitHub Issues](https://github.com/company/rust-jwt-backend/issues)
- **Discussions**: [GitHub Discussions](https://github.com/company/rust-jwt-backend/discussions)
- **Slack**: #rust-backend channel

### Reporting Security Issues

Please report security vulnerabilities to security@company.com. Do not use public issue trackers for security-related problems.

## 🗺️ Roadmap

### Q1 2024
- [ ] gRPC API support
- [ ] GraphQL endpoint
- [ ] Multi-factor authentication
- [ ] Advanced rate limiting strategies

### Q2 2024
- [ ] Kafka event streaming
- [ ] Advanced analytics dashboard
- [ ] Mobile SDK integration
- [ ] Federated authentication (SAML/OIDC)

### Q3 2024
- [ ] Machine learning fraud detection
- [ ] Advanced audit capabilities
- [ ] Multi-region deployment
- [ ] Performance optimizations

---

**Built with ❤️ by the Platform Engineering Team**

## 🧑‍💻 API Usage Examples (with curl)

> All endpoints assume the server is running locally on http://localhost:3000 and the database is configured via .env.

### Health Checks
```bash
curl -i http://localhost:3000/health/live
curl -i http://localhost:3000/health/ready
```

### Authentication
# Register
curl -i -X POST http://localhost:3000/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"user@example.com","password":"SecurePassword123!","full_name":"John Doe"}'

# Login
curl -i -X POST http://localhost:3000/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"user@example.com","password":"SecurePassword123!"}'

# Refresh Token
curl -i -X POST http://localhost:3000/api/v1/auth/refresh \
  -H 'Authorization: Bearer <refresh_token>'

### User CRUD
# Create User
curl -i -X POST http://localhost:3000/api/v1/users \
  -H 'Content-Type: application/json' \
  -d '{"id":"<uuid>","email":"user@example.com","password_hash":"<hash>","full_name":"John Doe","preferences":null,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}'

# Get User (requires JWT)
curl -i http://localhost:3000/api/v1/users/<id> \
  -H 'Authorization: Bearer <access_token>'

# Update User (full_name)
curl -i -X PUT http://localhost:3000/api/v1/users/<id> \
  -H 'Authorization: Bearer <access_token>' \
  -H 'Content-Type: application/json' \
  -d '"Jane Doe"'

# Delete User
curl -i -X DELETE http://localhost:3000/api/v1/users/<id> \
  -H 'Authorization: Bearer <access_token>'

### Refresh Token CRUD
# Create Refresh Token
curl -i -X POST http://localhost:3000/api/v1/refresh_tokens \
  -H 'Content-Type: application/json' \
  -d '{"id":"<uuid>","user_id":"<uuid>","token":"<token>","expires_at":"2024-01-01T00:00:00Z","created_at":"2024-01-01T00:00:00Z"}'

# Get Refresh Token
curl -i http://localhost:3000/api/v1/refresh_tokens/<id>

# Update Refresh Token (token string)
curl -i -X PUT http://localhost:3000/api/v1/refresh_tokens/<id> \
  -H 'Content-Type: application/json' \
  -d '"new_token_string"'

# Delete Refresh Token
curl -i -X DELETE http://localhost:3000/api/v1/refresh_tokens/<id>