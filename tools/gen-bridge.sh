#!/usr/bin/env bash
#
# gen-bridge.sh - regenerate the flutter_rust_bridge glue.
#
# WHY THIS EXISTS
# ---------------
# src/bridge_generated.rs, src/bridge_generated.io.rs and
# flutter/lib/generated_bridge.dart are GENERATED and gitignored - by us and by
# upstream. They are not produced by `cargo build`, so after any change to
# src/flutter_ffi.rs (including an upstream merge) they silently go stale and the
# build fails with "cannot find function ..." against functions that no longer
# exist. That is exactly what the 1.4.9 merge hit.
#
# TWO THINGS THAT ARE EASY TO GET WRONG
#
# 1. --llvm-path is REQUIRED on this machine. ffigen does NOT read LIBCLANG_PATH.
#    Its built-in search list is Homebrew/Linux paths plus C:/Program Files/llvm
#    and C:/msys64/mingw64 - ours is at O:/devtools/LLVM64. Without it ffigen
#    fails, and the failure is doubly nasty: codegen still writes a Rust file, but
#    one containing placeholder stubs that reference an undefined Dart_Handle, AND
#    it leaves the Dart half of the bridge untouched and stale.
#
# 2. The codegen version is pinned by CI (.github/workflows/bridge.yml). Using a
#    different one produces subtly different output.
#
# Usage:  tools/gen-bridge.sh

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LLVM_PATH="${LLVM_PATH:-O:/devtools/LLVM64}"

# Version pinned by .github/workflows/bridge.yml (FLUTTER_RUST_BRIDGE_VERSION).
WANT_VERSION="$(grep -o 'FLUTTER_RUST_BRIDGE_VERSION: *"[0-9.]*"' "$REPO/.github/workflows/bridge.yml" 2>/dev/null \
                | grep -o '[0-9][0-9.]*' | head -1)"
WANT_VERSION="${WANT_VERSION:-1.80.1}"

CODEGEN="$(command -v flutter_rust_bridge_codegen 2>/dev/null || true)"
if [ -z "$CODEGEN" ]; then
    CODEGEN="$HOME/.cargo/bin/flutter_rust_bridge_codegen.exe"
    [ -f "$CODEGEN" ] || CODEGEN="C:/Users/$USER/.cargo/bin/flutter_rust_bridge_codegen.exe"
fi

if [ ! -f "$CODEGEN" ] && ! command -v flutter_rust_bridge_codegen >/dev/null 2>&1; then
    echo "!!! flutter_rust_bridge_codegen not installed."
    echo "!!! Install the version CI pins:"
    echo "      cargo install flutter_rust_bridge_codegen --version $WANT_VERSION --features uuid --locked"
    exit 1
fi

if [ ! -f "$LLVM_PATH/bin/libclang.dll" ]; then
    echo "!!! libclang.dll not found under $LLVM_PATH/bin"
    echo "!!! Set LLVM_PATH to your LLVM install; ffigen cannot find it on its own."
    exit 1
fi

echo "=== regenerating flutter_rust_bridge (codegen pinned at $WANT_VERSION)"
cd "$REPO" || exit 1

cmd.exe //c "cd /d $(cygpath -w "$REPO" 2>/dev/null || echo "$REPO") && call tools\\env.bat && flutter_rust_bridge_codegen --rust-input ./src/flutter_ffi.rs --dart-output ./flutter/lib/generated_bridge.dart --c-output ./flutter/macos/Runner/bridge_generated.h --llvm-path $(cygpath -w "$LLVM_PATH" 2>/dev/null || echo "$LLVM_PATH")"
rc=$?

if [ $rc -ne 0 ]; then
    echo "!!! codegen failed (exit $rc)"
    exit 1
fi

# Guard against the silent-failure mode described above.
if grep -q "Dart_Handle" "$REPO/src/bridge_generated.rs" 2>/dev/null; then
    echo "!!! bridge_generated.rs references Dart_Handle - ffigen did not run properly."
    echo "!!! This compiles to 'cannot find type Dart_Handle'. Check --llvm-path."
    exit 1
fi

echo "    bridge regenerated:"
for f in src/bridge_generated.rs src/bridge_generated.io.rs flutter/lib/generated_bridge.dart; do
    [ -f "$REPO/$f" ] && echo "      $f  ($(stat -c %y "$REPO/$f" 2>/dev/null | cut -d. -f1))"
done
