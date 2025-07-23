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
    
    # Check if sudo is available
    SUDO_CMD=""
    if command -v sudo &> /dev/null; then
        SUDO_CMD="sudo"
        echo "sudo is available, using it for package installation"
    else
        echo "sudo not available (likely in container environment), attempting without sudo"
    fi
    
    if command -v apt-get &> /dev/null; then
        # Ubuntu/Debian systems
        echo "Using apt-get package manager..."
        if [ -n "$SUDO_CMD" ]; then
            $SUDO_CMD apt-get update
            $SUDO_CMD apt-get install -y protobuf-compiler
        else
            # Try without sudo (for containers)
            apt-get update && apt-get install -y protobuf-compiler || {
                echo "Failed to install with apt-get, trying manual installation..."
                ./scripts/install-protoc.sh
            }
        fi
    elif command -v yum &> /dev/null; then
        # RHEL/CentOS systems
        echo "Using yum package manager..."
        if [ -n "$SUDO_CMD" ]; then
            $SUDO_CMD yum install -y protobuf-compiler
        else
            # Try without sudo (for containers)
            yum install -y protobuf-compiler || {
                echo "Failed to install with yum, trying manual installation..."
                ./scripts/install-protoc.sh
            }
        fi
    elif command -v apk &> /dev/null; then
        # Alpine Linux systems (common in containers)
        echo "Using apk package manager..."
        if [ -n "$SUDO_CMD" ]; then
            $SUDO_CMD apk add --no-cache protoc protobuf-dev
        else
            # Try without sudo (for containers)
            apk add --no-cache protoc protobuf-dev || {
                echo "Failed to install with apk, trying manual installation..."
                ./scripts/install-protoc.sh
            }
        fi
    elif command -v brew &> /dev/null; then
        # macOS with Homebrew (doesn't need sudo)
        echo "Using Homebrew package manager..."
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

# First check if protoc is already in PATH
if command -v protoc &> /dev/null && protoc --version &> /dev/null; then
    PROTOC_PATH=$(which protoc)
    echo "Found protoc in PATH: $PROTOC_PATH"
    export PROTOC="$PROTOC_PATH"
    
    # Set standard include paths for system-installed protoc, but only include existing directories
    POTENTIAL_INCLUDES=()
    if [[ "$PROTOC_PATH" == "/opt/homebrew/bin/protoc" ]]; then
        POTENTIAL_INCLUDES+=("/opt/homebrew/include")
    elif [[ "$PROTOC_PATH" == "/usr/local/bin/protoc" ]]; then
        POTENTIAL_INCLUDES+=("/usr/local/include")
    fi
    
    # Only add system directories if they exist
    [ -d "/usr/include" ] && POTENTIAL_INCLUDES+=("/usr/include")
    [ -d "/usr/local/include" ] && POTENTIAL_INCLUDES+=("/usr/local/include")
    
    # Build PROTOC_INCLUDE with only existing directories
    EXISTING_INCLUDES=()
    for include_dir in "${POTENTIAL_INCLUDES[@]}"; do
        if [ -d "$include_dir" ]; then
            EXISTING_INCLUDES+=("$include_dir")
        fi
    done
    
    # Join the existing directories with colons
    if [ ${#EXISTING_INCLUDES[@]} -gt 0 ]; then
        export PROTOC_INCLUDE=$(IFS=:; echo "${EXISTING_INCLUDES[*]}")
        echo "Set PROTOC_INCLUDE to: $PROTOC_INCLUDE"
    else
        # If no standard directories exist, don't set PROTOC_INCLUDE
        echo "Warning: No standard include directories found, PROTOC_INCLUDE will not be set"
        unset PROTOC_INCLUDE
    fi
else
    # If not in PATH, check common installation directories from install-protoc.sh
    echo "protoc not found in PATH, checking common installation directories..."
    
    # Check the directories that install-protoc.sh uses in order of preference
    POTENTIAL_PATHS=(
        "$HOME/.local/bin/protoc"
        "$(pwd)/local/bin/protoc"
        "/tmp/protoc-install/bin/protoc"
    )
    
    PROTOC_FOUND=false
    for PROTOC_CANDIDATE in "${POTENTIAL_PATHS[@]}"; do
        if [ -f "$PROTOC_CANDIDATE" ] && [ -x "$PROTOC_CANDIDATE" ]; then
            echo "Found protoc at: $PROTOC_CANDIDATE"
            PROTOC_DIR=$(dirname "$PROTOC_CANDIDATE")
            INSTALL_DIR=$(dirname "$PROTOC_DIR")
            
            # Add to PATH
            export PATH="$PROTOC_DIR:$PATH"
            export PROTOC="$PROTOC_CANDIDATE"
            
            # Set include path based on installation directory, but only include existing directories
            POTENTIAL_INCLUDES=()
            [ -d "$INSTALL_DIR/include" ] && POTENTIAL_INCLUDES+=("$INSTALL_DIR/include")
            [ -d "/usr/include" ] && POTENTIAL_INCLUDES+=("/usr/include")
            [ -d "/usr/local/include" ] && POTENTIAL_INCLUDES+=("/usr/local/include")
            
            # Build PROTOC_INCLUDE with only existing directories
            EXISTING_INCLUDES=()
            for include_dir in "${POTENTIAL_INCLUDES[@]}"; do
                if [ -d "$include_dir" ]; then
                    EXISTING_INCLUDES+=("$include_dir")
                fi
            done
            
            # Join the existing directories with colons
            if [ ${#EXISTING_INCLUDES[@]} -gt 0 ]; then
                export PROTOC_INCLUDE=$(IFS=:; echo "${EXISTING_INCLUDES[*]}")
                echo "Set PROTOC_INCLUDE to: $PROTOC_INCLUDE"
            else
                # If no include directories exist, don't set PROTOC_INCLUDE
                echo "Warning: No include directories found, PROTOC_INCLUDE will not be set"
                unset PROTOC_INCLUDE
            fi
            
            PROTOC_FOUND=true
            break
        fi
    done
    
    if [ "$PROTOC_FOUND" = false ]; then
        echo "Error: protoc not found after installation"
        echo "PATH: $PATH"
        echo "Checked locations:"
        for path in "${POTENTIAL_PATHS[@]}"; do
            echo "  - $path"
        done
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
