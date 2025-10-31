# RustDesk MouseMux Edition - Critical Bug Fixes

This document records critical bugs and their fixes for the MouseMux Edition. **NEVER FORGET THESE!**

---

## BUG #1: "Waiting for Image" Issue on Windows 11 (CRITICAL)

**Date Fixed:** 2025-10-31
**Severity:** CRITICAL - Completely breaks remote desktop functionality

### Symptoms
- Client shows "waiting for image..." indefinitely
- Host mouse moves to coordinates (0,0) and clicks
- Portable service starts but never captures frames
- Issue affects Windows 11 hosts
- Official RustDesk 1.4.3 works, MouseMux edition doesn't
- Old working builds suddenly stopped working (environmental change, not code regression)

### Root Cause
**APP_NAME contained spaces: "RustDesk MouseMux Edition"**

This caused Windows 11 to block/restrict access to:
1. **Shared memory paths**: `C:\ProgramData\RustDesk MouseMux Edition\shared_memory_portable_service`
2. **Named pipes**: `\\.\pipe\RustDesk MouseMux Edition\query_portable_service`

Windows 11 appears to have stricter path handling for SYSTEM-level services when paths contain spaces, causing the portable service (running as SYSTEM) to fail silently when creating IPC mechanisms.

### The Fix

**File:** `libs/hbb_common/src/config.rs`
**Line:** 64

**BEFORE (BROKEN):**
```rust
pub static ref APP_NAME: RwLock<String> = RwLock::new("RustDesk MouseMux Edition".to_owned());
```

**AFTER (FIXED):**
```rust
pub static ref APP_NAME: RwLock<String> = RwLock::new("rustdesk-mousemux-edition".to_owned());
```

### Why This Fixes It

1. **No spaces in paths**: Paths become `C:\ProgramData\rustdesk-mousemux-edition\...`
2. **No spaces in named pipes**: `\\.\pipe\rustdesk-mousemux-edition\...`
3. **Windows 11 compatibility**: Removes any ambiguity in path parsing for SYSTEM services
4. **Matches APP_DIR_NAME**: Consistent with the existing `APP_DIR_NAME` constant (line 53) which already uses dashes

### Verification

Check logs to confirm the fix:

**BROKEN (old logs):**
```
[INFO] Create shared memory, flink: C:\ProgramData\RustDesk MouseMux Edition\shared_memory_portable_service
[INFO] Started ipc_portable_service server at path: \\.\pipe\RustDesk MouseMux Edition\query_portable_service
```

**FIXED (new logs):**
```
[INFO] Create shared memory, flink: C:\ProgramData\rustdesk-mousemux-edition\shared_memory_portable_service
[INFO] Started ipc_portable_service server at path: \\.\pipe\rustdesk-mousemux-edition\query_portable_service
```

### Related Code

**Also important:** `libs/hbb_common/src/config.rs` line 53
```rust
pub const APP_DIR_NAME: &str = "rustdesk-mousemux-edition";
```

This constant was already correct and should remain unchanged.

### Additional Notes

- The portable service uses 3-stage privilege escalation: user → admin → SYSTEM
- Shared memory is used for IPC between portable service and main process
- The service starts successfully but fails silently when creating IPC with spaces in paths
- Old builds worked until a Windows 11 update likely tightened security policies
- Official RustDesk uses "RustDesk" (no spaces) and works fine

### Prevention

**NEVER use spaces in APP_NAME!**
- Always use lowercase with dashes: `app-name-here`
- Test on Windows 11 with portable service functionality
- Check logs for actual paths being created

---

## BUG #2: Hardware Codec Build Failures

**Date Fixed:** 2025-10-31
**Severity:** HIGH - Prevents hardware encoding, forces slow software encoding

### Symptoms
- Build fails with multiple "unresolved external symbol" errors
- Missing: `swr_alloc`, `swr_init`, `swr_convert`, `swr_free`, `swr_close`, `swr_is_initialized`
- Missing: `IID_ICodecAPI`, `IID_IMFMediaEventGenerator`, `IID_IMFTransform`
- hwcodec feature compiles but links fail

### Root Cause
hwcodec crate's `build.rs` was missing required library linkage for:
1. FFmpeg audio resampling library (`swresample`)
2. Windows Media Foundation libraries (`mfuuid`, `mfplat`, `strmiids`)

### The Fix

**File:** `C:/Users/Developer/.cargo/git/checkouts/hwcodec-3f3da9ff8e484625/17c1dbb/build.rs`

**Fix 1 - Add swresample (line ~157):**

**BEFORE:**
```rust
let mut static_libs = vec!["avcodec", "avutil", "avformat"];
```

**AFTER:**
```rust
let mut static_libs = vec!["avcodec", "avutil", "avformat", "swresample"];
```

