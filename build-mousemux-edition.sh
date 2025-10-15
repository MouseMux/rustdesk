#!/bin/bash
# RustDesk MouseMux Edition Build Script
# Complete build script for RustDesk with MouseMux V2.1 integration

set -e  # Exit on error

# Configuration
PROJECT_ROOT="/o/rustdesk-build/rustdesk"
export VCPKG_ROOT="/o/rustdesk-build/vcpkg"

echo "========================================"
echo "RustDesk MouseMux Edition Build Script"
echo "========================================"
echo ""

# Change to project directory
cd "$PROJECT_ROOT"

echo "[1/5] Building main RustDesk executable..."
cargo clean -p rustdesk
cargo build --release --bin rustdesk
echo "✓ rustdesk.exe built successfully"
echo ""

echo "[2/5] Copying files to resources folder..."
cp target/release/rustdesk.exe resources/rustdesk.exe
cp sciter.dll resources/sciter.dll
echo "✓ Files copied to resources/"
echo ""

echo "[3/5] Generating compressed data package..."
cd libs/portable
"/c/Program Files/Python313/python" generate.py \
    -f ../../resources \
    -o . \
    -e ../../resources/rustdesk.exe
cd "$PROJECT_ROOT"
echo "✓ data.bin created in libs/portable/"
echo ""

echo "[4/5] Building portable packer with embedded data..."
# Touch source file to force rebuild
touch libs/portable/src/main.rs
cargo build --release -p rustdesk-portable-packer
echo "✓ rustdesk-portable-packer.exe built"
echo ""

echo "[5/5] Verifying build outputs..."
echo ""
echo "Main executable:"
ls -lh target/release/rustdesk.exe
echo ""
echo "Portable installer:"
ls -lh target/release/rustdesk-portable-packer.exe
echo ""

echo "========================================"
echo "Build complete!"
echo "========================================"
echo ""
echo "Outputs:"
echo "  • Main exe:       target/release/rustdesk.exe"
echo "  • Portable packer: target/release/rustdesk-portable-packer.exe"
echo ""
echo "To test:"
echo "  1. Run rustdesk.exe directly (requires sciter.dll in same folder)"
echo "  2. Run rustdesk-portable-packer.exe (self-extracting, includes everything)"
echo ""
