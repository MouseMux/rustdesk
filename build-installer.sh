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

echo "[5/7] Generating portable installer metadata and building packer..." | tee -a "$LOGFILE"
echo "CRITICAL: generate.py creates data.bin AND builds the packer in one step" | tee -a "$LOGFILE"
echo "The packer embeds data.bin at compile time via include_bytes!()" | tee -a "$LOGFILE"
cd libs/portable
# IMPORTANT: Output directory (-o) must be "." (libs/portable) so data.bin is created
# in the same directory where the packer will be built, allowing include_bytes!() to find it
"/c/Program Files/Python313/python" generate.py -f ../../resources -o . -e ../../resources/rustdesk.exe 2>&1 | tee -a "$LOGFILE"
echo "✓ Metadata generated and packer built!" | tee -a "$LOGFILE"

# CHECK: Verify data.bin was created in libs/portable (needed for include_bytes!)
echo "[CHECK] Verifying metadata files in libs/portable..." | tee -a "$LOGFILE"
if [ -f "data.bin" ]; then
    echo "✓ Found: libs/portable/data.bin" | tee -a "$LOGFILE"
    ls -lh data.bin | tee -a "$LOGFILE"
else
    echo "✗ ERROR: libs/portable/data.bin NOT FOUND!" | tee -a "$LOGFILE"
    echo "Packer cannot embed data.bin without this file!" | tee -a "$LOGFILE"
    exit 1
fi
if [ -f "app_metadata.toml" ]; then
    echo "✓ Found: libs/portable/app_metadata.toml" | tee -a "$LOGFILE"
    cat app_metadata.toml | tee -a "$LOGFILE"
else
    echo "✗ ERROR: libs/portable/app_metadata.toml NOT FOUND!" | tee -a "$LOGFILE"
    exit 1
fi
echo "" | tee -a "$LOGFILE"

echo "[6/7] Verifying portable packer was built..." | tee -a "$LOGFILE"
echo "Note: generate.py already built the packer (see above)" | tee -a "$LOGFILE"

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
echo "- Window title: 'RustDesk' (uses get_app_name())" | tee -a "$LOGFILE"
echo "- Install path: C:\\Program Files\\RustDesk\\" | tee -a "$LOGFILE"
echo "- APP_NAME: 'RustDesk' (set in libs/hbb_common/src/config.rs)" | tee -a "$LOGFILE"
echo "- MouseMux V2.1: Integrated with per-connection ID assignment" | tee -a "$LOGFILE"
echo "" | tee -a "$LOGFILE"
echo "Full log saved to: $LOGFILE" | tee -a "$LOGFILE"
