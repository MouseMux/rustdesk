#!/bin/bash

echo "========================================="
echo "RustDesk MouseMux Flutter - Clean All"
echo "========================================="
echo ""

# Function to safely remove directories
safe_remove() {
    if [ -d "$1" ]; then
        echo "Removing: $1"
        rm -rf "$1"
    else
        echo "Skipping (not found): $1"
    fi
}

# Function to safely remove files
safe_remove_file() {
    if [ -f "$1" ]; then
        echo "Removing file: $1"
        rm -f "$1"
    else
        echo "Skipping file (not found): $1"
    fi
}

echo "Step 1: Cleaning Rust build artifacts..."
echo "-----------------------------------------"
safe_remove "target"
cargo clean 2>/dev/null || echo "cargo clean skipped (no Cargo.toml in current dir)"

echo ""
echo "Step 2: Cleaning Flutter build artifacts..."
echo "-----------------------------------------"
safe_remove "flutter/build"
safe_remove "flutter/.dart_tool"
safe_remove "flutter/.flutter-plugins"
safe_remove "flutter/.flutter-plugins-dependencies"
# NOTE: Do NOT remove pubspec.lock - it's needed by flutter_rust_bridge_codegen

echo ""
echo "Step 3: Cleaning Flutter cache..."
echo "-----------------------------------------"
cd flutter 2>/dev/null && flutter clean 2>/dev/null && cd .. || echo "flutter clean skipped"

echo ""
echo "Step 4: Cleaning portable packer artifacts..."
echo "-----------------------------------------"
safe_remove "libs/portable/target"
safe_remove_file "libs/portable/data.bin"
safe_remove "target/release/rustdesk-portable-packer.exe"
safe_remove "target/release/rustdesk-portable-packer"

echo ""
echo "Step 5: Cleaning installer outputs..."
echo "-----------------------------------------"
safe_remove_file "rustdesk-*.exe"
safe_remove_file "rustdesk-*.msi"
safe_remove_file "rustdesk-*.dmg"
safe_remove_file "rustdesk-*.pkg"
safe_remove_file "rustdesk-*.deb"
safe_remove_file "rustdesk-*.rpm"

echo ""
echo "Step 6: Cleaning log files..."
echo "-----------------------------------------"
safe_remove_file "build.log"
safe_remove_file "flutter_build.log"
safe_remove_file "flutter_test.log"
safe_remove_file "flutter_build_retry.log"
safe_remove_file "build_after_fix.log"
safe_remove_file "build_with_ui_fixes.log"
safe_remove_file "build-sciter-final.log"
safe_remove_file "/tmp/build_output.log"
safe_remove_file "/tmp/build_full.log"

echo ""
echo "Step 7: Cleaning generated bridge files..."
echo "-----------------------------------------"
safe_remove_file "flutter/lib/generated_bridge.dart"
safe_remove_file "flutter/macos/Runner/bridge_generated.h"

echo ""
echo "Step 8: Cleaning Windows-specific artifacts..."
echo "-----------------------------------------"
safe_remove "flutter/windows/flutter/ephemeral"

echo ""
echo "Step 9: Cleaning submodule build artifacts..."
echo "-----------------------------------------"
cd libs/hbb_common 2>/dev/null && cargo clean 2>/dev/null && cd ../.. || echo "hbb_common clean skipped"
cd libs/scrap 2>/dev/null && cargo clean 2>/dev/null && cd ../.. || echo "scrap clean skipped"
cd libs/enigo 2>/dev/null && cargo clean 2>/dev/null && cd ../.. || echo "enigo clean skipped"
cd libs/clipboard 2>/dev/null && cargo clean 2>/dev/null && cd ../.. || echo "clipboard clean skipped"
cd libs/virtual_display 2>/dev/null && cargo clean 2>/dev/null && cd ../.. || echo "virtual_display clean skipped"
cd libs/portable 2>/dev/null && cargo clean 2>/dev/null && cd ../.. || echo "portable clean skipped"

echo ""
echo "========================================="
echo "Clean complete!"
echo "========================================="
echo ""
echo "To rebuild from scratch, run:"
echo "  export VCPKG_ROOT=/o/rustdesk-build/vcpkg"
echo "  python3 build.py --flutter"
echo ""
echo "Note: pubspec.lock was preserved (needed for build)"
echo ""
