# MouseMux RustDesk - Ongoing Maintenance Repository

**Last Updated:** October 19, 2025

## Purpose

This directory (`O:\rustdesk-development\rustdesk-current`) is the **ongoing maintenance repository** for keeping MouseMux protocol support compatible with the latest version of RustDesk.

**Key Goals:**
1. Stay current with upstream RustDesk development
2. Maintain MouseMux V2.1 protocol compatibility
3. Provide users with the latest RustDesk features + MouseMux support
4. Test compatibility before public releases

---

## Directory Structure

```
rustdesk-current/
├── MOUSEMUX_MAINTENANCE.md     # This file - maintenance documentation
├── build.sh                     # Build script for MouseMux Flutter edition
├── clean-build.sh               # Clean build artifacts script
├── src/platform/windows_mousemux.rs  # Core MouseMux protocol implementation
├── src/server/                  # Modified server components
├── flutter/                     # Modified Flutter UI with branding
└── libs/enigo/                  # Modified input library for per-connection IDs
```

---

## Maintenance Workflow

### Regular Updates (Recommended: Weekly or Monthly)

**1. Pull Latest RustDesk Master**
```bash
cd /o/rustdesk-development/rustdesk-current
git checkout master
git pull origin master
```

**2. Check Compatibility**
```bash
# See how many commits ahead of our base
git log db4296533..HEAD --oneline | wc -l

# Review changes to files we modified
git log --oneline --since="1 week ago" -- \
  src/server/connection.rs \
  src/server/input_service.rs \
  libs/enigo/src/win/win_impl.rs \
  src/flutter_ffi.rs
```

**3. Re-apply MouseMux Changes**
```bash
# Switch to MouseMux branch
git checkout mousemux-flutter-current

# Rebase onto latest master
git rebase master

# If conflicts occur, resolve them:
# - Check conflict files
# - Preserve MouseMux-specific code
# - Keep upstream improvements
# - Test build after resolution
```

**4. Test Build**
```bash
./clean-build.sh
./build.sh
```

**5. Test Functionality**
- Start MouseMux application
- Connect 2-3 clients
- Verify per-connection IDs work
- Test input from all clients
- Check user count display

**6. Update Public Branches** (if tests pass)
```bash
# Update the minimal branch in rustdesk-clean
cd /o/rustdesk-development/rustdesk-clean
git checkout mousemux-flutter-minimal

# Copy updated files from rustdesk-current
# ... (manual file comparison and update)

# Commit and push
git add -A
git commit -m "Update to RustDesk [version] - Maintain MouseMux compatibility"
git push origin mousemux-flutter-minimal
```

---

## Files Modified for MouseMux

### Core Protocol Files
- `src/platform/windows_mousemux.rs` - **MouseMux V2.1 protocol implementation**
  - Per-connection ID management
  - Windows message-based IPC with MouseMux app
  - Connection lifecycle tracking

### Input Handling
- `libs/enigo/src/win/win_impl.rs` - **Modified for HashMap-based ID lookup**
  - Separate mouse/keyboard IDs per connection
  - SendInput with per-connection device IDs

### Server Components
- `src/server/connection.rs` - **Connection lifecycle integration**
  - Request IDs on connect
  - Release IDs on disconnect

- `src/server/input_service.rs` - **ID synchronization**
  - Sync IDs between main process and portable service
  - Track connected users

- `src/server/portable_service.rs` - **IPC for elevated process**
  - Synchronize MouseMux state with portable service

### UI Components
- `flutter/lib/desktop/pages/desktop_home_page.dart` - **Branding and user count**
- `flutter/lib/main.dart` - **UI customization**
- `src/flutter_ffi.rs` - **FFI bridge for user count**
- `src/ui.rs`, `src/ui/index.css`, `src/ui/index.tis` - **Sciter UI (legacy)**

### Build Configuration
- `Cargo.toml`, `Cargo.lock` - **Dependency management**
- `build.rs` - **Build script**
- `build.py` - **Python build wrapper**

---

## Compatibility Checklist

Before releasing an update, verify:

### Code Compatibility
- [ ] No merge conflicts in core MouseMux files
- [ ] Protocol constants still match MouseMux application
- [ ] IPC message format unchanged
- [ ] Build completes without errors

