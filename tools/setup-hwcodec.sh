#!/usr/bin/env bash
#
# setup-hwcodec.sh - make a hwcodec-enabled build possible on this machine.
#
# WHY THIS EXISTS
# ---------------
# `hwcodec` is a non-default cargo feature, and building with it fails to LINK
# unless the hwcodec crate's own build.rs is patched. That file lives in the cargo
# git checkout, NOT in this repo - so the fix is wiped by any cargo cache clear and
# is absent on every fresh machine. See MOUSEMUX_FIX_TRACKER.md, Finding 20.
#
# On 2026-08-09 this bit us: a build was produced with `--features flutter` only and
# shipped with no hardware codec at all, which would have been a silent encoding
# regression against the 2025-11-01 builds.
#
# Run this before:  cargo build --features flutter,hwcodec --lib --release
#
# Idempotent - safe to run repeatedly.
#
# !!! NOT SUFFICIENT ON ITS OWN AS OF 2026-08-09 !!!
# This script fixes the LINKAGE problem documented in BUGFIXES.md, and that part
# works. But hwcodec 0.7.1 (the revision RustDesk 1.4.3 pins) will still fail to
# COMPILE against FFmpeg 7.0+ because of API removals:
#     util.cpp:59,61         FF_PROFILE_H264_HIGH / FF_PROFILE_HEVC_MAIN
#                            -> renamed to AV_PROFILE_*
#     ffmpeg_ram_decode:218  AVFrame::key_frame  -> replaced by AV_FRAME_FLAG_KEY
# This machine's vcpkg has FFmpeg 8.0.1 (libavutil 60.8), so hwcodec cannot be
# built here until either RustDesk is upgraded to a version pinning a newer
# hwcodec, or FFmpeg is downgraded to 6.x, or the hwcodec C++ is patched.
# Tracked as Finding 20.

set -uo pipefail

VCPKG_ROOT="${VCPKG_ROOT:-O:/devtools/vcpkg}"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"

echo "=== 1/3  vcpkg: mfx-dispatch (Intel Media SDK dispatcher) ==="
# --classic is REQUIRED: without it vcpkg finds a vcpkg.json, switches to manifest
# mode, and refuses named packages. Manifest mode would also install into a
# project-local vcpkg_installed/ rather than the global tree that build.rs reads.
if [ -f "$VCPKG_ROOT/installed/x64-windows-static/lib/libmfx.lib" ]; then
    echo "    already installed, skipping"
else
    "$VCPKG_ROOT/vcpkg.exe" install mfx-dispatch:x64-windows-static --classic \
        || { echo "!!! vcpkg install failed"; exit 1; }
fi

echo "=== 2/3  patch hwcodec build.rs ==="
# Two link fixes: FFmpeg audio resampling, and Windows Media Foundation.
patched=0
for BUILD_RS in "$CARGO_HOME"/git/checkouts/hwcodec-*/*/build.rs; do
    [ -f "$BUILD_RS" ] || continue
    echo "    $BUILD_RS"

    if grep -q '"avcodec", "avutil", "avformat", "swresample"' "$BUILD_RS"; then
        echo "      swresample: already patched"
    else
        sed -i 's/vec!\["avcodec", "avutil", "avformat"\]/vec!["avcodec", "avutil", "avformat", "swresample"]/' "$BUILD_RS"
        echo "      swresample: patched"
    fi

    if grep -q '"mfuuid", "mfplat", "strmiids"' "$BUILD_RS"; then
        echo "      media foundation: already patched"
    else
        sed -i 's/\["User32", "bcrypt", "ole32", "advapi32"\]/["User32", "bcrypt", "ole32", "advapi32", "mfuuid", "mfplat", "strmiids"]/' "$BUILD_RS"
        echo "      media foundation: patched"
    fi
    patched=1
done
[ "$patched" = "1" ] || echo "    WARNING: no hwcodec checkout found - run a build first, then re-run this"

echo "=== 3/3  purge hwcodec build cache ==="
# Without this cargo silently relinks the OLD .rlib, built before the patch, and the
# fix appears to have done nothing.
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rm -rf "$REPO"/target/release/.fingerprint/hwcodec-* \
       "$REPO"/target/release/build/hwcodec-* \
       "$REPO"/target/release/deps/*hwcodec* \
       "$REPO"/target/release/*hwcodec* 2>/dev/null
echo "    purged"

echo
echo "Done. Now build with:"
echo "  cargo build --features flutter,hwcodec --lib --release"
echo
echo "Verify afterwards - the DLL must reference the codec libs:"
echo "  grep -c 'avcodec' target/release/librustdesk.dll   # expect > 0"
