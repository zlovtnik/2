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
    # Add local bin to PATH and check our local installation
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        export PATH="$HOME/.local/bin:$PATH"
    fi
    
    if [ -f "$HOME/.local/bin/protoc" ] && $HOME/.local/bin/protoc --version &> /dev/null; then
        echo "Using locally installed protoc"
        export PROTOC="$HOME/.local/bin/protoc"
        export PROTOC_INCLUDE="$HOME/.local/include:/usr/include:/usr/local/include"
    else
        echo "Error: protoc not found in any expected location"
        echo "Checked locations:"
        echo "  - /opt/homebrew/bin/protoc: $([ -f "/opt/homebrew/bin/protoc" ] && echo 'exists' || echo 'not found')"
        echo "  - /usr/local/bin/protoc: $([ -f "/usr/local/bin/protoc" ] && echo 'exists' || echo 'not found')"
        echo "  - /usr/bin/protoc: $([ -f "/usr/bin/protoc" ] && echo 'exists' || echo 'not found')"
        echo "  - $HOME/.local/bin/protoc: $([ -f "$HOME/.local/bin/protoc" ] && echo 'exists' || echo 'not found')"
        exit 1
    fi
fi

echo "Using protoc at: $PROTOC"
echo "PROTOC_INCLUDE: $PROTOC_INCLUDE"
echo "protoc version: $($PROTOC --version)"

# Build the project
echo "Building the project..."
cargo build --release

echo "Build completed successfully!"
