#!/bin/bash
set -e

# Install protoc if not already installed
./scripts/install-protoc.sh

# Add local bin to PATH if not already there
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    export PATH="$HOME/.local/bin:$PATH"
fi

# Set up environment variables - check multiple possible locations
if [ -f "$HOME/.local/bin/protoc" ]; then
    export PROTOC="$HOME/.local/bin/protoc"
    export PROTOC_INCLUDE="$HOME/.local/include:/usr/include:/usr/local/include"
elif [ -f "/usr/local/bin/protoc" ]; then
    export PROTOC="/usr/local/bin/protoc"
    export PROTOC_INCLUDE="/usr/local/include:/usr/include"
elif [ -f "/usr/bin/protoc" ]; then
    export PROTOC="/usr/bin/protoc"
    export PROTOC_INCLUDE="/usr/include:/usr/local/include"
else
    echo "Error: protoc not found in any expected location"
    exit 1
fi

echo "Using protoc at: $PROTOC"
echo "PROTOC_INCLUDE: $PROTOC_INCLUDE"
echo "protoc version: $($PROTOC --version)"

# Build the project
echo "Building the project..."
cargo build --release

echo "Build completed successfully!"