### Functional Testing
- [ ] MouseMux detection works
- [ ] ID assignment (WM_APP+30 → WM_APP+100)
- [ ] ID release (WM_APP+40)
- [ ] Multi-client input simultaneous
- [ ] User count display accurate
- [ ] Late-start scenario (MouseMux starts after clients connect)

### Performance
- [ ] No input lag with multiple clients
- [ ] No memory leaks with long sessions
- [ ] Clean disconnect handling

---

## Known Upstream Changes That May Cause Issues

Watch for RustDesk changes in these areas:

### High Risk (Breaking Changes)
1. **Input handling refactoring** (`libs/enigo/`)
   - Our changes: Per-connection ID HashMap
   - Risk: Upstream might change input architecture

2. **Connection lifecycle** (`src/server/connection.rs`)
   - Our changes: ID request/release on connect/disconnect
   - Risk: Connection state management changes

3. **IPC between processes** (`src/server/portable_service.rs`)
   - Our changes: MouseMux ID synchronization
   - Risk: IPC protocol changes

### Medium Risk
4. **Flutter FFI** (`src/flutter_ffi.rs`)
   - Our changes: User count bridge
   - Risk: FFI restructuring

5. **Build system** (`build.py`, `Cargo.toml`)
   - Our changes: Dependencies, build flags
   - Risk: New dependencies, incompatible versions

### Low Risk
6. **UI components** (Flutter pages, Sciter UI)
   - Our changes: Branding, user count display
   - Risk: UI refactoring (usually cosmetic)

---

## Conflict Resolution Guide

If conflicts occur during rebase/merge:

### Step 1: Identify Conflict Type
```bash
git status
# Look for files with "both modified"
```

### Step 2: Understand the Change
```bash
# Show what changed upstream
git log master --oneline -10 -- [conflicted_file]

# Show our changes
git log mousemux-flutter-current --oneline -10 -- [conflicted_file]
```

### Step 3: Resolve Strategically

**For MouseMux-specific files** (e.g., `windows_mousemux.rs`):
- Keep our version entirely
- Review upstream changes to see if they're relevant
- Usually: `git checkout --ours [file]`

**For shared files** (e.g., `connection.rs`):
- Manually merge
- Preserve MouseMux hooks (request_ids, release_ids calls)
- Keep upstream improvements
- Test thoroughly

**For build files** (e.g., `Cargo.toml`):
- Merge dependencies
- Keep both upstream additions and our additions
- Watch for version conflicts

### Step 4: Test After Resolution
```bash
./clean-build.sh
./build.sh
# Full functional testing required
```

---

## Version Tracking

### Base Version
- **RustDesk Base Commit:** `db4296533` (October 2025, nightly branch)
- **MouseMux Protocol:** V2.1

### Current Version
- **RustDesk Master:** `c90d72d72` (as of October 19, 2025)
- **Commits Ahead of Base:** 87 commits
- **Last Updated:** October 19, 2025

### Update History
| Date | RustDesk Version | Commits Ahead | Status | Notes |
|------|------------------|---------------|--------|-------|
| 2025-10-19 | c90d72d72 | 87 | ✅ Compatible | Initial setup |

---

## Build Scripts

### `build.sh`
Main build script for MouseMux Flutter edition.

**Usage:**
```bash
export VCPKG_ROOT=/path/to/vcpkg
./build.sh
```

**Output:**
- `flutter/build/windows/x64/runner/Release/rustdesk.exe`
- `rustdesk-[version]-install.exe` (installer, if created)

### `clean-build.sh`
Removes all build artifacts for a clean rebuild.

**Usage:**
```bash
./clean-build.sh
```

---

## Contact & Support

For questions about maintenance:
- Check `O:\rustdesk-development\PROJECT_HISTORY.md`
- Check `O:\rustdesk-development\TODO.md`
- Review git commit history for resolution examples

---

## Quick Reference Commands

```bash
# Pull latest upstream
git checkout master && git pull

# Switch to MouseMux branch
git checkout mousemux-flutter-current

# Rebase onto latest
git rebase master

# Clean build
./clean-build.sh && ./build.sh

# Check what changed upstream
git log master --oneline --since="1 week ago"

# See our modifications
git diff master..mousemux-flutter-current --stat
```

---

**Remember:** This folder is for **testing compatibility** with the latest RustDesk. Always test thoroughly before updating public branches!
