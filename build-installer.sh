#!/bin/bash
set -e

echo "========================================="
echo "RustDesk MouseMux v2.1 Build Script"
echo "========================================="
echo ""

echo "[1/5] Setting environment..."
cd /o/rustdesk-build/rustdesk
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
echo "VCPKG_ROOT=$VCPKG_ROOT"
echo ""

echo "[2/5] Building rustdesk.exe (this takes ~15 minutes)..."
cargo build --release --bin rustdesk
echo "✓ Build complete!"
echo ""

echo "[3/5] Copying executable to resources..."
cp target/release/rustdesk.exe resources/RustDesk.exe
echo "✓ Copied to resources/RustDesk.exe"
echo ""

echo "[4/5] Generating portable installer metadata..."
cd libs/portable
"/c/Program Files/Python313/python" generate.py -f ../../resources -o ../../target/release -e ../../resources/RustDesk.exe
echo "✓ Metadata generated!"
echo ""

echo "[5/5] Building portable packer executable..."
cd /o/rustdesk-build/rustdesk/libs/portable
cargo build --release
cp target/release/rustdesk-portable-packer.exe ../../target/release/
echo "✓ Portable packer built!"
echo ""

echo "[6/6] Creating final installer..."
cd /o/rustdesk-build/rustdesk
cp target/release/rustdesk-portable-packer.exe rustdesk-1.4.2-mousemux-v2.1-x86_64.exe
echo "✓ Final installer created!"
echo ""

echo "========================================="
echo "Build Complete!"
echo "========================================="
echo "Installer: rustdesk-1.4.2-mousemux-v2.1-x86_64.exe"
echo ""
echo "MD5 checksum:"
md5sum rustdesk-1.4.2-mousemux-v2.1-x86_64.exe
echo ""
echo "File size:"
ls -lh rustdesk-1.4.2-mousemux-v2.1-x86_64.exe
echo ""
