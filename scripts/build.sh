#!/bin/bash
set -e

echo "=== Starting build process ==="
echo "Current working directory: $(pwd)"
echo "Current PATH: $PATH"
echo "Current user: $(whoami)"
echo "Home directory: $HOME"

# Install protoc if not already installed
echo "=== Installing protoc ==="
if ! command -v protoc &> /dev/null; then
    echo "protoc not found, installing via package manager..."
    if command -v apt-get &> /dev/null; then
        # Ubuntu/Debian systems
        sudo apt-get update
        sudo apt-get install -y protobuf-compiler
    elif command -v yum &> /dev/null; then
        # RHEL/CentOS systems
        sudo yum install -y protobuf-compiler
    elif command -v brew &> /dev/null; then
        # macOS with Homebrew
        brew install protobuf
    else
        echo "No supported package manager found. Falling back to manual installation..."
        ./scripts/install-protoc.sh
    fi
else
    echo "protoc is already installed: $(protoc --version)"
fi

# Set up environment variables - use protoc from PATH
echo "=== Setting up protoc environment ==="

if command -v protoc &> /dev/null && protoc --version &> /dev/null; then
    PROTOC_PATH=$(which protoc)
    echo "Found protoc in PATH: $PROTOC_PATH"
    export PROTOC="$PROTOC_PATH"
    
    # Set standard include paths for system-installed protoc
    if [[ "$PROTOC_PATH" == "/opt/homebrew/bin/protoc" ]]; then
        export PROTOC_INCLUDE="/opt/homebrew/include:/usr/include:/usr/local/include"
    elif [[ "$PROTOC_PATH" == "/usr/local/bin/protoc" ]]; then
        export PROTOC_INCLUDE="/usr/local/include:/usr/include"
    else
        export PROTOC_INCLUDE="/usr/include:/usr/local/include"
    fi
else
    echo "Error: protoc not found after installation"
    echo "PATH: $PATH"
    exit 1
fi

echo "Using protoc at: $PROTOC"
echo "PROTOC_INCLUDE: $PROTOC_INCLUDE"
echo "protoc version: $($PROTOC --version)"

# Build the project
echo "Building the project..."
cargo build --release

echo "Build completed successfully!"
