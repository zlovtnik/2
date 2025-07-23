#!/bin/bash
set -e

# Install protoc if not already installed
if ! command -v protoc &> /dev/null; then
    echo "Installing protoc..."
    PROTOC_VERSION=25.1
    PROTOC_ARCH=linux-x86_64
    PROTOC_ZIP=protoc-${PROTOC_VERSION}-${PROTOC_ARCH}.zip
    
    # Download and install protoc
    curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${PROTOC_ZIP}
    unzip -o ${PROTOC_ZIP} -d /usr/local bin/protoc
    unzip -o ${PROTOC_ZIP} -d /usr/local 'include/*'
    rm -f ${PROTOC_ZIP}
    chmod +x /usr/local/bin/protoc
    
    echo "protoc ${PROTOC_VERSION} installed successfully"
else
    echo "protoc is already installed: $(protoc --version)"
fi
