#!/bin/bash
# Wrapper to call Python via msys2_shell for build.py
cd /c/msys64
./msys2_shell.cmd -defterm -no-start -mingw64 -c "cd /c/RustDesk-build/rustdesk && python \"\$@\""
