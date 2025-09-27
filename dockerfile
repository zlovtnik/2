
# Dockerfile for deploying pre-built Rust executable from prod/
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


# Copy the pre-built binary from prod directory
COPY prod/server /usr/local/bin/server

# Copy SSL certificates for HTTPS requests (optional, comment out if not needed)
# COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

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