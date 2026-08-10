#!/usr/bin/env bash
#
# build-release.sh - full release build + package for RustDesk MouseMux Edition.
#
#   tools/build-release.sh [--no-hwcodec] [--debug-logging] [--skip-bridge]
#
# Produces:
#   rustdesk-development/final-builds/rustdesk-mousemux-<version>-<tags>-<date>/
#       Release/      runnable build
#       Release.zip
#       README.txt    what is in it, and what was verified
#
# WHY A SCRIPT
# ------------
# Every step below has bitten this project at least once. In particular:
#   - build.bat passes --skip-cargo and does NOT rebuild librustdesk.dll, so a
#     "successful build" can silently ship a months-old binary (this happened:
#     the Feb 2026 build shipped an APP_NAME regression nobody could see).
#   - the .bat wrappers end with `echo`, so %ERRORLEVEL% is 0 even when cargo
#     exited 101. Never trust the exit status of a .bat here; grep the log.
#   - hwcodec, the bridge and the printer driver are all prerequisites that live
#     OUTSIDE this repo and go stale invisibly.
# The verification block at the end exists because "it compiled" has repeatedly
# not meant "it contains what we think".

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO" || exit 1

HWCODEC=1
DEBUG_LOGGING=0
SKIP_BRIDGE=0
for arg in "$@"; do
    case "$arg" in
        --no-hwcodec)    HWCODEC=0 ;;
        --debug-logging) DEBUG_LOGGING=1 ;;
        --skip-bridge)   SKIP_BRIDGE=1 ;;
        *) echo "unknown option: $arg"; exit 1 ;;
    esac
done

FEATURES="flutter"
TAGS=""
[ "$HWCODEC" = "1" ]       && FEATURES="$FEATURES,hwcodec" && TAGS="hwcodec"  || TAGS="nohwcodec"
[ "$DEBUG_LOGGING" = "1" ] && FEATURES="$FEATURES,mousemux-debug" && TAGS="$TAGS-debug"

VERSION="$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)"
STAMP="$(date +%Y-%m-%d_%H-%M-%S)"
OUT="$REPO/../rustdesk-development/final-builds/rustdesk-mousemux-$VERSION-$TAGS-$STAMP"
REL="$REPO/flutter/build/windows/x64/runner/Release"
LOGDIR="$(mktemp -d)"

echo "=============================================================="
echo " version   : $VERSION"
echo " features  : $FEATURES"
echo " output    : $OUT"
echo "=============================================================="

die() { echo; echo "!!! FAILED: $*"; echo "!!! logs in $LOGDIR"; exit 1; }

# --- 1. prerequisites outside this repo ------------------------------------
if [ "$HWCODEC" = "1" ]; then
    echo "=== [1/6] hwcodec prerequisites"
    bash "$REPO/tools/setup-hwcodec.sh" > "$LOGDIR/hwcodec.log" 2>&1 \
        || { tail -20 "$LOGDIR/hwcodec.log"; die "setup-hwcodec.sh"; }
    echo "    ok (vcpkg baseline pinned, hwcodec patched, cache purged)"
else
    echo "=== [1/6] hwcodec disabled by flag"
fi

# --- 2. generated bridge ----------------------------------------------------
if [ "$SKIP_BRIDGE" = "0" ]; then
    echo "=== [2/6] flutter_rust_bridge"
    bash "$REPO/tools/gen-bridge.sh" > "$LOGDIR/bridge.log" 2>&1 \
        || { tail -20 "$LOGDIR/bridge.log"; die "gen-bridge.sh"; }
    echo "    ok"
else
    echo "=== [2/6] bridge regeneration skipped by flag"
fi

# --- 3. cargo ---------------------------------------------------------------
# NOT build.bat: that passes --skip-cargo and would leave librustdesk.dll stale.
echo "=== [3/6] cargo build --features $FEATURES --lib --release"
cat > "$LOGDIR/cargo.bat" <<EOF
@echo off
call "$(cygpath -w "$REPO" 2>/dev/null || echo "$REPO")\\tools\\env.bat"
cd /d "$(cygpath -w "$REPO" 2>/dev/null || echo "$REPO")"
cargo build --features $FEATURES --lib --release
echo === cargo exit code: %ERRORLEVEL% ===
EOF
cmd.exe //c "$(cygpath -w "$LOGDIR/cargo.bat" 2>/dev/null || echo "$LOGDIR/cargo.bat")" > "$LOGDIR/cargo.log" 2>&1
# The .bat always exits 0 (it ends with echo) - read the real code from the log.
CARGO_RC="$(grep -o 'cargo exit code: [0-9]*' "$LOGDIR/cargo.log" | tail -1 | grep -o '[0-9]*$')"
[ "$CARGO_RC" = "0" ] || { grep -n '^error' "$LOGDIR/cargo.log" | head -20; die "cargo exited $CARGO_RC"; }
echo "    ok"

# --- 4. flutter -------------------------------------------------------------
echo "=== [4/6] flutter build windows --release"
cat > "$LOGDIR/flutter.bat" <<EOF
@echo off
call "$(cygpath -w "$REPO" 2>/dev/null || echo "$REPO")\\tools\\env.bat"
cd /d "$(cygpath -w "$REPO" 2>/dev/null || echo "$REPO")\\flutter"
flutter build windows --release
echo === flutter exit code: %ERRORLEVEL% ===
EOF
cmd.exe //c "$(cygpath -w "$LOGDIR/flutter.bat" 2>/dev/null || echo "$LOGDIR/flutter.bat")" > "$LOGDIR/flutter.log" 2>&1
grep -q "Built build" "$LOGDIR/flutter.log" || { tail -25 "$LOGDIR/flutter.log"; die "flutter build"; }
echo "    ok"

