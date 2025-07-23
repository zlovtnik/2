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

# Temporarily hide existing protoc installations to force the installation scenario
echo ""
echo "=== Hiding existing protoc installations ==="
ORIGINAL_PATH="$PATH"
# Remove common protoc locations from PATH to force installation, but preserve essential system directories
export PATH=$(echo "$PATH" | sed 's|/opt/homebrew/bin:||g' | sed 's|:/opt/homebrew/bin||g' | sed 's|/usr/local/bin:||g' | sed 's|:/usr/local/bin||g')
# Ensure essential system directories are still in PATH
if [[ ":$PATH:" != *":/usr/bin:"* ]]; then
    export PATH="$PATH:/usr/bin"
fi
if [[ ":$PATH:" != *":/bin:"* ]]; then
    export PATH="$PATH:/bin"
fi
echo "Modified PATH: $PATH"

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
# Create a modified build script that stops after protoc detection
cat > /tmp/test_build_protoc.sh << 'EOF'
#!/bin/bash
set -e

echo "=== Starting build process ==="
echo "Current working directory: $(pwd)"
echo "Current PATH: $PATH"
echo "Current user: $(whoami)"
echo "Home directory: $HOME"

# Install protoc if not already installed
echo "=== Installing protoc ==="
./scripts/install-protoc.sh

# Set up environment variables - check multiple possible locations
echo "=== Setting up protoc environment ==="

# Check common system locations first (before adding local bin to PATH)
if [ -f "/opt/homebrew/bin/protoc" ] && /opt/homebrew/bin/protoc --version &> /dev/null; then
    echo "Found working Homebrew protoc"
    export PROTOC="/opt/homebrew/bin/protoc"
    export PROTOC_INCLUDE="/opt/homebrew/include:/usr/include:/usr/local/include"
elif [ -f "/usr/local/bin/protoc" ] && /usr/local/bin/protoc --version &> /dev/null; then
    echo "Found working system protoc at /usr/local/bin"
    export PROTOC="/usr/local/bin/protoc"
    export PROTOC_INCLUDE="/usr/local/include:/usr/include"
elif [ -f "/usr/bin/protoc" ] && /usr/bin/protoc --version &> /dev/null; then
    echo "Found working system protoc at /usr/bin"
    export PROTOC="/usr/bin/protoc"
    export PROTOC_INCLUDE="/usr/include:/usr/local/include"
else
    # Check for locally installed protoc in various possible locations
    # The install-protoc.sh script may have installed to alternative locations
    
    # First, try to find protoc in PATH (install-protoc.sh adds its location to PATH)
    if command -v protoc &> /dev/null && protoc --version &> /dev/null; then
        PROTOC_PATH=$(which protoc)
        echo "Found protoc in PATH: $PROTOC_PATH"
        export PROTOC="$PROTOC_PATH"
        
        # Determine include directory based on protoc location
        PROTOC_DIR=$(dirname "$PROTOC_PATH")
        PROTOC_BASE=$(dirname "$PROTOC_DIR")
        if [ -d "$PROTOC_BASE/include" ]; then
            export PROTOC_INCLUDE="$PROTOC_BASE/include:/usr/include:/usr/local/include"
        else
            export PROTOC_INCLUDE="/usr/include:/usr/local/include"
        fi
    else
        # Fallback: check specific locations including alternative install directories
        PROTOC_LOCATIONS=(
            "$HOME/.local/bin/protoc"
            "./local/bin/protoc"
            "/tmp/protoc-install/bin/protoc"
        )
        
        FOUND_PROTOC=""
        for location in "${PROTOC_LOCATIONS[@]}"; do
            if [ -f "$location" ] && "$location" --version &> /dev/null; then
                FOUND_PROTOC="$location"
                break
            fi
        done
        
        if [ -n "$FOUND_PROTOC" ]; then
            echo "Using locally installed protoc at: $FOUND_PROTOC"
            export PROTOC="$FOUND_PROTOC"
            
            # Set include directory based on protoc location
            PROTOC_DIR=$(dirname "$FOUND_PROTOC")
            PROTOC_BASE=$(dirname "$PROTOC_DIR")
            if [ -d "$PROTOC_BASE/include" ]; then
                export PROTOC_INCLUDE="$PROTOC_BASE/include:/usr/include:/usr/local/include"
            else
                export PROTOC_INCLUDE="/usr/include:/usr/local/include"
            fi
            
            # Add to PATH if not already there
            if [[ ":$PATH:" != *":$PROTOC_DIR:"* ]]; then
                export PATH="$PROTOC_DIR:$PATH"
            fi
        else
            echo "Error: protoc not found in any expected location"
            echo "Checked locations:"
            echo "  - /opt/homebrew/bin/protoc: $([ -f "/opt/homebrew/bin/protoc" ] && echo 'exists' || echo 'not found')"
            echo "  - /usr/local/bin/protoc: $([ -f "/usr/local/bin/protoc" ] && echo 'exists' || echo 'not found')"
            echo "  - /usr/bin/protoc: $([ -f "/usr/bin/protoc" ] && echo 'exists' || echo 'not found')"
            echo "  - $HOME/.local/bin/protoc: $([ -f "$HOME/.local/bin/protoc" ] && echo 'exists' || echo 'not found')"
            echo "  - ./local/bin/protoc: $([ -f "./local/bin/protoc" ] && echo 'exists' || echo 'not found')"
            echo "  - /tmp/protoc-install/bin/protoc: $([ -f "/tmp/protoc-install/bin/protoc" ] && echo 'exists' || echo 'not found')"
            echo "PATH: $PATH"
            exit 1
        fi
    fi
fi

echo "Using protoc at: $PROTOC"
echo "PROTOC_INCLUDE: $PROTOC_INCLUDE"
echo "protoc version: $($PROTOC --version)"

echo "Protoc detection completed successfully!"
EOF

chmod +x /tmp/test_build_protoc.sh

# Run the modified build script
BUILD_OUTPUT=$(/tmp/test_build_protoc.sh 2>&1)
if echo "$BUILD_OUTPUT" | grep -q "Using protoc at:"; then
    echo "✓ build.sh successfully detected protoc"
    PROTOC_LOCATION=$(echo "$BUILD_OUTPUT" | grep "Using protoc at:" | cut -d' ' -f4)
    echo "  Detected protoc at: $PROTOC_LOCATION"
    
    # Verify the detected protoc actually works
    if [ -f "$PROTOC_LOCATION" ] && "$PROTOC_LOCATION" --version &> /dev/null; then
        echo "✓ Detected protoc is working: $($PROTOC_LOCATION --version)"
    else
        echo "✗ Detected protoc is not working"
        exit 1
    fi
else
    echo "✗ build.sh failed to detect protoc"
    echo "Build output:"
    echo "$BUILD_OUTPUT"
    exit 1
fi

# Clean up the temporary script
rm -f /tmp/test_build_protoc.sh

# Cleanup
echo ""
echo "=== Cleanup ==="
chmod 755 "$TEST_HOME/.local" 2>/dev/null || true
rm -rf "$TEST_HOME"
rm -rf "./local" 2>/dev/null || true
rm -rf "/tmp/protoc-install" 2>/dev/null || true

echo "✓ All tests passed! The permission fix is working correctly."