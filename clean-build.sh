#!/bin/bash
# MouseMux RustDesk - Clean Build Artifacts Script

echo "========================================"
echo " MouseMux RustDesk - Cleaning Build"
echo "========================================"
echo ""

echo "[1/5] Removing Rust build artifacts..."
cargo clean
echo "✓ Rust artifacts cleaned"
echo ""

echo "[2/5] Removing Flutter build artifacts..."
cd flutter
flutter clean
cd ..
echo "✓ Flutter artifacts cleaned"
echo ""

echo "[3/5] Removing generated installers..."
rm -f rustdesk-*-install.exe
rm -f rustdesk-*.msi
rm -f *.deb
rm -f *.rpm
echo "✓ Installers removed"
echo ""

echo "[4/5] Removing build log files..."
rm -f build*.log
rm -f flutter*.log
rm -f bridge_generation.log
echo "✓ Build logs removed"
echo ""

echo "[5/5] Removing temporary files..."
find . -name "*.tmp" -delete 2>/dev/null
find . -name "*.bak" -delete 2>/dev/null
echo "✓ Temporary files removed"
echo ""

echo "========================================"
echo " Clean Complete!"
echo "========================================"
echo ""
echo "Project is now clean and ready for a fresh build."
echo ""
