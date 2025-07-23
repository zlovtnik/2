#!/bin/bash
echo "Testing install-protoc.sh for syntax errors..."
bash -n ./scripts/install-protoc.sh
if [ $? -eq 0 ]; then
    echo "No syntax errors found"
else
    echo "Syntax errors detected"
fi