@echo off
rem ===========================================================================
rem Canonical build environment for RustDesk MouseMux Edition.
rem
rem Call this FIRST from any build step:
rem     call tools\env.bat
rem
rem Every line here exists because getting it wrong cost real time on
rem 2026-08-09/10. Do not reorder.
rem ===========================================================================

rem MSVC toolchain. Must come BEFORE VCPKG_ROOT is set - see below.
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (
    echo [env] FATAL: vcvars64.bat failed
    exit /b 1
)

rem --------------------------------------------------------------------------
rem VCPKG_ROOT MUST be set AFTER vcvars64.
rem vcvars64 overwrites VCPKG_ROOT with Visual Studio's own bundled vcpkg, which
rem has no installed/ directory. Setting it first means scrap and magnum-opus
rem fail with "fatal error: 'vpx/vp8.h' file not found" - which looks like a
rem missing dependency but is actually the wrong vcpkg.
rem --------------------------------------------------------------------------
set VCPKG_ROOT=O:\devtools\vcpkg

rem libclang, needed by bindgen. NOTE: ffigen does NOT read this - it needs an
rem explicit --llvm-path, see tools\gen-bridge.sh.
set LIBCLANG_PATH=O:\devtools\LLVM64\bin

rem Some build scripts read this and msys can leave it unset.
set "PROGRAMFILES(X86)=C:\Program Files (x86)"

rem Keep temp on a local disk; builds on the network/virtual drive are slower
rem and some tools mis-handle the drive letter.
set TEMP=C:\Users\dev\AppData\Local\Temp
set TMP=C:\Users\dev\AppData\Local\Temp

set PATH=C:\Users\dev\.cargo\bin;O:\devtools\LLVM64\bin;O:\devtools\flutter\bin;%PATH%

exit /b 0
