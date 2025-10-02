#!/bin/bash
# Copy the built release executable to prod/ for deployment
# Usage: ./scripts/deploy_to_prod.sh

set -euo pipefail

EXECUTABLE=server  # Change if your binary has a different name
BUILD_DIR=target/release
PROD_DIR=prod

# Ensure the executable exists
if [ ! -f "$BUILD_DIR/$EXECUTABLE" ]; then
  echo "Error: $BUILD_DIR/$EXECUTABLE not found. Build first with 'cargo build --release'."
  exit 1
fi

# Safety checks before cleaning the prod directory
if [ -z "$PROD_DIR" ]; then
  echo "Error: PROD_DIR is not set."
  exit 1
fi

if [ "$PROD_DIR" = "/" ]; then
  echo "Error: PROD_DIR cannot be '/'."
  exit 1
fi

if [[ "$PROD_DIR" == /* && "$PROD_DIR" != /*/* ]]; then
  echo "Error: PROD_DIR '$PROD_DIR' is a top-level directory. Refusing to remove it."
  exit 1
fi

if [[ "$PROD_DIR" =~ ^\.{1,2}$ ]]; then
  echo "Error: PROD_DIR cannot be '.' or '..'."
  exit 1
fi

if [ ${#PROD_DIR} -lt 2 ]; then
  echo "Error: PROD_DIR '$PROD_DIR' is too short to be safe."
  exit 1
fi

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