# --- 5. assemble the Release folder ----------------------------------------
echo "=== [5/6] assembling Release"
cp -f "$REPO/target/release/deps/dylib_virtual_display.dll" "$REL/" \
    || die "dylib_virtual_display.dll missing - was the cargo build complete?"
echo "    dylib_virtual_display.dll"
bash "$REPO/tools/fetch-printer-driver.sh" "$REL" || die "printer driver"

# --- 6. verify, then package ------------------------------------------------
# Verify the BINARY, not the source. "It compiled" has repeatedly not meant "it
# contains what we think it contains".
echo "=== [6/6] verifying the built binary"
DLL="$REL/librustdesk.dll"
[ -f "$DLL" ] || die "librustdesk.dll not in Release"

fail=0
check() { # name pattern expectation(gt0|eq0)
    # grep -c already prints 0 when there are no matches, but EXITS 1 - so a
    # `|| echo 0` appends a second line and the numeric test below then errors
    # out and silently falls through to "ok". Take the first line and default.
    local c
    c="$(grep -c "$2" "$DLL" 2>/dev/null | head -1)"
    c="${c//[!0-9]/}"
    c="${c:-0}"
    if [ "$3" = "gt0" ] && [ "$c" -eq 0 ]; then
        echo "    MISSING  $1"; fail=1
    elif [ "$3" = "eq0" ] && [ "$c" -ne 0 ]; then
        echo "    PRESENT (should not be)  $1 ($c)"; fail=1
    else
        echo "    ok  $1 ($c)"
    fi
}
check "version string $VERSION"       "$VERSION"                          gt0
check "branding rustdesk-mousemux"    "rustdesk-mousemux-edition"         gt0
check "versioned rustdesk window"     "mousemux-v3.rustdesk.window.query" gt0
check "mousemux lookup class"         "mousemux-v3.main.window.query"     gt0
if [ "$HWCODEC" = "1" ]; then
    check "hwcodec: avcodec" "avcodec" gt0
    check "hwcodec: nvenc"   "nvenc"   gt0
    check "hwcodec: qsv"     "qsv"     gt0
    check "hwcodec: amf"     "amf"     gt0
else
    check "hwcodec absent"   "avcodec" eq0
fi
if [ "$DEBUG_LOGGING" = "0" ]; then
    check "per-event logging compiled out" "map_keyboard_mode() called" eq0
fi
[ "$fail" = "0" ] || die "binary verification"

SIZE_MB=$(( $(stat -c %s "$DLL") / 1048576 ))
echo "    librustdesk.dll ${SIZE_MB}MB"

echo "=== packaging"
mkdir -p "$OUT/Release"
cp -r "$REL/." "$OUT/Release/" || die "copy Release"
( cd "$OUT" && "O:/devtools/shell/msys64/usr/bin/zip.exe" -qr Release.zip Release ) \
    || powershell.exe -NoProfile -Command "Compress-Archive -Path '$(cygpath -w "$OUT/Release")' -DestinationPath '$(cygpath -w "$OUT/Release.zip")' -Force" \
    || die "zip"

cmp -s "$OUT/Release/librustdesk.dll" "$DLL" || die "archived DLL differs from build output"

cat > "$OUT/README.txt" <<EOF
RustDesk MouseMux Edition $VERSION
$(printf '=%.0s' $(seq 1 $((34 + ${#VERSION}))))

Build Date : $STAMP
Features   : $FEATURES
Branch     : $(git -C "$REPO" rev-parse --abbrev-ref HEAD 2>/dev/null)
Commit     : $(git -C "$REPO" rev-parse --short HEAD 2>/dev/null)
hbb_common : $(git -C "$REPO/libs/hbb_common" rev-parse --short HEAD 2>/dev/null)

Built by tools/build-release.sh, which also verified the binary below.

VERIFIED IN librustdesk.dll (${SIZE_MB}MB)
$(grep -c "$VERSION" "$DLL" >/dev/null && echo "  version string, branding, versioned window names")
$([ "$HWCODEC" = "1" ] && echo "  hardware codec linked (avcodec / nvenc / qsv / amf)" || echo "  NO hardware codec (--no-hwcodec)")
$([ "$DEBUG_LOGGING" = "0" ] && echo "  per-event MouseMux logging compiled out" || echo "  per-event MouseMux logging ENABLED (debug build)")

NOT VERIFIED
  Nothing here is runtime-tested. Compiling and containing the right strings is
  not the same as behaving correctly. The test that matters most is two users
  connected at once, both typing, each getting their own cursor - MouseMux routes
  purely on the hardware ID RustDesk stamps and has no safety net of its own.

REBUILD THIS EXACTLY
  git clone --recursive https://github.com/MouseMux/rustdesk
  cd rustdesk && git checkout $(git -C "$REPO" rev-parse --short HEAD 2>/dev/null)
  git submodule update --init --recursive
  tools/build-release.sh$([ "$HWCODEC" = "0" ] && echo " --no-hwcodec")
EOF

echo
echo "=============================================================="
echo " DONE"
echo "   $OUT"
echo "   Release.zip $(stat -c %s "$OUT/Release.zip" 2>/dev/null | awk '{printf "%.1f MB", $1/1048576}')"
echo "=============================================================="
