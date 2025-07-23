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
    
    # Create local bin directory in home if it doesn't exist
    mkdir -p $HOME/.local/bin
    mkdir -p $HOME/.local/include
    
    # Create temporary directory for extraction
    TEMP_DIR=$(mktemp -d)
    cd ${TEMP_DIR}
    
    # Download and install protoc
    echo "Downloading protoc ${PROTOC_VERSION}..."
    curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${PROTOC_ZIP}
    unzip -o ${PROTOC_ZIP}
    
    # Install to user's local directory (no sudo required)
    cp bin/protoc $HOME/.local/bin/protoc
    cp -r include/* $HOME/.local/include/ 2>/dev/null || true
    chmod +x $HOME/.local/bin/protoc
    
    # Add to PATH if not already there
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        export PATH="$HOME/.local/bin:$PATH"
    fi
    
    # Clean up
    cd /
    rm -rf ${TEMP_DIR}
    
    echo "protoc ${PROTOC_VERSION} installed successfully"
    echo "protoc location: $HOME/.local/bin/protoc"
    echo "protoc version: $($HOME/.local/bin/protoc --version)"
else
    echo "protoc is already installed: $(protoc --version)"
    echo "protoc location: $(which protoc)"
fi
