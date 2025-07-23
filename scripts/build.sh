#!/bin/bash
set -e

# Set up environment variables
export PROTOC=/usr/bin/protoc
export PROTOC_INCLUDE="/usr/include:/usr/local/include"

# Install protoc if not already installed
./scripts/install-protoc.sh

# Build the project
echo "Building the project..."
cargo build --release

echo "Build completed successfully!"
