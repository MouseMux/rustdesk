#!/bin/bash
# MouseMux Flutter Edition Build Script

set -e  # Exit on error

echo "========================================"
echo " MouseMux Flutter Edition - Build Script"
echo "========================================"
echo ""

# Set environment variables
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
export PATH="/o/rustdesk-build/flutter/bin:$PATH"

echo "[1/4] Generating Flutter-Rust bridge files..."
flutter_rust_bridge_codegen \
  --rust-input ./src/flutter_ffi.rs \
  --dart-output ./flutter/lib/generated_bridge.dart \
  --c-output ./flutter/macos/Runner/bridge_generated.h

if [ $? -ne 0 ]; then
    echo "ERROR: Bridge generation failed!"
    exit 1
fi

echo ""
echo "[2/4] Building Flutter application..."
"/c/Program Files/Python313/python" build.py --flutter

if [ $? -ne 0 ]; then
    echo "ERROR: Flutter build failed!"
    exit 1
fi

echo ""
echo "[3/4] Build complete!"
echo ""

# Show completion time
echo "========================================"
echo " Build Completed At:"
echo "========================================"
date
echo ""

# List all generated executables
echo "========================================"
echo " Generated Executables:"
echo "========================================"
echo ""

# Main executable
if [ -f "flutter/build/windows/x64/runner/Release/rustdesk.exe" ]; then
    echo "Main executable:"
    ls -lh "flutter/build/windows/x64/runner/Release/rustdesk.exe" | awk '{print "  " $9 " - " $5 " - " $6 " " $7 " " $8}'
    echo ""
else
    echo "ERROR: Main rustdesk.exe not found!"
    exit 1
fi

# Installer (if it exists)
INSTALLER=$(ls rustdesk-*-install.exe 2>/dev/null | head -1)
if [ -n "$INSTALLER" ]; then
    echo "Installer:"
    ls -lh "$INSTALLER" | awk '{print "  " $9 " - " $5 " - " $6 " " $7 " " $8}'
    echo ""
fi

# Resources executable (if it exists)
if [ -f "resources/rustdesk.exe" ]; then
    echo "Resources executable:"
    ls -lh "resources/rustdesk.exe" | awk '{print "  " $9 " - " $5 " - " $6 " " $7 " " $8}'
    echo ""
fi

echo "========================================"
echo " Build SUCCESS!"
echo "========================================"
