# Multi-stage Dockerfile for production-grade Rust application
# Optimized for security, performance, and minimal attack surface

# ==============================================================================
# Build Stage - Alpine Linux for smaller intermediate layers
# ==============================================================================
FROM rustlang/rust:nightly-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    ca-certificates \
    curl \
    unzip

# Install protoc manually from official releases for better compatibility
RUN PROTOC_VERSION=25.1 && \
    PROTOC_ARCH=linux-x86_64 && \
    curl -LO "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-${PROTOC_ARCH}.zip" && \
    unzip "protoc-${PROTOC_VERSION}-${PROTOC_ARCH}.zip" -d /usr/local && \
    rm "protoc-${PROTOC_VERSION}-${PROTOC_ARCH}.zip" && \
    chmod +x /usr/local/bin/protoc

# Create app user for security
RUN addgroup -g 1000 appgroup && \
    adduser -D -s /bin/sh -u 1000 -G appgroup appuser

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY proto/ ./proto/
COPY build.rs ./

# Build the actual application
# Use SQLX_OFFLINE=true to avoid database requirement during build
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin server

# Strip binary to reduce size
RUN strip target/release/server

# ==============================================================================
# Security Scanner Stage (Optional - can be used in CI/CD)
# ==============================================================================
FROM builder AS security-scan
RUN cargo audit --deny warnings

# ==============================================================================
# Runtime Stage - Distroless for maximum security
# ==============================================================================
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

# Labels for metadata and best practices
LABEL \
    maintainer="racasantos@icloud.com" \
    version="1.0.0" \
    description="Production-grade Rust JWT Backend" \
    org.opencontainers.image.title="server" \
    org.opencontainers.image.description="Enterprise JWT authentication backend" \
    org.opencontainers.image.version="1.0.0" \
    org.opencontainers.image.vendor="Company Inc." \
    org.opencontainers.image.licenses="MIT" \
    org.opencontainers.image.source="https://github.com/zlovtnik/server"

# Copy the binary from builder stage
COPY --from=builder /app/target/release/server /usr/local/bin/server

# Copy SSL certificates for HTTPS requests
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set security-focused environment variables
ENV RUST_LOG=info
ENV RUFST_BACKTRACE=0
ENV ENVIRONMENT=production

# Use non-root user (distroless default)
USER nonroot:nonroot

# Expose ports (should match your application configuration)
# Port 3000 for REST API, Port 3001 for gRPC server
EXPOSE 3000 3001

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/server", "--health-check"] || exit 1

# Run the application
ENTRYPOINT ["/usr/local/bin/server"]