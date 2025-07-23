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

# Build the project
echo "Building the project..."
cargo build --release

echo "Build completed successfully!"
