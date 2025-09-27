#!/bin/bash
# Copy the built release executable to prod/ for deployment
# Usage: ./scripts/deploy_to_prod.sh

set -e

EXECUTABLE=server  # Change if your binary has a different name
BUILD_DIR=target/release
PROD_DIR=prod

# Ensure the executable exists
if [ ! -f "$BUILD_DIR/$EXECUTABLE" ]; then
  echo "Error: $BUILD_DIR/$EXECUTABLE not found. Build first with 'cargo build --release'."
  exit 1
fi


# Ensure prod directory exists
mkdir -p "$PROD_DIR"
# Clean prod directory by removing and recreating it
rm -rf "$PROD_DIR"
mkdir -p "$PROD_DIR"

# Copy the executable
cp "$BUILD_DIR/$EXECUTABLE" "$PROD_DIR/"

# (Optional) Copy any other required runtime files, e.g. migrations, static assets
# cp -r migrations "$PROD_DIR/"
# cp -r static "$PROD_DIR/"

chmod 700 "$PROD_DIR/$EXECUTABLE"
echo "Deployment package prepared in $PROD_DIR/"
