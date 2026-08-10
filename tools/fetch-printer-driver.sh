#!/usr/bin/env bash
#
# fetch-printer-driver.sh - download the prebuilt printer driver files.
#
# WHY THIS EXISTS
# ---------------
# RustDesk 1.4.9 logs "printer_driver_adapter.dll not found" and remote printing
# silently does not work. The DLL is NOT built from source: upstream CI downloads
# it as a prebuilt binary from a GitHub release
# (.github/workflows/flutter-build.yml), so a local build has nothing to produce
# it and no reason to notice.
#
# Idempotent - skips the download if the files are already in place.
#
# Usage:  tools/fetch-printer-driver.sh <release-dir>

set -uo pipefail

REL="${1:-}"
if [ -z "$REL" ] || [ ! -d "$REL" ]; then
    echo "usage: $0 <release-dir>"
    exit 1
fi

BASE="https://github.com/rustdesk/hbb_common/releases/download/driver"
DRIVER_ZIP="rustdesk_printer_driver_v4-1.4.zip"
ADAPTER_ZIP="printer_driver_adapter.zip"

if [ -f "$REL/printer_driver_adapter.dll" ] && [ -d "$REL/drivers/RustDeskPrinterDriver" ]; then
    echo "    printer driver already present, skipping"
    exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK" || exit 1

echo "    downloading printer driver + adapter"
for f in "$DRIVER_ZIP" "$ADAPTER_ZIP" sha256sums; do
    if ! curl.exe -sSL -o "$f" "$BASE/$f"; then
        echo "!!! download failed: $f"
        exit 1
    fi
done

# Verify against upstream's own manifest before unpacking anything. These are
# binaries that end up shipped next to rustdesk.exe, so an unverified download is
# not acceptable.
verify() {
    local zip="$1"
    local want
    want="$(grep -F " *$zip" sha256sums | cut -d' ' -f1 | tr 'A-F' 'a-f')"
    local got
    got="$(sha256sum "$zip" | cut -d' ' -f1 | tr 'A-F' 'a-f')"
    if [ -z "$want" ]; then
        echo "!!! no checksum listed for $zip"; return 1
    fi
    if [ "$want" != "$got" ]; then
        echo "!!! CHECKSUM MISMATCH for $zip"
        echo "      expected $want"
        echo "      actual   $got"
        return 1
    fi
    echo "    checksum ok: $zip"
}

verify "$DRIVER_ZIP"  || exit 1
verify "$ADAPTER_ZIP" || exit 1

"O:/devtools/shell/msys64/usr/bin/unzip.exe" -qo "$ADAPTER_ZIP" || exit 1
"O:/devtools/shell/msys64/usr/bin/unzip.exe" -qo "$DRIVER_ZIP"  || exit 1

cp -f ./printer_driver_adapter.dll "$REL/" || exit 1
mkdir -p "$REL/drivers"
if [ -d "./rustdesk_printer_driver_v4-1.4" ]; then
    rm -rf "$REL/drivers/RustDeskPrinterDriver"
    cp -r "./rustdesk_printer_driver_v4-1.4" "$REL/drivers/RustDeskPrinterDriver" || exit 1
fi

echo "    printer driver installed into $REL"
