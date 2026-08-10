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
# THE FFMPEG VERSION MATTERS - THIS IS THE PART THAT BITES
#
# hwcodec 0.7.1 (pinned by RustDesk 1.4.3 AND 1.4.9) compiles only against
# FFmpeg 7.x. It uses two APIs that FFmpeg 8.0 removed:
#     util.cpp:59,61         FF_PROFILE_H264_HIGH / FF_PROFILE_HEVC_MAIN
#     ffmpeg_ram_decode:218  AVFrame::key_frame
#
# Verified 2026-08-10 against the FFmpeg release tags:
#     n7.1.1 -> both present      n8.0 -> both gone
#
# `vcpkg.json` pins baseline 120deac3062162151622ca4860575a33844ba10b, which is
# FFmpeg 7.1.1 - i.e. the project already declares the correct version. The
# failure mode is a vcpkg tree that has drifted AHEAD of the baseline: install
# FFmpeg 8.x and hwcodec stops compiling, with C++ errors that look like a hwcodec
# bug but are not. Upgrading RustDesk does NOT help; 1.4.9's newer hwcodec still
# uses FF_PROFILE_* unguarded.
#
# This script therefore pins the vcpkg ports tree to the manifest baseline before
# installing. See MOUSEMUX_FIX_TRACKER.md, Finding 20.

set -uo pipefail

VCPKG_ROOT="${VCPKG_ROOT:-O:/devtools/vcpkg}"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="$(grep -o '"baseline": *"[0-9a-f]*"' "$REPO_ROOT/vcpkg.json" | grep -o '[0-9a-f]\{40\}')"

echo "=== 1/4  pin vcpkg ports tree to the manifest baseline ==="
echo "    baseline from vcpkg.json: $BASELINE"
PREV="$(git -C "$VCPKG_ROOT" rev-parse HEAD)"
echo "    current vcpkg HEAD:       $PREV   (restore with: git -C '$VCPKG_ROOT' checkout $PREV)"
if [ "$PREV" != "$BASELINE" ]; then
    if ! git -C "$VCPKG_ROOT" diff --quiet; then
        echo "!!! vcpkg tree has local modifications - refusing to switch. Resolve first."
        exit 1
    fi
    git -C "$VCPKG_ROOT" checkout -q "$BASELINE" || { echo "!!! checkout failed"; exit 1; }
    echo "    switched to baseline"
else
    echo "    already at baseline"
fi

echo "=== 2/4  vcpkg: ffmpeg (7.1.1) + mfx-dispatch ==="
# --classic is REQUIRED: without it vcpkg finds a vcpkg.json, switches to manifest
# mode, and refuses named packages. Manifest mode would also install into a
# project-local vcpkg_installed/ rather than the global tree that build.rs reads.
#
# The feature list is NOT optional. A bare `vcpkg install ffmpeg` builds the
# default feature set, which omits the hardware encoders - you get a build that
# links and runs but has no hardware acceleration at all.
"$VCPKG_ROOT/vcpkg.exe" install "ffmpeg[core,amf,nvcodec,qsv]:x64-windows-static" --classic --recurse \
    || { echo "!!! ffmpeg install failed"; exit 1; }
if [ -f "$VCPKG_ROOT/installed/x64-windows-static/lib/libmfx.lib" ]; then
    echo "    mfx-dispatch already installed"
else
    "$VCPKG_ROOT/vcpkg.exe" install mfx-dispatch:x64-windows-static --classic \
        || { echo "!!! vcpkg install failed"; exit 1; }
fi

echo "=== 3/4  patch hwcodec build.rs ==="
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

echo "=== 4/4  purge hwcodec build cache ==="
# Without this cargo silently relinks the OLD .rlib, built before the patch, and the
# fix appears to have done nothing.
REPO="$REPO_ROOT"
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
echo "  grep -c 'avcodec' target/release/librustdesk.dll   # expect ~200"
echo "  grep -c 'nvenc'   target/release/librustdesk.dll   # NVIDIA"
echo "  grep -c 'qsv'     target/release/librustdesk.dll   # Intel"
echo "  grep -c 'amf'     target/release/librustdesk.dll   # AMD"
echo
echo "A hwcodec-enabled librustdesk.dll is roughly 45 MB; without it, roughly 28 MB." 
