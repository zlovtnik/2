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
    # Temporarily disable exit on error for directory testing
    set +e
    mkdir -p "$HOME/.local/bin" 2>/dev/null
    HOME_LOCAL_MKDIR_SUCCESS=$?
    touch "$HOME/.local/bin/.test" 2>/dev/null
    HOME_LOCAL_TOUCH_SUCCESS=$?
    set -e
    
    if [ $HOME_LOCAL_MKDIR_SUCCESS -ne 0 ] || [ $HOME_LOCAL_TOUCH_SUCCESS -ne 0 ]; then
        echo "Cannot write to $HOME/.local/bin, trying alternative locations..."
        rm -f "$HOME/.local/bin/.test" 2>/dev/null || true
        
        # Try current working directory
        set +e
        mkdir -p "./local/bin" 2>/dev/null
        LOCAL_MKDIR_SUCCESS=$?
        touch "./local/bin/.test" 2>/dev/null
        LOCAL_TOUCH_SUCCESS=$?
        set -e
        
        if [ $LOCAL_MKDIR_SUCCESS -eq 0 ] && [ $LOCAL_TOUCH_SUCCESS -eq 0 ]; then
            INSTALL_DIR="$(pwd)/local"
            echo "Using local installation directory: $INSTALL_DIR"
            rm -f "./local/bin/.test"
        else
            # Try /tmp as last resort
            set +e
            mkdir -p "/tmp/protoc-install/bin" 2>/dev/null
            TMP_MKDIR_SUCCESS=$?
            touch "/tmp/protoc-install/bin/.test" 2>/dev/null
            TMP_TOUCH_SUCCESS=$?
            set -e
            
            if [ $TMP_MKDIR_SUCCESS -eq 0 ] && [ $TMP_TOUCH_SUCCESS -eq 0 ]; then
                INSTALL_DIR="/tmp/protoc-install"
                echo "Using temporary installation directory: $INSTALL_DIR"
                rm -f "/tmp/protoc-install/bin/.test"
            else
                echo "Error: Cannot find a writable directory for protoc installation"
                echo "Tried: $HOME/.local, $(pwd)/local, /tmp/protoc-install"
                echo "Debug info:"
                echo "  HOME_LOCAL_MKDIR_SUCCESS: $HOME_LOCAL_MKDIR_SUCCESS"
                echo "  HOME_LOCAL_TOUCH_SUCCESS: $HOME_LOCAL_TOUCH_SUCCESS"
                echo "  LOCAL_MKDIR_SUCCESS: $LOCAL_MKDIR_SUCCESS"
                echo "  LOCAL_TOUCH_SUCCESS: $LOCAL_TOUCH_SUCCESS"
                echo "  TMP_MKDIR_SUCCESS: $TMP_MKDIR_SUCCESS"
                echo "  TMP_TOUCH_SUCCESS: $TMP_TOUCH_SUCCESS"
                exit 1
            fi
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
