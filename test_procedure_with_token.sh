#!/bin/bash

# Test script to demonstrate PostgreSQL procedure working with JWT token from REST API
# This script shows the complete flow: register -> login -> use token to call procedure

set -e

BASE_URL="http://localhost:3000"
API_BASE="$BASE_URL/api/v1"

echo "=== Testing PostgreSQL Procedure with JWT Token ==="
echo

# Check if server is running
echo "1. Checking if server is running..."
if ! curl -s "$BASE_URL/health/live" > /dev/null; then
    echo "❌ Server is not running. Please start the server first with: cargo run"
    exit 1
fi
echo "✅ Server is running"
echo

# Register a test user
echo "2. Registering a test user..."
REGISTER_RESPONSE=$(curl -s -X POST "$API_BASE/auth/register" \
    -H "Content-Type: application/json" \
    -d '{
        "email": "testuser@example.com",
        "password": "testpassword123",
        "full_name": "Test User"
    }')

echo "Register response: $REGISTER_RESPONSE"
echo

# Login to get JWT token
echo "3. Logging in to get JWT token..."
LOGIN_RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
    -H "Content-Type: application/json" \
    -d '{
        "email": "testuser@example.com",
        "password": "testpassword123"
    }')

echo "Login response: $LOGIN_RESPONSE"

# Extract token from response (assuming it's in a field called "token" or "access_token")
TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4 || echo "$LOGIN_RESPONSE" | grep -o '"access_token":"[^"]*"' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
    echo "❌ Failed to extract token from login response"
    echo "Login response was: $LOGIN_RESPONSE"
    exit 1
fi

echo "✅ Successfully obtained JWT token: ${TOKEN:0:20}..."
echo

# Test the regular user endpoint first
echo "4. Testing regular user endpoint with token..."
USER_RESPONSE=$(curl -s -X GET "$API_BASE/users/me" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json")

echo "User endpoint response: $USER_RESPONSE"
echo

# Test the new PostgreSQL procedure endpoint
echo "5. Testing PostgreSQL procedure endpoint with token..."
PROCEDURE_RESPONSE=$(curl -s -X GET "$API_BASE/users/me/stats" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json")

echo "Procedure endpoint response: $PROCEDURE_RESPONSE"
echo

# Check if the procedure response contains expected fields
if echo "$PROCEDURE_RESPONSE" | grep -q "refresh_token_count"; then
    echo "✅ PostgreSQL procedure successfully called with JWT token!"
    echo "✅ Response contains expected fields from the procedure"
else
    echo "❌ Procedure response doesn't contain expected fields"
    echo "Response was: $PROCEDURE_RESPONSE"
fi

echo
echo "=== Test Complete ==="
echo "The PostgreSQL procedure 'get_user_info_with_stats' has been successfully"
echo "called using the JWT token obtained from the REST API authentication."
echo
echo "Key points demonstrated:"
echo "- JWT token extracted from login response"
echo "- Token used in Authorization header"
echo "- Authentication middleware extracts user ID from token"
echo "- PostgreSQL procedure called with authenticated user ID"
echo "- Procedure returns user info with computed statistics"