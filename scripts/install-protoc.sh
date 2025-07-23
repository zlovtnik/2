#!/bin/bash
set -e

# Check if protoc is already installed and working
if command -v protoc &> /dev/null && protoc --version &> /dev/null; then
    echo "protoc is already installed and working: $(protoc --version)"
    echo "protoc location: $(which protoc)"
    exit 0
fi

# Install protoc if not already installed or not working
echo "protoc not found or not working, installing..."
    PROTOC_VERSION=25.1
    PROTOC_ARCH=linux-x86_64
    PROTOC_ZIP=protoc-${PROTOC_VERSION}-${PROTOC_ARCH}.zip
    
    # Determine installation directory - prefer $HOME/.local but fallback to writable alternatives
    INSTALL_DIR="$HOME/.local"
    
    # Check if we can write to $HOME/.local, if not use alternatives
    if ! mkdir -p "$HOME/.local/bin" 2>/dev/null || ! touch "$HOME/.local/bin/.test" 2>/dev/null; then
        echo "Cannot write to $HOME/.local/bin, trying alternative locations..."
        rm -f "$HOME/.local/bin/.test" 2>/dev/null || true
        
        # Try current working directory
        if mkdir -p "./local/bin" 2>/dev/null && touch "./local/bin/.test" 2>/dev/null; then
            INSTALL_DIR="$(pwd)/local"
            echo "Using local installation directory: $INSTALL_DIR"
            rm -f "./local/bin/.test"
        # Try /tmp as last resort
        elif mkdir -p "/tmp/protoc-install/bin" 2>/dev/null && touch "/tmp/protoc-install/bin/.test" 2>/dev/null; then
            INSTALL_DIR="/tmp/protoc-install"
            echo "Using temporary installation directory: $INSTALL_DIR"
            rm -f "/tmp/protoc-install/bin/.test"
        else
            echo "Error: Cannot find a writable directory for protoc installation"
            echo "Tried: $HOME/.local, $(pwd)/local, /tmp/protoc-install"
            exit 1
        fi
    else
        rm -f "$HOME/.local/bin/.test" 2>/dev/null || true
        echo "Using standard installation directory: $INSTALL_DIR"
    fi
    
    # Create the directories
    mkdir -p "$INSTALL_DIR/bin"
    mkdir -p "$INSTALL_DIR/include"
    
    # Create temporary directory for extraction
    TEMP_DIR=$(mktemp -d)
    cd ${TEMP_DIR}
    
    # Download and install protoc
    echo "Downloading protoc ${PROTOC_VERSION}..."
    curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${PROTOC_ZIP}
    unzip -o ${PROTOC_ZIP}
    
    # Install to the determined directory (no sudo required)
    cp bin/protoc "$INSTALL_DIR/bin/protoc"
    cp -r include/* "$INSTALL_DIR/include/" 2>/dev/null || true
    chmod +x "$INSTALL_DIR/bin/protoc"
    
    # Add to PATH if not already there
    if [[ ":$PATH:" != *":$INSTALL_DIR/bin:"* ]]; then
        export PATH="$INSTALL_DIR/bin:$PATH"
    fi
    
    # Clean up
    cd /
    rm -rf ${TEMP_DIR}
    
    echo "protoc ${PROTOC_VERSION} installed successfully"
    echo "protoc location: $INSTALL_DIR/bin/protoc"
    echo "protoc version: $($INSTALL_DIR/bin/protoc --version)"
