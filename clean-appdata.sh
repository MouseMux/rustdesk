#!/bin/bash

# Clean MouseMux RustDesk AppData directories
# This script removes all RustDesk MouseMux Edition data for testing

echo "Cleaning MouseMux RustDesk AppData directories..."
echo ""

# Convert Windows paths to Unix-style for Git Bash
LOCALAPPDATA_PATH=$(echo "$LOCALAPPDATA" | sed 's/\\/\//g' | sed 's/://')
APPDATA_PATH=$(echo "$APPDATA" | sed 's/\\/\//g' | sed 's/://')

# New directory name (after fix)
LOCAL_DIR="/$LOCALAPPDATA_PATH/rustdesk-mousemux-edition"
ROAMING_DIR="/$APPDATA_PATH/rustdesk-mousemux-edition"

# Old directory name (before fix - with spaces and capitals)
OLD_ROAMING_DIR="/$APPDATA_PATH/RustDesk MouseMux Edition"

# Function to safely remove directory
remove_dir() {
    local dir="$1"
    if [ -d "$dir" ]; then
        echo "Removing: $dir"
        rm -rf "$dir"
        if [ $? -eq 0 ]; then
            echo "  ✓ Removed successfully"
        else
            echo "  ✗ Failed to remove (may need admin privileges)"
        fi
    else
        echo "Not found: $dir (skipping)"
    fi
    echo ""
}

# Remove directories
echo "1. Local AppData (portable installer extraction):"
remove_dir "$LOCAL_DIR"

echo "2. Roaming AppData (config directory - new name):"
remove_dir "$ROAMING_DIR"

echo "3. Roaming AppData (config directory - old name with spaces):"
remove_dir "$OLD_ROAMING_DIR"

echo "=========================================="
echo "Cleanup complete!"
echo ""
echo "This removed:"
echo "  - Portable installer files"
echo "  - Configuration files"
echo "  - Log files"
echo "  - Connection history"
echo ""
echo "Next run will start fresh with new directory names."
