#!/bin/bash
set -e

echo "=== Testing protoc installation permission fix ==="

# Simulate the Render environment by making $HOME/.local unwritable
TEST_HOME="/tmp/test_protoc_fix"
mkdir -p "$TEST_HOME"
export HOME="$TEST_HOME"

# Create a non-writable .local directory to simulate the permission issue
mkdir -p "$TEST_HOME/.local"
chmod 000 "$TEST_HOME/.local"

echo "Test environment setup:"
echo "HOME: $HOME"
echo "Current directory: $(pwd)"
echo "Testing with non-writable $HOME/.local directory"

# Test the install script
echo ""
echo "=== Testing install-protoc.sh ==="
if ./scripts/install-protoc.sh; then
    echo "✓ install-protoc.sh completed successfully"
else
    echo "✗ install-protoc.sh failed"
    exit 1
fi

# Check if protoc was installed to an alternative location
echo ""
echo "=== Checking protoc installation ==="
if command -v protoc &> /dev/null; then
    echo "✓ protoc found in PATH: $(which protoc)"
    echo "✓ protoc version: $(protoc --version)"
else
    echo "✗ protoc not found in PATH"
    
    # Check alternative locations
    if [ -f "./local/bin/protoc" ]; then
        echo "✓ protoc found at ./local/bin/protoc"
    elif [ -f "/tmp/protoc-install/bin/protoc" ]; then
        echo "✓ protoc found at /tmp/protoc-install/bin/protoc"
    else
        echo "✗ protoc not found in any alternative location"
        exit 1
    fi
fi

# Test the build script (just the protoc detection part)
echo ""
echo "=== Testing build.sh protoc detection ==="
# We'll just test the protoc detection part, not the full build
if ./scripts/build.sh 2>&1 | grep -q "Using protoc at:"; then
    echo "✓ build.sh successfully detected protoc"
else
    echo "✗ build.sh failed to detect protoc"
    exit 1
fi

# Cleanup
echo ""
echo "=== Cleanup ==="
chmod 755 "$TEST_HOME/.local" 2>/dev/null || true
rm -rf "$TEST_HOME"
rm -rf "./local" 2>/dev/null || true
rm -rf "/tmp/protoc-install" 2>/dev/null || true

echo "✓ All tests passed! The permission fix is working correctly."