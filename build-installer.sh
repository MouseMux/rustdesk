#!/bin/bash
set -e

LOGFILE="/o/rustdesk-build/rustdesk/build.log"
echo "=========================================" | tee "$LOGFILE"
echo "RustDesk MouseMux v2.1 Build Script" | tee -a "$LOGFILE"
echo "=========================================" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"
echo "Log file: $LOGFILE" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

echo "[1/7] Setting environment..." | tee -a "$LOGFILE"
cd /o/rustdesk-build/rustdesk
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
echo "VCPKG_ROOT=$VCPKG_ROOT" | tee -a "$LOGFILE"
echo "Current directory: $(pwd)" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

echo "[2/7] Cleaning previous build artifacts..." | tee -a "$LOGFILE"
echo "Skipping cargo clean for faster incremental builds" | tee -a "$LOGFILE"
# cargo clean 2>&1 | tee -a "$LOGFILE"  # Commented out - only clean manually when needed
rm -f resources/* target/release/data.bin target/release/app_metadata.toml 2>&1 | tee -a "$LOGFILE"
echo "✓ Clean complete!" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

echo "[3/7] Building rustdesk.exe (this takes ~15 minutes)..." | tee -a "$LOGFILE"
echo "Build command: cargo build --release --bin rustdesk" | tee -a "$LOGFILE"
cargo build --release --bin rustdesk 2>&1 | tee -a "$LOGFILE"
echo "✓ Build complete!" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

# CHECK: Verify rustdesk.exe was created
echo "[CHECK] Verifying rustdesk.exe was built..." | tee -a "$LOGFILE"
if [ -f "target/release/rustdesk.exe" ]; then
    echo "✓ Found: target/release/rustdesk.exe" | tee -a "$LOGFILE"
    ls -lh target/release/rustdesk.exe | tee -a "$LOGFILE"
else
    echo "✗ ERROR: target/release/rustdesk.exe NOT FOUND!" | tee -a "$LOGFILE"
    echo "Build failed to create rustdesk.exe" | tee -a "$LOGFILE"
    exit 1
fi
echo "" | tee -a "$LOGFILE"

echo "[4/7] Preparing resources directory..." | tee -a "$LOGFILE"
mkdir -p resources
cp target/release/rustdesk.exe resources/ 2>&1 | tee -a "$LOGFILE"
cp sciter.dll resources/ 2>&1 | tee -a "$LOGFILE"
echo "✓ Resources prepared!" | tee -a "$LOGFILE"

# CHECK: Verify resources were copied
echo "[CHECK] Verifying resources..." | tee -a "$LOGFILE"
ls -lh resources/ | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

echo "[5/7] Generating portable installer metadata..." | tee -a "$LOGFILE"
cd libs/portable
"/c/Program Files/Python313/python" generate.py -f ../../resources -o ../../target/release -e ../../resources/rustdesk.exe 2>&1 | tee -a "$LOGFILE"
echo "✓ Metadata generated!" | tee -a "$LOGFILE"

# CHECK: Verify metadata files were created
echo "[CHECK] Verifying metadata files..." | tee -a "$LOGFILE"
if [ -f "../../target/release/data.bin" ]; then
    echo "✓ Found: data.bin" | tee -a "$LOGFILE"
    ls -lh ../../target/release/data.bin | tee -a "$LOGFILE"
else
    echo "✗ ERROR: data.bin NOT FOUND!" | tee -a "$LOGFILE"
    exit 1
fi
if [ -f "../../target/release/app_metadata.toml" ]; then
    echo "✓ Found: app_metadata.toml" | tee -a "$LOGFILE"
    cat ../../target/release/app_metadata.toml | tee -a "$LOGFILE"
else
    echo "✗ ERROR: app_metadata.toml NOT FOUND!" | tee -a "$LOGFILE"
    exit 1
fi
echo "" | tee -a "$LOGFILE"

echo "[6/7] Building portable packer executable..." | tee -a "$LOGFILE"
echo "Incremental rebuild to embed new data.bin" | tee -a "$LOGFILE"
cd /o/rustdesk-build/rustdesk/libs/portable
# cargo clean 2>&1 | tee -a "$LOGFILE"  # Commented out - only clean manually when needed
cargo build --release 2>&1 | tee -a "$LOGFILE"
echo "✓ Portable packer built with fresh data!" | tee -a "$LOGFILE"

# CHECK: Verify packer was built
echo "[CHECK] Verifying portable packer..." | tee -a "$LOGFILE"
if [ -f "../../target/release/rustdesk-portable-packer.exe" ]; then
    echo "✓ Found: rustdesk-portable-packer.exe" | tee -a "$LOGFILE"
    ls -lh ../../target/release/rustdesk-portable-packer.exe | tee -a "$LOGFILE"
else
    echo "✗ ERROR: rustdesk-portable-packer.exe NOT FOUND!" | tee -a "$LOGFILE"
    exit 1
fi
echo "" | tee -a "$LOGFILE"

echo "[7/7] Creating final installer..." | tee -a "$LOGFILE"
cd /o/rustdesk-build/rustdesk
cp target/release/rustdesk-portable-packer.exe rustdesk-1.4.2-mousemux-v2.1-x86_64.exe 2>&1 | tee -a "$LOGFILE"
echo "✓ Final installer created!" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

echo "=========================================" | tee -a "$LOGFILE"
echo "Build Complete!" | tee -a "$LOGFILE"
echo "=========================================" | tee -a "$LOGFILE"
echo "Installer: rustdesk-1.4.2-mousemux-v2.1-x86_64.exe" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"
echo "MD5 checksum:" | tee -a "$LOGFILE"
md5sum rustdesk-1.4.2-mousemux-v2.1-x86_64.exe | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"
echo "File size:" | tee -a "$LOGFILE"
ls -lh rustdesk-1.4.2-mousemux-v2.1-x86_64.exe | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"

echo "IMPORTANT NOTES:" | tee -a "$LOGFILE"
echo "- Window title will show: 'RustDesk (MouseMux compliant edition)'" | tee -a "$LOGFILE"
echo "- Install path will be: C:\\Program Files\\RustDesk\\" | tee -a "$LOGFILE"
echo "- APP_NAME is set to 'RustDesk' (not the full branding name)" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"
echo "Full log saved to: $LOGFILE" | tee -a "$LOGFILE"
