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
echo "Output location:"
echo "  flutter/build/windows/x64/runner/Release/rustdesk.exe"
echo ""

# Check if executable exists
if [ -f "flutter/build/windows/x64/runner/Release/rustdesk.exe" ]; then
    SIZE=$(du -h "flutter/build/windows/x64/runner/Release/rustdesk.exe" | cut -f1)
    echo "Executable size: $SIZE"
    echo ""
    echo "========================================"
    echo " Build SUCCESS!"
    echo "========================================"
else
    echo "ERROR: rustdesk.exe not found!"
    exit 1
fi
