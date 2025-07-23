#!/bin/bash

# Test script to verify protoc fix in Docker build
set -e

echo "🔧 Testing protoc fix in Docker build..."

# Build the Docker image
echo "📦 Building Docker image..."
docker build -t server-protoc-test .

if [ $? -eq 0 ]; then
    echo "✅ Docker build succeeded! protoc fix is working."
    
    # Optional: Test that protoc is available in the build stage
    echo "🔍 Verifying protoc installation..."
    docker run --rm server-protoc-test protoc --version 2>/dev/null || echo "Note: protoc not available in runtime stage (expected for distroless)"
    
    echo "🎉 All tests passed!"
else
    echo "❌ Docker build failed. protoc fix needs more work."
    exit 1
fi