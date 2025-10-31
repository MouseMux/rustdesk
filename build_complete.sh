#!/bin/bash
#
# RustDesk MouseMux Edition - Complete Build Script
# Builds everything from start to finish with hwcodec support
#
# Usage:
#   ./build_complete.sh                    # Run all steps
#   ./build_complete.sh --from-step 4      # Start from step 4
#   ./build_complete.sh --only-step 6      # Run only step 6
#   ./build_complete.sh --list             # List all steps
#

set -e  # Exit on any error

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VCPKG_ROOT="/o/rustdesk-build/vcpkg"
FLUTTER_BIN="/o/rustdesk-build/flutter/bin"
HWCODEC_BUILD_RS="/c/Users/Developer/.cargo/git/checkouts/hwcodec-3f3da9ff8e484625/17c1dbb/build.rs"
FINAL_BUILDS_DIR="/o/rustdesk-development/final-builds"

# Export environment variables
export VCPKG_ROOT
export PATH="${FLUTTER_BIN}:${PATH}"

# Parse command line arguments
START_STEP=1
END_STEP=7
ONLY_STEP=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --from-step)
            START_STEP="$2"
            shift 2
            ;;
        --only-step)
            ONLY_STEP="$2"
            START_STEP="$2"
            END_STEP="$2"
            shift 2
            ;;
        --list)
            echo "Available build steps:"
            echo "  1. Install Hardware Codec Dependencies (vcpkg)"
            echo "  2. Fix hwcodec build.rs Linkage"
            echo "  3. Clean hwcodec Build Cache"
            echo "  4. Build Rust Library (hwcodec enabled)"
            echo "  5. Build Flutter App"
            echo "  6. Create Portable Installer"
            echo "  7. Create ZIP and Organize Final Builds"
            echo ""
            echo "Usage examples:"
            echo "  ./build_complete.sh                    # Run all steps"
            echo "  ./build_complete.sh --from-step 4      # Start from step 4"
            echo "  ./build_complete.sh --only-step 6      # Run only step 6"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --list to see available options"
            exit 1
            ;;
    esac
done

# ============================================================================
# Utility Functions
# ============================================================================

print_header() {
    echo ""
    echo "========================================"
    echo "$1"
    echo "========================================"
}

print_success() {
    echo "✓ $1"
}

print_error() {
    echo "✗ ERROR: $1" >&2
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        print_error "$1 not found in PATH"
        exit 1
    fi
}

should_run_step() {
    local step=$1
    if [ -n "$ONLY_STEP" ]; then
        [ "$step" -eq "$ONLY_STEP" ]
    else
        [ "$step" -ge "$START_STEP" ] && [ "$step" -le "$END_STEP" ]
    fi
}

# ============================================================================
# Pre-flight Checks (always run)
# ============================================================================

print_header "Pre-flight Checks"

check_command python
check_command cargo
check_command flutter

if [ ! -d "$VCPKG_ROOT" ]; then
    print_error "vcpkg not found at $VCPKG_ROOT"
    exit 1
fi

print_success "All prerequisites found"

