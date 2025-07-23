#!/bin/bash

# Test script to verify protoc issue is resolved
echo "Testing protoc compilation fix..."

# Clean previous builds
echo "Cleaning previous builds..."
cargo clean

# Test local build (should work since protoc is available locally)
echo "Testing local build..."
if cargo build --release; then
    echo "✅ Local build successful"
else
    echo "❌ Local build failed"
    exit 1
fi

# Test Docker build (this would test the dockerfile changes)
echo "Testing Docker build..."
if docker build -t rust-jwt-backend-test .; then
    echo "✅ Docker build successful"
else
    echo "❌ Docker build failed"
    exit 1
fi

echo "🎉 All tests passed! The protoc issue has been resolved."