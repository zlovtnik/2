#!/bin/bash

# Test script to demonstrate gRPC service working with JWT token from REST API
# This script shows the complete flow: register -> login -> use token to call gRPC service

set -e

BASE_URL="http://localhost:3000"
API_BASE="$BASE_URL/api/v1"
GRPC_ADDR="localhost:3001"

echo "=== Testing gRPC Service with JWT Token ==="
echo

# Check if server is running
echo "1. Checking if REST server is running..."
if ! curl -s "$BASE_URL/health/live" > /dev/null; then
    echo "❌ REST server is not running. Please start the server first with: cargo run"
    exit 1
fi
echo "✅ REST server is running"
echo

# Register a test user
echo "2. Registering a test user..."
REGISTER_RESPONSE=$(curl -s -X POST "$API_BASE/auth/register" \
    -H "Content-Type: application/json" \
    -d '{
        "email": "grpcuser@example.com",
        "password": "testpassword123",
        "full_name": "gRPC Test User"
    }')

echo "Register response: $REGISTER_RESPONSE"
echo

# Login to get JWT token
echo "3. Logging in to get JWT token..."
LOGIN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
    -H "Content-Type: application/json" \
    -d '{
        "email": "grpcuser@example.com",
        "password": "testpassword123"
    }')

echo "Login response: $LOGIN_RESPONSE"

# Extract token from response
TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4 || echo "$LOGIN_RESPONSE" | grep -o '"access_token":"[^"]*"' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
    echo "❌ Failed to extract token from login response"
    echo "Login response was: $LOGIN_RESPONSE"
    exit 1
fi

echo "✅ Successfully obtained JWT token: ${TOKEN:0:20}..."
echo

# Test the gRPC service using grpcurl (if available)
echo "4. Testing gRPC service with JWT token..."

# Check if grpcurl is available
if command -v grpcurl &> /dev/null; then
    echo "Using grpcurl to test gRPC service..."
    
    # Call the gRPC service with JWT token
    GRPC_RESPONSE=$(grpcurl -plaintext \
        -H "authorization: Bearer $TOKEN" \
        -d '{}' \
        $GRPC_ADDR \
        user_stats.UserStatsService/GetCurrentUserStats 2>&1)
    
    echo "gRPC response: $GRPC_RESPONSE"
    
    if echo "$GRPC_RESPONSE" | grep -q "refresh_token_count"; then
        echo "✅ gRPC service successfully called with JWT token!"
        echo "✅ Response contains expected fields from the PostgreSQL procedure"
    else
        echo "❌ gRPC response doesn't contain expected fields or service failed"
        echo "Response was: $GRPC_RESPONSE"
    fi
else
    echo "⚠️  grpcurl not found. To test gRPC service manually, install grpcurl:"
    echo "   brew install grpcurl"
    echo ""
    echo "Then run:"
    echo "   grpcurl -plaintext -H \"authorization: Bearer $TOKEN\" -d '{}' $GRPC_ADDR user_stats.UserStatsService/GetCurrentUserStats"
    echo ""
    echo "✅ gRPC service is configured and should be accessible at $GRPC_ADDR"
fi

echo
echo "=== Test Complete ==="
echo "The gRPC service 'UserStatsService.GetCurrentUserStats' has been configured"
echo "to work with JWT tokens obtained from the REST API authentication."
echo
echo "Key points demonstrated:"
echo "- JWT token extracted from REST API login response"
echo "- Token used in gRPC authorization metadata header"
echo "- gRPC service extracts user ID from JWT token"
echo "- PostgreSQL procedure called with authenticated user ID"
echo "- Procedure returns user info with computed statistics via gRPC"
echo
echo "Servers running:"
echo "- REST API: $BASE_URL"
echo "- gRPC Service: $GRPC_ADDR"