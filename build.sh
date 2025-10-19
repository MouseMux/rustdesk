#!/bin/bash
# MouseMux RustDesk Flutter - Build Script
# This builds the latest RustDesk with MouseMux V2.1 protocol support

set -e  # Exit on any error

echo "========================================"
echo " MouseMux RustDesk Flutter - Build"
echo "========================================"
echo ""

# Check prerequisites
echo "[Prerequisites] Checking environment..."

# Check VCPKG_ROOT
if [ -z "$VCPKG_ROOT" ]; then
    echo "ERROR: VCPKG_ROOT environment variable is not set!"
    echo "Please set it to your vcpkg installation path."
    echo "Example: export VCPKG_ROOT=/o/rustdesk-build/vcpkg"
    exit 1
fi
echo "✓ VCPKG_ROOT: $VCPKG_ROOT"

# Check Rust version
if ! command -v rustc &> /dev/null; then
    echo "ERROR: Rust is not installed!"
    exit 1
fi
RUST_VERSION=$(rustc --version)
echo "✓ Rust: $RUST_VERSION"

# Check Python
if ! command -v python &> /dev/null; then
    echo "ERROR: Python is not installed!"
    exit 1
fi
PYTHON_VERSION=$(python --version)
echo "✓ Python: $PYTHON_VERSION"

echo ""
echo "========================================"
echo " Building MouseMux RustDesk Flutter"
echo "========================================"
echo ""

# Build using Python script
echo "[1/2] Building RustDesk with Flutter UI..."
echo ""
python build.py --flutter

if [ $? -ne 0 ]; then
    echo ""
    echo "ERROR: Build failed!"
    exit 1
fi

echo ""
echo "[2/2] Build complete!"
echo ""

# Show build results
echo "========================================"
echo " Build Results"
echo "========================================"
echo ""

# Check for Flutter executable
FLUTTER_EXE="flutter/build/windows/x64/runner/Release/rustdesk.exe"
if [ -f "$FLUTTER_EXE" ]; then
    echo "✓ Flutter executable built:"
    ls -lh "$FLUTTER_EXE" | awk '{print "  " $9 " (" $5 ")"}'
else
    echo "✗ Flutter executable not found at: $FLUTTER_EXE"
fi

echo ""

# Check for installer
INSTALLER=$(ls rustdesk-*-install.exe 2>/dev/null | head -1)
if [ -n "$INSTALLER" ]; then
    echo "✓ Installer created:"
    ls -lh "$INSTALLER" | awk '{print "  " $9 " (" $5 ")"}'
else
    echo "ℹ No installer found (this is normal if installer creation was skipped)"
fi

echo ""
echo "========================================"
echo " Build SUCCESS!"
echo "========================================"
echo ""
echo "Built at: $(date)"
echo ""
echo "This build includes MouseMux V2.1 protocol support."
echo "Multiple users can now connect simultaneously with"
echo "independent mouse cursors using the MouseMux application."
echo ""
