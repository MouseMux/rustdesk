#!/bin/bash
# RustDesk MouseMux Edition - Complete Clean Script
# Removes all executables and build artifacts

set -e  # Exit on error

PROJECT_ROOT="/o/rustdesk-build/rustdesk"

echo "========================================"
echo "RustDesk MouseMux Edition Clean Script"
echo "========================================"
echo ""

cd "$PROJECT_ROOT"

echo "[1/4] Cleaning Cargo build artifacts..."
cargo clean
echo "✓ Cargo clean complete"
echo ""

echo "[2/4] Removing executables from resources/..."
rm -f resources/rustdesk.exe
rm -f resources/RustDesk.exe
rm -f resources/sciter.dll
echo "✓ Resources cleaned"
echo ""

echo "[3/4] Removing portable packer artifacts..."
rm -f libs/portable/data.bin
rm -f libs/portable/rustdesk-portable-packer.exe
echo "✓ Portable artifacts cleaned"
echo ""

echo "[4/4] Removing any MouseMux edition builds..."
rm -f rustdesk-*-mousemux-*.exe
rm -f rustdesk-*-x86_64*.exe
echo "✓ MouseMux builds cleaned"
echo ""

echo "========================================"
echo "Clean complete!"
echo "========================================"
echo ""
echo "All build artifacts and executables have been removed."
echo "Run ./build-mousemux-edition.sh to rebuild from scratch."
echo ""