# Get version and build name
VERSION=$(grep "^version = " Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
TIMESTAMP=$(date +%Y-%m-%d_%H-%M-%S)
BUILD_NAME="rustdesk-mousemux-${VERSION}-hwcodec-${TIMESTAMP}"

echo "Version: $VERSION"
echo "Build: $BUILD_NAME"

if [ -n "$ONLY_STEP" ]; then
    echo "Running only step: $ONLY_STEP"
elif [ "$START_STEP" -gt 1 ]; then
    echo "Starting from step: $START_STEP"
fi

# Initialize timing variables
RUST_BUILD_TIME=0
FLUTTER_BUILD_TIME=0

# ============================================================================
# Step 1: Install Hardware Codec Dependencies
# ============================================================================

if should_run_step 1; then
    print_header "Step 1/7: Installing Hardware Codec Dependencies"

    cd "$VCPKG_ROOT"

    echo "Installing FFmpeg and Intel MFX dispatcher..."
    ./vcpkg install ffmpeg:x64-windows-static mfx-dispatch:x64-windows-static

    if [ $? -ne 0 ]; then
        print_error "Dependency installation failed"
        exit 1
    fi

    print_success "Dependencies installed (FFmpeg, mfx-dispatch)"
    cd "$SCRIPT_DIR"
else
    echo "Skipping Step 1: Install Hardware Codec Dependencies"
fi

# ============================================================================
# Step 2: Fix hwcodec build.rs Linkage Issues
# ============================================================================

if should_run_step 2; then
    print_header "Step 2/7: Fixing hwcodec build.rs Linkage"

    if [ -f "$HWCODEC_BUILD_RS" ]; then
        echo "Creating backup at ${HWCODEC_BUILD_RS}.bak"
        cp -f "$HWCODEC_BUILD_RS" "${HWCODEC_BUILD_RS}.bak" 2>/dev/null || true

        echo "Applying library linkage fixes..."

        # Fix 1: Add swresample for FFmpeg audio resampling
        sed -i 's/let mut static_libs = vec!\["avcodec", "avutil", "avformat"\];/let mut static_libs = vec!["avcodec", "avutil", "avformat", "swresample"];/' "$HWCODEC_BUILD_RS"

        # Fix 2: Add Windows Media Foundation libraries
        sed -i 's/\["User32", "bcrypt", "ole32", "advapi32"\]\.to_vec()/["User32", "bcrypt", "ole32", "advapi32", "mfuuid", "mfplat", "strmiids"].to_vec()/' "$HWCODEC_BUILD_RS"

        print_success "hwcodec build.rs patched"
        echo "  - Added: swresample (FFmpeg audio resampling)"
        echo "  - Added: mfuuid, mfplat, strmiids (Windows Media Foundation)"
    else
        echo "Note: hwcodec build.rs not found yet (will be downloaded during build)"
    fi
else
    echo "Skipping Step 2: Fix hwcodec build.rs Linkage"
fi

# ============================================================================
# Step 3: Clean hwcodec Build Cache
# ============================================================================

if should_run_step 3; then
    print_header "Step 3/7: Cleaning hwcodec Build Cache"

    cd "$SCRIPT_DIR"

    echo "Removing cached hwcodec artifacts..."
    rm -rf target/release/.fingerprint/hwcodec-* \
           target/release/build/hwcodec-* \
           target/release/deps/*hwcodec* \
           target/release/*hwcodec* 2>/dev/null || true

    print_success "Build cache cleared"
else
    echo "Skipping Step 3: Clean hwcodec Build Cache"
fi

# ============================================================================
# Step 4: Build Rust Library with Hardware Codec
# ============================================================================

if should_run_step 4; then
    print_header "Step 4/7: Building Rust Library (hwcodec enabled)"

    cd "$SCRIPT_DIR"

    echo "Building Rust library with features: hwcodec, flutter"
    echo "This will take several minutes..."
    echo ""

    BUILD_START=$(date +%s)

    # Build only the Rust library (not Flutter yet)
    cargo build --release --features hwcodec,flutter 2>&1 | tee "build-${BUILD_NAME}-rust.log"

    BUILD_STATUS=$?
    BUILD_END=$(date +%s)
    RUST_BUILD_TIME=$((BUILD_END - BUILD_START))

    if [ $BUILD_STATUS -ne 0 ]; then
        print_error "Rust build failed after ${RUST_BUILD_TIME}s"
        echo "Check build-${BUILD_NAME}-rust.log for details"
        exit 1
    fi

    print_success "Rust library built in ${RUST_BUILD_TIME}s"
else
    echo "Skipping Step 4: Build Rust Library"
fi

# ============================================================================
# Step 5: Build Flutter App
# ============================================================================

if should_run_step 5; then
    print_header "Step 5/7: Building Flutter App"

    cd "$SCRIPT_DIR/flutter"

    echo "Building Flutter Windows app..."
    echo "PATH includes: $FLUTTER_BIN"
    echo ""

    FLUTTER_START=$(date +%s)

    flutter build windows --release 2>&1 | tee "../build-${BUILD_NAME}-flutter.log"

    FLUTTER_STATUS=$?
    FLUTTER_END=$(date +%s)
    FLUTTER_BUILD_TIME=$((FLUTTER_END - FLUTTER_START))

    if [ $FLUTTER_STATUS -ne 0 ]; then
        print_error "Flutter build failed after ${FLUTTER_BUILD_TIME}s"
        echo "Check build-${BUILD_NAME}-flutter.log for details"
        exit 1
    fi

    cd "$SCRIPT_DIR"

    print_success "Flutter app built in ${FLUTTER_BUILD_TIME}s"
else
    echo "Skipping Step 5: Build Flutter App"
fi

TOTAL_BUILD_TIME=$((RUST_BUILD_TIME + FLUTTER_BUILD_TIME))

# ============================================================================
# Step 6: Create Portable Installer
# ============================================================================

if should_run_step 6; then
    print_header "Step 6/7: Creating Portable Installer"

    cd "$SCRIPT_DIR"

    FLUTTER_BUILD_DIR="flutter/build/windows/x64/runner/Release"
    MAIN_EXE="${FLUTTER_BUILD_DIR}/rustdesk.exe"

    if [ ! -f "$MAIN_EXE" ]; then
        print_error "Main executable not found at $MAIN_EXE"
        exit 1
    fi

    print_success "Main executable found"

    FILE_SIZE=$(ls -lh "$MAIN_EXE" | awk '{print $5}')
    echo "  Size: $FILE_SIZE"

    cd libs/portable

    echo "Installing portable packer requirements..."
    pip3 install -r requirements.txt > /dev/null 2>&1

    echo "Generating portable installer..."
    python ./generate.py -f "../../${FLUTTER_BUILD_DIR}" -o . -e "../../${MAIN_EXE}"

    if [ $? -ne 0 ]; then
        print_error "Portable installer generation failed"
        exit 1
    fi

    cd "$SCRIPT_DIR"

    # Move the portable packer to root with correct name
    PORTABLE_INSTALLER="rustdesk-${VERSION}-install.exe"

    if [ -f "target/release/rustdesk-portable-packer.exe" ]; then
        if [ -f "$PORTABLE_INSTALLER" ]; then
            rm -f "$PORTABLE_INSTALLER"
        fi
        mv "target/release/rustdesk-portable-packer.exe" "$PORTABLE_INSTALLER"
        print_success "Portable installer created: $PORTABLE_INSTALLER"
    else
        print_error "Portable packer not found"
        exit 1
    fi

    INSTALLER_SIZE=$(ls -lh "$PORTABLE_INSTALLER" | awk '{print $5}')
    echo "  Size: $INSTALLER_SIZE"
else
    echo "Skipping Step 6: Create Portable Installer"
    # Still need these variables for step 7
    PORTABLE_INSTALLER="rustdesk-${VERSION}-install.exe"
    if [ -f "$PORTABLE_INSTALLER" ]; then
        INSTALLER_SIZE=$(ls -lh "$PORTABLE_INSTALLER" | awk '{print $5}')
    else
        INSTALLER_SIZE="unknown"
    fi
fi

# ============================================================================
# Step 7: Create ZIP and Organize Final Builds
# ============================================================================

if should_run_step 7; then
    print_header "Step 7/7: Creating ZIP Package and Final Organization"

    cd "$SCRIPT_DIR"

    # Create final builds directory structure
    mkdir -p "${FINAL_BUILDS_DIR}/${BUILD_NAME}"

    FLUTTER_BUILD_DIR="flutter/build/windows/x64/runner/Release"

    echo "Creating ZIP archive..."
    cd "${FLUTTER_BUILD_DIR}"

    # Use native zip command (much faster than Python for many files)
    ZIP_FILE="${BUILD_NAME}.zip"
    zip -r "$ZIP_FILE" . > /dev/null 2>&1

    if [ $? -ne 0 ]; then
        print_error "ZIP creation failed"
        exit 1
    fi

    ZIP_SIZE_HUMAN=$(ls -lh "$ZIP_FILE" | awk '{print $5}')
    echo "Created: $ZIP_FILE ($ZIP_SIZE_HUMAN)"

    mv "$ZIP_FILE" "${FINAL_BUILDS_DIR}/${BUILD_NAME}/"

    cd "$SCRIPT_DIR"

    ZIP_SIZE=$(ls -lh "${FINAL_BUILDS_DIR}/${BUILD_NAME}/${ZIP_FILE}" | awk '{print $5}')
    print_success "ZIP package created: ${ZIP_FILE} (${ZIP_SIZE})"

    # Copy portable installer
    if [ -f "$PORTABLE_INSTALLER" ]; then
        cp "$PORTABLE_INSTALLER" "${FINAL_BUILDS_DIR}/${BUILD_NAME}/"
        print_success "Copied: $PORTABLE_INSTALLER"
    else
        echo "Warning: Portable installer not found, skipping copy"
    fi

    # Copy build logs
    cp "build-${BUILD_NAME}-rust.log" "${FINAL_BUILDS_DIR}/${BUILD_NAME}/" 2>/dev/null || true
    cp "build-${BUILD_NAME}-flutter.log" "${FINAL_BUILDS_DIR}/${BUILD_NAME}/" 2>/dev/null || true
    print_success "Copied: build logs"

    # Create README
    cat > "${FINAL_BUILDS_DIR}/${BUILD_NAME}/README.txt" << EOF
RustDesk MouseMux Edition - Hardware Codec Build
================================================

Version: ${VERSION}
Build Date: ${TIMESTAMP}
Build Time: ${TOTAL_BUILD_TIME}s (Rust: ${RUST_BUILD_TIME}s, Flutter: ${FLUTTER_BUILD_TIME}s)

CRITICAL FIX:
- APP_NAME changed from "RustDesk MouseMux Edition" to "rustdesk-mousemux-edition"
- Removes spaces from Windows paths (C:\ProgramData\rustdesk-mousemux-edition\...)
- Fixes named pipes (\\.\pipe\rustdesk-mousemux-edition\...)
- This should resolve the "waiting for image" issue

Features:
- Hardware Codec (hwcodec) enabled
- H265 hardware encoding via NVIDIA NVENC
- Intel MFX support
- FFmpeg with swresample

Files in this package:
1. ${PORTABLE_INSTALLER} (${INSTALLER_SIZE})
   - Portable installer with all dependencies packed
   - Run this to install RustDesk

2. ${ZIP_FILE} (${ZIP_SIZE})
   - Portable ZIP package
   - Extract and run rustdesk.exe directly

3. Build logs (rust + flutter)

Hardware Codec Libraries:
- FFmpeg (avcodec, avutil, avformat, swresample)
- Intel MFX (libmfx dispatcher)
- Windows Media Foundation (mfuuid, mfplat, strmiids)

This build should resolve the "waiting for image" issue by:
1. Using H265 hardware encoding instead of VP9 software encoding
2. Fixing APP_NAME to remove spaces (Windows 11 path issue)
EOF

    print_success "Created: README.txt"

    echo ""
    echo "Final Builds Location:"
    echo "  ${FINAL_BUILDS_DIR}/${BUILD_NAME}/"
else
    echo "Skipping Step 7: Create ZIP and Organize Final Builds"
fi

# ============================================================================
# Build Summary
# ============================================================================

if should_run_step 7; then
    print_header "Build Summary"

    echo "Build Configuration:"
    echo "  Version: $VERSION"
    echo "  Features: hwcodec, flutter"
    echo "  Codec: H265 hardware encoding (NVENC)"
    if [ $TOTAL_BUILD_TIME -gt 0 ]; then
        echo "  Build Time: ${TOTAL_BUILD_TIME}s (Rust: ${RUST_BUILD_TIME}s + Flutter: ${FLUTTER_BUILD_TIME}s)"
    fi
    echo ""

    echo "Critical Fix Applied:"
    echo "  APP_NAME: rustdesk-mousemux-edition (no spaces)"
    echo "  Paths: C:\ProgramData\rustdesk-mousemux-edition\"
    echo "  Pipes: \\.\pipe\rustdesk-mousemux-edition\"
    echo ""

    echo "Build Artifacts:"
    echo "  1. Portable Installer: ${INSTALLER_SIZE}"
    echo "     ${FINAL_BUILDS_DIR}/${BUILD_NAME}/${PORTABLE_INSTALLER}"
    echo ""
    echo "  2. ZIP Package: ${ZIP_SIZE}"
    echo "     ${FINAL_BUILDS_DIR}/${BUILD_NAME}/${ZIP_FILE}"
    echo ""

    print_header "Build Completed Successfully!"

    echo ""
    echo "Next Steps:"
    echo "  1. Test the portable installer:"
    echo "     ${FINAL_BUILDS_DIR}/${BUILD_NAME}/${PORTABLE_INSTALLER}"
    echo ""
    echo "  2. Verify APP_NAME fix in logs (should show rustdesk-mousemux-edition)"
    echo ""
    echo "  3. Test connection - 'waiting for image' should be FIXED!"
    echo ""
fi