**Fix 2 - Add Windows Media Foundation libs (line ~177):**

**BEFORE:**
```rust
["User32", "bcrypt", "ole32", "advapi32"].to_vec()
```

**AFTER:**
```rust
["User32", "bcrypt", "ole32", "advapi32", "mfuuid", "mfplat", "strmiids"].to_vec()
```

### Build Script Integration

The fixes are automatically applied by `build_complete.sh` (Step 2):

```bash
# Fix 1: Add swresample for FFmpeg audio resampling
sed -i 's/let mut static_libs = vec!\["avcodec", "avutil", "avformat"\];/let mut static_libs = vec!["avcodec", "avutil", "avformat", "swresample"];/' "$HWCODEC_BUILD_RS"

# Fix 2: Add Windows Media Foundation libraries
sed -i 's/\["User32", "bcrypt", "ole32", "advapi32"\]\.to_vec()/["User32", "bcrypt", "ole32", "advapi32", "mfuuid", "mfplat", "strmiids"].to_vec()/' "$HWCODEC_BUILD_RS"
```

### Required Dependencies

Install via vcpkg before building:
```bash
./vcpkg install ffmpeg:x64-windows-static mfx-dispatch:x64-windows-static
```

### Cache Clearing

After applying fixes, **MUST** clear hwcodec build cache:
```bash
rm -rf target/release/.fingerprint/hwcodec-* \
       target/release/build/hwcodec-* \
       target/release/deps/*hwcodec* \
       target/release/*hwcodec*
```

Otherwise Cargo will use old cached `.rlib` files compiled without the fixes!

### Verification

Check build output for:
- ✅ No linker errors
- ✅ `hwcodec` feature compiled successfully
- ✅ Logs show H265 hardware encoding (not VP9 software encoding)

**Log verification:**
```
usable: vp8=true, av1=true, h264=true, h265=true  # ← h265 should be true!
```

### Prevention

- Always use `build_complete.sh` which applies these fixes automatically
- Keep vcpkg dependencies up to date
- Clear cache when switching between hwcodec/non-hwcodec builds

---

## How to Use These Fixes

### Quick Reference

1. **APP_NAME fix** is permanent in source code (already applied)
2. **hwcodec fixes** are automated in `build_complete.sh`
3. Always use `./build_complete.sh` for new builds
4. Test on Windows 11 with portable service

### Build Commands

```bash
# Full build from scratch
./build_complete.sh

# Skip dependency installation (if already installed)
./build_complete.sh --from-step 2

# Just package existing build
./build_complete.sh --from-step 6

# List all available steps
./build_complete.sh --list
```

### Testing Checklist

After building with these fixes:

- [ ] Check APP_NAME in logs (should be `rustdesk-mousemux-edition`)
- [ ] Check shared memory path (no spaces)
- [ ] Check named pipe path (no spaces)
- [ ] Verify H265 hardware encoding enabled
- [ ] Test remote connection on Windows 11
- [ ] Confirm "waiting for image" is resolved
- [ ] Verify portable service captures frames

---

## Maintenance Notes

### When Updating hwcodec Dependency

If `hwcodec` version changes in `Cargo.toml`:
1. The checkout path in `HWCODEC_BUILD_RS` variable may change
2. Update path in `build_complete.sh` line 22
3. Re-verify fixes are still needed
4. Update this document if fixes change

### When Updating APP_NAME

**DON'T!** Unless absolutely necessary. If you must:
1. Use lowercase with dashes only
2. Update `APP_DIR_NAME` to match (line 53)
3. Update all documentation
4. Test portable service thoroughly on Windows 11
5. Update `clean-appdata.sh` to handle old and new names

---

## Historical Context

### Why It Took So Long to Find

1. **Misleading symptoms**: Mouse moving to 0,0 suggested auto-login feature bug
2. **Silent failure**: Portable service started but failed silently (no errors logged)
3. **Environmental change**: Old working builds stopped working (Windows update suspected)
4. **Hwcodec red herring**: Initially thought missing hardware codec was the cause
5. **Log comparison**: Only by comparing official RustDesk logs did we notice the path differences

### Timeline

- **Oct 18**: Last working version (used dashed name)
- **Oct 25**: Version with spaces introduced
- **Oct 31**: Issue discovered and fixed

### Key Insight

**The fix was hiding in plain sight!**
- `APP_DIR_NAME` constant already used dashes (correct)
- `APP_NAME` RwLock used spaces (wrong)
- The inconsistency should have been a red flag

---

## Contact

If you encounter similar issues:
1. Check logs for actual paths being created
2. Verify APP_NAME has no spaces
3. Compare with official RustDesk behavior
4. Test portable service specifically on Windows 11

**Remember:** Spaces in Windows paths + SYSTEM privileges = BAD TIME!
