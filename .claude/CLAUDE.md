# RustDesk Build Process - Complete History

## ✅ Latest Build - MouseMux V2.1 with Correct Install Path (October 14, 2025)

**Status:** ✅ **COMPLETE - READY FOR TESTING**

**Git Branch:** mousemux
**Final Outputs:**
- ✅ Executable: `target/release/rustdesk.exe` (27MB)
- ✅ Installer: `rustdesk-1.4.2-mousemux-v2.1-x86_64.exe` (11MB)
- ✅ Install path: `C:\Program Files\RustDesk\` (CORRECT!)
- ✅ Window title: "RustDesk" (uses get_app_name())
- ✅ MouseMux V2.1: Hidden top-level window (findable by MouseMux)

**Critical Fixes Today:**
1. **Fixed MouseMux window finding issue** (commit 7b8d1aa93)
   - Changed from message-only window (HWND_MESSAGE) to hidden top-level window
   - MouseMux can now find "rustdesk.mousemux.window.query" using FindWindowA

2. **Fixed portable packer embedding wrong data.bin** (October 14)
   - Root cause: Packer was embedding OLD data.bin from October 13
   - Solution: Delete old libs/portable/data.bin, regenerate with correct rustdesk.exe
   - The packer uses include_bytes!("../data.bin") at compile time
   - CRITICAL: generate.py must output to libs/portable/ not target/release/

**Build Timeline Today:**
- Started: resources/rustdesk.exe from yesterday (Oct 13 17:35) - had correct APP_NAME
- Problem: Portable packer still showed long install path
- Investigation: Packer embedded libs/portable/data.bin from Oct 13 21:57 (OLD!)
- Solution: Deleted old data.bin, regenerated with correct rustdesk.exe
- Result: Fresh packer at 18:58 with correct install path

**Git Commits:**
- `5b1671ab5` - Update build script: Comment out cargo clean
- `4753c1e54` - Fix build script: Correct data.bin generation path
- `7b8d1aa93` - Fix MouseMux window finding issue

---

## ✅ Build Success Summary

**Status:** Successfully built RustDesk 1.4.2 with Sciter UI and MouseMux V2.1

**Final Outputs:**
- ✅ Executable: `target/release/rustdesk.exe` (27MB)
- ✅ Installer: `rustdesk-1.4.2-mousemux-v2.1-x86_64.exe` (11MB)

**Build Time:** ~15 minutes total

**Key Configuration:**
- Rust 1.75.0 (via rustup override)
- Sciter UI (not Flutter - Flutter is broken)
- Git branch: nightly
- VCPKG_ROOT: /o/rustdesk-build/vcpkg

**Quick Rebuild:**
```bash
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
cd /o/rustdesk-build/rustdesk
cargo build --release
```

---

## MouseMux Integration Implementation Status

### ✅ Protocol V2.1 - Per-Connection IDs (October 9, 2025)

**Status:** ✅ **COMPLETE AND BUILT SUCCESSFULLY**

**Git Branch:** mousemux
**Build Output:** target/release/rustdesk.exe (27MB, built Oct 9 16:11)
**Total Commits:** 6 commits (Phase 1-5 + compilation fixes)
**Patch Files:** 5 patches in `C:\Users\Developer\Desktop\test\mousemux-v2.1-patches\`

**Why V2.1?** V2 was implemented but not yet tested. Design review revealed that the global ID model (all connections share same IDs) prevents multiple users from collaborating simultaneously. V2.1 redesigns the protocol to assign unique IDs per connection, enabling true multi-user collaboration.

**Implementation Complete:**
- ✅ Phase 1: Core protocol (windows_mousemux.rs) - HashMap-based state management
- ✅ Phase 2: Connection lifecycle (connection.rs) - Pass conn_id and peer_info
- ✅ Phase 3: ID synchronization (input_service.rs) - Per-connection ID sync
- ✅ Phase 4: Enigo updates (win_impl.rs) - HashMap-based ID lookup
- ✅ Phase 5: Keyboard events (connection.rs) - Propagate conn_id through input pipeline
- ✅ All compilation errors fixed (type mismatches, borrow checker, function signatures)
- ✅ Successful build: 15m 57s, 30 warnings (non-critical)
- ✅ Comprehensive documentation: .claude/MOUSEMUX_V2.1_IMPLEMENTATION.md (511 lines)

**Next Step:** Testing with actual MouseMux application to verify protocol implementation

---

### ✅ MouseMux V2.1: Portable Service IPC Synchronization Fix (October 13, 2025)

**Status:** ✅ **COMPLETE - BUILD SUCCESSFUL**

**Git Commit:** `b0c802199` - MouseMux V2.1: Fix portable service IPC synchronization for multi-client support
**Patch File:** `C:\Users\Developer\Desktop\test\mousemux-v2.1-patches\0001-MouseMux-V2.1-Fix-portable-service-IPC-synchronizati.patch`
**Build Time:** 15m 57s
**Build Output:** target/release/rustdesk.exe

#### Problem Description

**Critical Bug:** MouseMux IDs were not synchronized to the portable service process, causing all SendInput calls to use the default ID (100) instead of unique per-connection IDs (6001+).

**Root Cause:** RustDesk uses a two-process architecture on Windows:
- **Main Process:** Handles networking, UI, receives input from remote clients
- **Portable Service Process:** Elevated/SYSTEM process that performs actual SendInput calls

Each process has its own separate Enigo instance. MouseMux V2.1 stored IDs only in the main process's Enigo, but all input injection happens in the portable service process which had no access to these IDs.

#### Solution Implemented

Added IPC (Inter-Process Communication) synchronization of MouseMux IDs between processes:

**1. IPC Message Type (src/ipc.rs)**
- Added `MouseMuxIds(i32, Option<u32>, Option<u32>)` variant to DataPortableService enum
- Carries conn_id, mouse_id, and keyboard_id across process boundary

**2. Synchronization Function (src/server/input_service.rs)**
- Added `sync_mousemux_ids(conn_id)` function
- Sends MouseMux IDs via IPC to portable service whenever they change
- Added `set_enigo_mousemux_ids()` public helper to update Enigo without exposing private ENIGO static

**3. Portable Service Handler (src/server/portable_service.rs)**
- Added IPC message handler for MouseMuxIds in `run_ipc_client()`
- Calls `set_enigo_mousemux_ids()` to update portable service's Enigo instance
- Added `send_mousemux_ids()` function in client module to send IDs via IPC

**4. Protocol Integration (src/platform/windows_mousemux.rs)**
- Modified `set_ids()` and `clear_ids()` to call `sync_mousemux_ids()` after state changes
- Ensures portable service stays synchronized with main process

#### Data Flow

```
1. MouseMux → Main Process (WM_APP+100/110)
   → window_proc receives mouse_id/keyboard_id for conn_id

2. Main Process → windows_mousemux.rs
   → set_ids() stores IDs in HashMap

3. windows_mousemux.rs → input_service.rs
   → sync_mousemux_ids(conn_id) called

4. input_service.rs → IPC → Portable Service
   → MouseMuxIds message sent via named pipe

5. Portable Service IPC Handler
   → Receives MouseMuxIds(conn_id, mouse_id, keyboard_id)

6. Portable Service → set_enigo_mousemux_ids()
   → Updates portable service's Enigo HashMap

7. Portable Service → SendInput()
   → Uses correct IDs (6001/6002) instead of default 100
```

#### Files Modified

- **src/ipc.rs:** Added MouseMuxIds enum variant (+1 line)
- **src/server/input_service.rs:** Added sync and helper functions (+12 lines)
- **src/server/portable_service.rs:** Added IPC handler and sender (+26 lines)
- **src/platform/windows_mousemux.rs:** Added sync calls in set_ids/clear_ids (+2 lines, refactored logging)

**Net Change:** +82 insertions, -14 deletions

#### Compilation Fix

**Initial Build Error:** `error[E0603]: static 'ENIGO' is private`
- portable_service.rs tried to access `crate::input_service::ENIGO.lock()` directly
- ENIGO is a private static within lazy_static! block

**Fix:** Created public helper function `set_enigo_mousemux_ids()` in input_service.rs
- Provides controlled access to update Enigo without exposing private static
- Maintains proper encapsulation and privacy boundaries

#### Testing Status

- ✅ **Build:** Successful compilation with no errors
- ⏳ **Runtime:** Pending verification with MouseMux application
- ⏳ **Multi-Client:** Pending test with multiple simultaneous connections
- ⏳ **ID Verification:** Pending log verification that SendInput uses 6001+ instead of 100

#### Documentation

Complete implementation details documented in:
- **MOUSEMUX_V2.1_HISTORY.md:** Comprehensive root cause analysis, data flow diagrams, verification steps

---

## 📋 MouseMux Protocol V2.1 - COMPLETE SPECIFICATION

### Protocol Overview

MouseMux V2.1 enables **multiple simultaneous users** to control a single Windows host, each with their own independent mouse cursor and keyboard. This is achieved by:
1. Assigning unique IDs per connection (not globally shared)
2. Tracking connections via `conn_id` (RustDesk's internal connection identifier)
3. Transmitting peer identity to MouseMux for user-friendly display
4. Using HashMap-based ID lookup during input injection

---

### Message Protocol Table

| Message | Direction | When | wParam | lParam | Purpose |
|---------|-----------|------|--------|--------|---------|
| **WM_APP+10** | RustDesk → MouseMux | **RustDesk starts** | RustDesk version (142) | RustDesk HWND* | "I'm running at version X, send responses to this window" |
| **WM_APP+20** | RustDesk → MouseMux | **RustDesk exits** | RustDesk version (142) | RustDesk HWND* | "I'm shutting down" (consistent params with +10) |
| **WM_APP+30** | RustDesk → MouseMux | **Client connects** | conn_id | Protocol version (121) | "Connection #N started, using protocol v1.21" |
| **WM_APP+32** | RustDesk → MouseMux | **Peer info char** | conn_id | char_code (or 0) | "Peer info character for connection #N" |
| **WM_APP+34** | RustDesk → MouseMux | **Peer info done** | conn_id | 0 | "All info sent, please generate IDs now" |
| **WM_APP+40** | RustDesk → MouseMux | **Client disconnects** | conn_id | 0 | "Connection #N ended, release its IDs" |
| **WM_APP+100** | MouseMux → RustDesk | **Mouse ID assigned** | conn_id | mouse_id | "Mouse ID for connection #N is Y" |
| **WM_APP+110** | MouseMux → RustDesk | **Keyboard ID assigned** | conn_id | keyboard_id | "Keyboard ID for connection #N is Z" |

**\*HWND:** The window handle to **"rustdesk.mousemux.window.query"** - RustDesk's message-only window created at startup to receive callbacks from MouseMux.

---

### Protocol Constants

```rust
const WM_APP: u32 = 0x8000;
const WM_MOUSEMUX_STARTUP: u32 = WM_APP + 10;        // 0x800A
const WM_MOUSEMUX_SHUTDOWN: u32 = WM_APP + 20;       // 0x8014
const WM_MOUSEMUX_CONN_START: u32 = WM_APP + 30;     // 0x801E
const WM_MOUSEMUX_PEER_INFO_CHAR: u32 = WM_APP + 32; // 0x8020
const WM_MOUSEMUX_PEER_INFO_DONE: u32 = WM_APP + 34; // 0x8022
const WM_MOUSEMUX_CONN_END: u32 = WM_APP + 40;       // 0x8028
const WM_MOUSEMUX_MOUSE_ID: u32 = WM_APP + 100;      // 0x8064
const WM_MOUSEMUX_KEYBOARD_ID: u32 = WM_APP + 110;   // 0x806E

const PROTOCOL_VERSION: u32 = 121;  // V2.1 = 121
const RUSTDESK_VERSION: u32 = 142;  // 1.4.2 = 142
```

---

### Complete Connection Flow

#### 1. RustDesk Startup
```
1. RustDesk creates message window "rustdesk.mousemux.window.query"
   → hwnd = 0x00AB1234 (example)

2. RustDesk → MouseMux:
   PostMessage(mousemux_hwnd, WM_APP+10, wParam=142, lParam=0x00AB1234)

   MouseMux now knows:
   - RustDesk version 1.4.2 is running
   - Send responses to hwnd 0x00AB1234
```

#### 2. Client Connection (Example: "John@laptop123" connects as conn_id=5)

**Step 1: Connection Start**
```
RustDesk → MouseMux:
PostMessage(WM_APP+30, wParam=5, lParam=121)

MouseMux creates entry: connections[5] = {peer_info: "", mouse_id: null, keyboard_id: null}
```

**Step 2: Peer Info Transmission (character-by-character)**
```
Peer string: "John@laptop123" (14 characters)

RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='J')   // char 0
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='o')   // char 1
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='h')   // char 2
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='n')   // char 3
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='@')   // char 4
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='l')   // char 5
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='a')   // char 6
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='p')   // char 7
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='t')   // char 8
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='o')   // char 9
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='p')   // char 10
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='1')   // char 11
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='2')   // char 12
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam='3')   // char 13
RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=5, lParam=0)     // null terminator

MouseMux appends each char to connections[5].peer_info → "John@laptop123"
```

**Step 3: Trigger ID Generation**
```
RustDesk → MouseMux:
PostMessage(WM_APP+34, wParam=5, lParam=0)

MouseMux:
- Assigns mouse_id = 6001
- Assigns keyboard_id = 6002
- Updates: connections[5] = {peer_info: "John@laptop123", mouse_id: 6001, keyboard_id: 6002}
```

**Step 4: ID Assignment Response**
```
MouseMux → RustDesk:
PostMessage(rustdesk_hwnd, WM_APP+100, wParam=5, lParam=6001)  // mouse ID
PostMessage(rustdesk_hwnd, WM_APP+110, wParam=5, lParam=6002)  // keyboard ID

RustDesk receives in window_proc:
- Stores: connections[5] = {peer_info: "John@laptop123", mouse_id: 6001, keyboard_id: 6002}
- Calls: sync_mousemux_ids(5) to update Enigo
```

**Step 5: Input Injection**
```
When conn_id=5 sends mouse/keyboard input:
- Look up IDs: connections[5] → mouse_id=6001, keyboard_id=6002
- SendInput() with dwExtraInfo = 6001 (for mouse) or 6002 (for keyboard)
- MouseMux sees ID 6001 → displays as "John@laptop123's cursor"
```

#### 3. Second Client Connection (Example: "Alice@desktop" as conn_id=6)

```
1. RustDesk → MouseMux: PostMessage(WM_APP+30, wParam=6, lParam=121)
2. RustDesk → MouseMux: 14x PostMessage(WM_APP+32, wParam=6, lParam=char) for "Alice@desktop"
3. RustDesk → MouseMux: PostMessage(WM_APP+32, wParam=6, lParam=0) — null
4. RustDesk → MouseMux: PostMessage(WM_APP+34, wParam=6, lParam=0) — generate IDs
5. MouseMux → RustDesk: PostMessage(WM_APP+100, wParam=6, lParam=6003) — mouse ID
6. MouseMux → RustDesk: PostMessage(WM_APP+110, wParam=6, lParam=6004) — keyboard ID
7. RustDesk stores: connections[6] = {peer_info: "Alice@desktop", mouse_id: 6003, keyboard_id: 6004}

Now TWO users are active:
- conn_id=5: John using IDs 6001/6002
- conn_id=6: Alice using IDs 6003/6004
```

#### 4. Client Disconnection (conn_id=5 disconnects)

```
RustDesk → MouseMux:
PostMessage(WM_APP+40, wParam=5, lParam=0)

MouseMux:
- Releases IDs 6001 and 6002
- Removes connections[5]

RustDesk:
- Removes connections[5] from HashMap
- No longer injects input with IDs 6001/6002
```

#### 5. RustDesk Shutdown

```
RustDesk → MouseMux:
PostMessage(WM_APP+20, wParam=142, lParam=rustdesk_hwnd)

RustDesk:
- Destroys "rustdesk.mousemux.window.query" window
- Terminates message loop thread

MouseMux:
- Releases all remaining IDs for this RustDesk instance
```

---

### Peer Info String Specification

**Format:** `"{name}@{id}"`
- Example: "John's Laptop@abc123def456"
- Source: `format!("{}@{}", self.lr.my_name, self.lr.my_id)`
- Max length: **256 characters** (truncate if longer)
- Character encoding: UTF-8 → cast to `u8` for lParam
- Fallback: If both name and ID are empty → `format!("conn_{}", conn_id)`

**Special Characters:**
- Spaces: Allowed ("John's Laptop" → works)
- Unicode: Should work (cast to u8, may truncate multibyte)
- Null bytes: Only sent as terminator (lParam=0)

**Transmission Rules:**
1. Send WM_APP+32 for each character (sequential, no index needed)
2. Always send null terminator (lParam=0) after last character
3. Then send WM_APP+34 to trigger ID generation
4. Do NOT send WM_APP+34 before null terminator

---

### State Management

#### RustDesk State Structures

```rust
use std::collections::HashMap;

pub struct MouseMuxState {
    pub hwnd: Option<SendSyncHwnd>,  // Handle to "rustdesk.mousemux.window.query"
    pub connections: HashMap<i32, MouseMuxConnectionIDs>,  // conn_id → IDs
    pub pending_peer_info: HashMap<i32, String>,  // Temporary storage while receiving WM_APP+32
}

pub struct MouseMuxConnectionIDs {
    pub conn_id: i32,
    pub peer_info: String,       // e.g., "John@laptop123"
    pub mouse_id: Option<u32>,   // Assigned by MouseMux
    pub keyboard_id: Option<u32>, // Assigned by MouseMux
}

impl MouseMuxState {
    pub fn new() -> Self {
        Self {
            hwnd: None,
            connections: HashMap::new(),
            pending_peer_info: HashMap::new(),
        }
    }
}
```

**Old V2 (Global IDs - ❌ SUPERSEDED):**
```rust
pub struct MouseMuxState {
    pub hwnd: Option<SendSyncHwnd>,
    pub mouse_id: Option<u32>,      // ❌ All connections shared same IDs
    pub keyboard_id: Option<u32>,   // ❌ Could not support multiple users
}
```

---

### Window Procedure Message Handling

```rust
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_MOUSEMUX_MOUSE_ID => {  // WM_APP+100
            let conn_id = wparam as i32;
            let mouse_id = lparam as u32;

            log::info!("MouseMux V2.1: Received mouse ID {} for conn_id {}", mouse_id, conn_id);

            if let Ok(mut state) = MOUSEMUX_STATE.lock() {
                state.connections
                    .entry(conn_id)
                    .or_insert(MouseMuxConnectionIDs {
                        conn_id,
                        peer_info: String::new(),
                        mouse_id: None,
                        keyboard_id: None,
                    })
                    .mouse_id = Some(mouse_id);
            }

            // Sync to Enigo
            crate::server::input_service::sync_mousemux_ids(conn_id);
            0
        }

        WM_MOUSEMUX_KEYBOARD_ID => {  // WM_APP+110
            let conn_id = wparam as i32;
            let keyboard_id = lparam as u32;

            log::info!("MouseMux V2.1: Received keyboard ID {} for conn_id {}", keyboard_id, conn_id);

            if let Ok(mut state) = MOUSEMUX_STATE.lock() {
                state.connections
                    .entry(conn_id)
                    .or_insert(MouseMuxConnectionIDs {
                        conn_id,
                        peer_info: String::new(),
                        mouse_id: None,
                        keyboard_id: None,
                    })
                    .keyboard_id = Some(keyboard_id);
            }

            // Sync to Enigo
            crate::server::input_service::sync_mousemux_ids(conn_id);
            0
        }

        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}
```

---

### Implementation Plan

#### Phase 1: Update windows_mousemux.rs (Core Protocol)
**File:** `src/platform/windows_mousemux.rs`

- [x] Update constants: Add WM_APP+32, WM_APP+34, remove old V2 constants
- [ ] Update `MouseMuxState` struct: Add `connections` HashMap and `pending_peer_info`
- [ ] Update `window_proc`: Handle WM_APP+100 (mouse ID only, extract conn_id)
- [ ] Update `window_proc`: Add WM_APP+110 handler (keyboard ID)
- [ ] Update `window_proc`: Add WM_APP+32 handler (receive peer info chars)
- [ ] Update `notify_startup()`: Keep version + HWND (already correct!)
- [ ] Update `notify_shutdown()`: Change params to match notify_startup (version + HWND)
- [ ] Update `request_ids()`: Change signature to `request_ids(conn_id: i32, peer_info: &str)`
  - Send WM_APP+30 with conn_id and protocol version
  - Send WM_APP+32 for each character
  - Send WM_APP+34 to trigger generation
- [ ] Update `release_ids()`: Change signature to `release_ids(conn_id: i32)`
  - Send WM_APP+40 with conn_id only
- [ ] Add `get_ids_for_connection(conn_id) -> Option<(u32, u32)>` function
- [ ] Update `clear_ids()` to remove specific conn_id from HashMap

#### Phase 2: Update connection.rs (Pass conn_id and peer_info)
**File:** `src/server/connection.rs`

- [ ] In `on_remote_authorized()` (line ~1671):
  - Build peer_info string: `format!("{}@{}", self.lr.my_name, self.lr.my_id)`
  - Truncate to 256 chars if longer
  - Fallback to `format!("conn_{}", self.inner.id())` if empty
  - Call `request_ids(self.inner.id(), &peer_info)`
- [ ] In `on_close()` (line ~3778):
  - Call `release_ids(self.inner.id())`

#### Phase 3: Update input_service.rs (Per-Connection ID Sync)
**File:** `src/server/input_service.rs`

- [ ] Update `sync_mousemux_ids()` signature: `sync_mousemux_ids(conn_id: i32)`
- [ ] Get IDs from windows_mousemux: `get_ids_for_connection(conn_id)`
- [ ] Sync to Enigo: `ENIGO.lock().unwrap().set_mousemux_ids(conn_id, mouse_id, keyboard_id)`

#### Phase 4: Update enigo/win_impl.rs (HashMap-Based ID Lookup)
**File:** `libs/enigo/src/win/win_impl.rs`

- [ ] Replace single `mousemux_mouse_id/keyboard_id` with `HashMap<i32, (u32, u32)>`
- [ ] Add `set_mousemux_ids(conn_id, mouse_id, keyboard_id)` method
- [ ] Update `get_mouse_extra_info()` to accept `conn_id` parameter
- [ ] Update `get_keyboard_extra_info()` to accept `conn_id` parameter
- [ ] Update all `mouse_event()` calls to pass conn_id and look up IDs
- [ ] Update all `keybd_event()` calls to pass conn_id and look up IDs

#### Phase 5: Update Input Event Flow (Add conn_id to Keyboard Events)
**File:** `src/server/connection.rs` (input handling)

- [ ] Find all keyboard input handling code
- [ ] Add conn_id to `MessageInput::Key` enum variant: `Key((KeyEvent, i32))`
- [ ] Pass `self.inner.id()` when creating keyboard input messages
- [ ] Ensure conn_id propagates to Enigo at SendInput time

---

### Testing Checklist

- [ ] **RustDesk startup**: Creates window, sends WM_APP+10 successfully
- [ ] **Single client connects**: Gets unique mouse + keyboard IDs
- [ ] **Two clients simultaneous**: Each gets different IDs (6001/6002, 6003/6004)
- [ ] **Three clients**: All get unique IDs, all cursors visible in MouseMux
- [ ] **Client disconnects**: IDs released via WM_APP+40, HashMap entry removed
- [ ] **Client reconnects**: Gets NEW IDs (not recycled immediately)
- [ ] **Peer info transmission**: All characters received correctly
- [ ] **Peer info with spaces**: "John's Laptop" works correctly
- [ ] **Peer info with special chars**: "@", "-", "_" work correctly
- [ ] **Empty peer info**: Falls back to "conn_{id}"
- [ ] **Long peer info (>256 chars)**: Truncates without crash
- [ ] **Input injection**: Each client's input uses correct IDs
- [ ] **RustDesk shutdown**: Sends WM_APP+20 with version + HWND

---

### Files to Modify

1. **`src/platform/windows_mousemux.rs`** (~500 lines expected)
   - Core protocol implementation
   - HashMap state management
   - All message handlers

2. **`src/server/connection.rs`** (~1900 lines, modify ~10 lines)
   - Build and pass peer_info string
   - Pass conn_id to request/release functions

3. **`src/server/input_service.rs`** (~300 lines, modify ~5 lines)
   - Update sync function signature
   - Pass conn_id when syncing

4. **`libs/enigo/src/win/win_impl.rs`** (~800 lines, modify ~50 lines)
   - HashMap-based ID storage
   - Per-connection ID lookup
   - Update all SendInput calls

5. **`src/server/connection.rs` (input handling)** (~4000 lines, modify ~20 lines)
   - Add conn_id to keyboard event enum
   - Pass conn_id through input pipeline

---

### Commit Strategy

**Commit after each phase** with descriptive messages:
- `Phase 1: Implement MouseMux V2.1 protocol in windows_mousemux.rs`
- `Phase 2: Update connection.rs to pass conn_id and peer_info`
- `Phase 3: Update input_service for per-connection ID sync`
- `Phase 4: Add HashMap-based ID lookup to Enigo`
- `Phase 5: Add conn_id to keyboard input events`

**Generate patch files** after each commit:
```bash
git format-patch -1 HEAD --output=mousemux-v2.1-phaseN.patch
```

Store patches in: `C:\Users\Developer\Desktop\test\mousemux-v2.1-patches\`

---

### ✅ Protocol V2 Implementation Complete (October 8, 2025)

**Status:** ⚠️ **SUPERSEDED BY V2.1** (see below for V2.1 specification)

The original V1 implementation has been completely replaced with a new bidirectional asynchronous protocol design.

#### Protocol V2 Key Changes:
- **Asynchronous Communication**: Uses PostMessage instead of SendMessage (non-blocking)
- **Bidirectional Messaging**: RustDesk creates window to receive messages from MouseMux
- **Connection-Based IDs**: IDs assigned when client connects, not on startup
- **Dual ID System**: Separate IDs for mouse and keyboard input
- **HWND Verification**: RustDesk passes its window handle for identity verification
- **Automatic Synchronization**: IDs automatically sync to Enigo instance when received

#### New Message Protocol:

| Message | Direction | When | wParam | lParam | Purpose |
|---------|-----------|------|--------|--------|---------|
| **WM_APP+10** | RustDesk → MouseMux | Startup | Version | RustDesk HWND | "RustDesk is running" |
| **WM_APP+20** | RustDesk → MouseMux | Shutdown | 0 | 0 | "RustDesk is exiting" |
| **WM_APP+30** | RustDesk → MouseMux | Client connects | 0 | 0 | "Please assign IDs" |
| **WM_APP+40** | RustDesk → MouseMux | Client disconnects | Mouse ID | Keyboard ID | "Release these IDs" |
| **WM_APP+100** | MouseMux → RustDesk | After WM_APP+30 | Mouse ID | Keyboard ID | "Here are your IDs" |

#### Implementation Phases:

**Phase 1: Core Infrastructure** ✅ **COMPLETE**
- ✅ Created Windows message window "rustdesk.mousemux.window.query"
- ✅ Background thread with message loop
- ✅ WndProc to handle incoming WM_APP+100 messages
- ✅ Thread-safe state management with Arc<Mutex<>>
- **Commit:** `c06fc9ec6` - Phase 1 implementation
- **Files:** src/platform/windows_mousemux.rs (NEW, 413 lines)

**Phase 2: Protocol Implementation** ✅ **COMPLETE**
- ✅ Startup: Create window, send WM_APP+10 with version and HWND
- ✅ Connection: Send WM_APP+30 on client authorization (src/server/connection.rs)
- ✅ Disconnection: Send WM_APP+40 with IDs on client close
- ✅ Shutdown: Send WM_APP+20 in global_clean() (src/common.rs)
- **Commits:**
  - `4c83f89e6` - Phase 2A (communication functions)
  - `d1977cb22` - Phase 2B (startup/shutdown integration)
  - `6566c1623` - Phase 2C (connection lifecycle hooks)
- **Files:** src/server.rs, src/common.rs, src/server/connection.rs

**Phase 3: Input Injection Updates** ✅ **COMPLETE**
- ✅ Split Enigo struct into mousemux_mouse_id + mousemux_keyboard_id
- ✅ Updated all mouse_event() calls to use get_mouse_extra_info()
- ✅ Updated all keybd_event() calls to use get_keyboard_extra_info()
- ✅ Added set_mousemux_ids() method for V2 protocol
- **Commit:** `6566c1623` - Phase 3 implementation
- **Files:** libs/enigo/src/win/win_impl.rs

**Phase 4: Integration & Cleanup** ✅ **COMPLETE**
- ✅ Added sync_mousemux_ids() to input_service
- ✅ Automatic ID sync when WM_APP+100 received
- ✅ Automatic ID sync when IDs cleared
- ✅ Updated portable service (removed V1 code)
- ✅ Removed all V1 code (enable_mousemux, disable_mousemux, etc.)
- ✅ Removed V1 constants (MOUSEMUX_WINDOW_CLASS, MOUSEMUX_MSG_*, etc.)
- **Commit:** `7bdcd52a1` - Phase 4 integration and cleanup
- **Net code change:** -135 lines (removed 161 lines of V1 code, added 26 lines of V2 integration)

#### Implementation Summary:

**New Files Created:**
- `src/platform/windows_mousemux.rs` (413 lines) - Complete V2 protocol implementation

**Files Modified:**
- `src/platform/mod.rs` - Added windows_mousemux module
- `src/server.rs` - Startup integration (init window + notify)
- `src/common.rs` - Shutdown integration (notify + cleanup)
- `src/server/connection.rs` - Connection lifecycle hooks (request/release IDs)
- `src/server/input_service.rs` - Added sync function, removed V1 helpers
- `src/server/portable_service.rs` - Removed V1 initialization
- `libs/enigo/src/win/win_impl.rs` - Dual ID system, removed V1 methods

**Total Commits:** 6 commits across 4 phases
**Total Patch Files:** 6 patches in `C:\Users\Developer\Desktop\test\mousemux-v2-patches\`

#### Integration Flow:
1. **Server Startup** → `init_mousemux_window()` creates message window
2. **Server Startup** → `notify_startup(version)` posts WM_APP+10 to MouseMux
3. **Client Connects** → `on_remote_authorized()` posts WM_APP+30 requesting IDs
4. **MouseMux Responds** → Posts WM_APP+100 with mouse_id and keyboard_id
5. **Window Proc** → Receives WM_APP+100, stores IDs, calls `sync_mousemux_ids()`
6. **Enigo Updated** → All SendInput calls use correct mouse/keyboard IDs
7. **Client Disconnects** → `on_close()` posts WM_APP+40 releasing IDs
8. **IDs Cleared** → `clear_ids()` resets state, calls `sync_mousemux_ids()`
9. **Server Shutdown** → `global_clean()` posts WM_APP+20, destroys window

#### Testing Status:
- ⏳ **Pending:** Integration testing with MouseMux application
- ⏳ **Pending:** Multi-client connection testing
- ⏳ **Pending:** ID assignment/release verification

---

### About This Build

**RustDesk (MouseMux compliant edition)** is a modified version of RustDesk that enables multiple people to connect to one host simultaneously and collaborate in real-time through MouseMux.

With MouseMux integration, each connected user can have their own independent mouse cursor and keyboard control, allowing true multi-user collaboration on a single Windows host machine. This is perfect for:
- Pair programming and code reviews
- Collaborative design and editing
- Remote training and demonstrations
- Technical support with multiple technicians

**Visit [mousemux.com](https://mousemux.com) for more information about MouseMux and how to set it up.**

---

### ✅ Completed Components

1. **Enigo Library (libs/enigo/src/win/win_impl.rs)**
   - ✅ Added MouseMux state field to Enigo struct
   - ✅ Implemented `enable_mousemux(version)` method
   - ✅ Implemented `disable_mousemux()` method
   - ✅ Implemented `is_mousemux_enabled()` method
   - ✅ Modified all `mouse_event()` calls to use MouseMux ID when enabled
   - ✅ Modified all `keybd_event()` calls to use MouseMux ID when enabled
   - ✅ Win32 API integration for window detection and messaging
   - ✅ Added comprehensive SendInput logging to track dwExtraInfo values

2. **Input Service (src/server/input_service.rs)**
   - ✅ Added global `enable_mousemux(version)` function
   - ✅ Added global `disable_mousemux()` function
   - ✅ Added global `is_mousemux_enabled()` function
   - ✅ Functions control the shared ENIGO static instance
   - ✅ Auto-enable on server startup (with 500ms delay)
   - ✅ Manual trigger function for testing

3. **Portable Service (src/server/portable_service.rs)** ⭐ **CRITICAL FIX**
   - ✅ Fixed MouseMux registration in elevated portable service process
   - ✅ Moved registration from `start_portable_service()` to `run_portable_service()`
   - ✅ Portable service process now gets unique MouseMux ID
   - ✅ All SendInput calls now use correct ID instead of default 100

4. **UI Interface (src/ui_interface.rs & src/ui.rs)**
   - ✅ Added `get_mousemux_enabled()` function
   - ✅ Added `set_mousemux_enabled(enabled)` function
   - ✅ Exposed functions to Sciter UI layer via function declarations
   - ✅ Cross-platform support (Windows implementation, stubs for other platforms)

### 🔄 Programmatic Usage (Available Now)

To enable MouseMux programmatically from Rust code:

```rust
#[cfg(windows)]
use crate::server::input_service;

// Enable MouseMux (call when connection starts or user enables it)
let rustdesk_version = 142; // 1.4.2 as integer
if input_service::enable_mousemux(rustdesk_version) {
    log::info!("MouseMux enabled successfully");
} else {
    log::warn!("MouseMux not available or failed to enable");
}

// Disable MouseMux (call when connection ends or user disables it)
input_service::disable_mousemux();

// Check status
if input_service::is_mousemux_enabled() {
    log::info!("MouseMux is currently enabled");
}
```

### ⏳ Pending Components

1. **UI Integration (Sciter)**
   - ⚠️ Backend functions implemented (get/set_mousemux_enabled)
   - ❌ Checkbox UI element in Sciter TIS files not yet added
   - **Status:** Backend ready, just needs UI element added to settings

2. **Connection Lifecycle Integration**
   - ⚠️ Auto-enable on server startup implemented
   - ❌ Per-connection enable/disable not yet implemented
   - ❌ Automatic disable on connection end not yet implemented
   - **Status:** Global enable works, per-connection management needs work

### 🐛 Critical Bug Fix: Portable Service Process Registration

**Issue Discovered (October 8, 2025):**
- MouseMux was receiving ID 100 (default `ENIGO_INPUT_EXTRA_VALUE`) instead of registered IDs
- Only 2 SendInput calls were logged with correct MouseMux IDs during entire remote sessions
- All subsequent mouse/keyboard input showed ID 100 in MouseMux

**Root Cause Analysis:**
RustDesk uses a **two-process architecture** on Windows for input injection:

1. **Main Process** (`rustdesk.exe`):
   - Runs with normal user privileges
   - Handles UI, networking, video encoding
   - Spawns the portable service process
   - Receives input events from remote clients via network

2. **Portable Service Process** (`rustdesk.exe --portable-service`):
   - Separate elevated/SYSTEM process spawned by main process
   - Handles actual `SendInput()` calls for mouse/keyboard injection
   - Required for injecting input into elevated applications
   - Communicates with main process via IPC (Inter-Process Communication)

**The Bug:**
- MouseMux registration was in `start_portable_service()` (client module, line 550)
- This function runs in the **main process** which spawns the portable service
- But the **portable service process** runs `run_portable_service()` (server module, line 237)
- The portable service process was never calling `enable_mousemux()`
- Its ENIGO instance was using default ID 100

**Input Flow (Simplified):**
```
Remote Client → Network → Main Process → IPC → Portable Service → SendInput()
                                                     ↑
                                            This process wasn't registered!
```

**The Fix (Commit: 6da40d7ce):**
- Moved `enable_mousemux()` call from `start_portable_service()` to `run_portable_service()`
- Added logging: "Portable service process: MouseMux enabled successfully"
- Removed duplicate registration from main process (line 542-551 deleted)
- Added comprehensive SendInput logging to track dwExtraInfo values

**Files Modified:**
- `src/server/portable_service.rs`: Added registration at line 238-247
- `libs/enigo/src/win/win_impl.rs`: Added SendInput logging at lines 50, 86

**Result:**
- Portable service process now registers with MouseMux when it starts (before IPC client connects)
- Gets unique MouseMux ID (e.g., 6001)
- All SendInput calls use correct ID instead of default 100
- MouseMux can properly track and display multiple concurrent RustDesk connections

**Testing:**
- Log message: `Portable service process: MouseMux enabled successfully`
- Log message: `MouseMux: SendInput(MOUSE) called with dwExtraInfo=6001` (repeated for each input)
- MouseMux output: Shows 6001 instead of 100

### 📝 Implementation Notes for Future Work

#### Adding UI Checkbox (Sciter)

To add a checkbox to the Sciter UI:

1. **Location:** `src/ui/remote.tis` - Remote desktop toolbar/menu
2. **Add checkbox element:**
   ```tis
   <checkbox #mousemux-enabled>MouseMux enabled</checkbox>
   ```
3. **Add event handler:**
   ```tis
   self.select("#mousemux-enabled").on("click", function() {
       var enabled = this.value;
       view.set_mousemux_enabled(enabled);
   });
   ```
4. **Rust handler in `src/ui/remote.rs`:**
   ```rust
   pub fn set_mousemux_enabled(&mut self, enabled: bool) {
       #[cfg(windows)]
       {
           if enabled {
               crate::server::input_service::enable_mousemux(142);
           } else {
               crate::server::input_service::disable_mousemux();
           }
       }
   }
   ```

#### Connection Lifecycle Integration

In `src/server/connection.rs`, add to connection start:
```rust
#[cfg(windows)]
{
    // Check if MouseMux should be enabled (from config or default)
    if Config::get_option("enable_mousemux").is_empty() == false {
        crate::server::input_service::enable_mousemux(142);
    }
}
```

In `Connection::drop()` or connection cleanup:
```rust
#[cfg(windows)]
crate::server::input_service::disable_mousemux();
```

---

## MouseMux Integration Feature Specification

### Overview
Enable RustDesk host (Windows) to support multiple simultaneous client connections via MouseMux integration.

### Technical Requirements

#### 1. MouseMux Detection
- **Window to find:** `"mousemux.main.window.query"`
- **Method:** Win32 `FindWindow()` or `FindWindowA()`
- **Frequency:** Check on connection establishment and when user toggles MouseMux checkbox

#### 2. Version Handshake Protocol
**On Connection / MouseMux Enable:**
- Find window handle (HWND) of `"mousemux.main.window.query"`
- If found:
  - Call `SendMessage()` with:
    - HWND: Found window handle
    - Message: `WM_APP + 20` (typically 0x8000 + 20 = 0x8014)
    - wParam: RustDesk version as integer
    - lParam: 0 (unused)
    - Timeout: 5 seconds (use `SendMessageTimeout`)
  - Check return value:
    - If return value > 6000 AND < 6200: Store as `mousemux_input_id`
    - Else: Fall back to `ENIGO_INPUT_EXTRA_VALUE`
- If not found: Use `ENIGO_INPUT_EXTRA_VALUE`

#### 3. Input Injection Modification
**Current behavior:**
- Uses `ENIGO_INPUT_EXTRA_VALUE` in `SendInput()` calls

**New behavior:**
- If MouseMux enabled AND `mousemux_input_id` is valid (6000-6200):
  - Use `mousemux_input_id` instead of `ENIGO_INPUT_EXTRA_VALUE`
- Else:
  - Use `ENIGO_INPUT_EXTRA_VALUE`

**Applies to:**
- Keyboard injection (via SendInput with INPUT_KEYBOARD)
- Mouse injection (via SendInput with INPUT_MOUSE)

#### 4. Disconnection Protocol
**On Client Disconnect:**
- If MouseMux was active for this connection:
  - Send message to `"mousemux.main.window.query"`:
    - Message: `WM_APP + 24` (0x8018)
    - wParam: `mousemux_input_id` (the assigned ID)
    - lParam: 0
    - Use `SendMessageTimeout` with 5 second timeout

#### 5. UI Integration
**Add Checkbox:**
- Label: "MouseMux enabled"
- Location: In connection settings/options (accessible during active connection)
- Behavior:
  - Can be toggled on/off multiple times during connection
  - On toggle: Re-run MouseMux detection and handshake
  - State change takes effect immediately on next input event

### Implementation Areas

#### Files to Modify (Expected)
1. **Input injection (Windows):**
   - `libs/enigo/src/win/win_impl.rs` - SendInput calls
   - Look for `ENIGO_INPUT_EXTRA_VALUE` usage

2. **Connection management:**
   - `src/server/connection.rs` - Handle connection lifecycle
   - Need to track MouseMux state per connection

3. **UI (Sciter):**
   - `src/ui/` - Add MouseMux checkbox
   - Sciter TIS files for UI elements

4. **Windows-specific code:**
   - `src/platform/windows.rs` or similar - Win32 API calls

### State Management
```rust
struct MouseMuxState {
    enabled: bool,              // User checkbox state
    input_id: Option<u32>,      // Assigned ID (6000-6200) or None
    window_hwnd: Option<HWND>,  // Cached window handle
}
```

### Constants to Define
```rust
const MOUSEMUX_WINDOW_CLASS: &str = "mousemux.main.window.query";
const MOUSEMUX_MSG_REGISTER: u32 = WM_APP + 20;    // 0x8014
const MOUSEMUX_MSG_UNREGISTER: u32 = WM_APP + 24;  // 0x8018
const MOUSEMUX_ID_MIN: u32 = 6000;
const MOUSEMUX_ID_MAX: u32 = 6200;
const MOUSEMUX_TIMEOUT_MS: u32 = 5000;
```

### Error Handling
- If `SendMessageTimeout` fails: Fall back to `ENIGO_INPUT_EXTRA_VALUE`
- If window not found: Fall back to `ENIGO_INPUT_EXTRA_VALUE`
- If return value out of range: Fall back to `ENIGO_INPUT_EXTRA_VALUE`
- Log warnings for troubleshooting (don't fail connection)

### Testing Scenarios
1. MouseMux not running → Use `ENIGO_INPUT_EXTRA_VALUE`
2. MouseMux running, checkbox enabled → Use assigned ID
3. MouseMux running, checkbox disabled → Use `ENIGO_INPUT_EXTRA_VALUE`
4. Toggle checkbox during connection → Behavior changes immediately
5. Client disconnects → Unregister message sent to MouseMux

---

## Initial Environment
- **Location:** C:\RustDesk-build\rustdesk
- **RAM:** 2GB (insufficient)
- **Disk:** C: drive 60GB (100% full, only 50MB free)
- **Rust Version:** 1.90.0 (too new)
- **Git Branch:** master (latest, unstable)

## Build Issues and Solutions

### Issue 1: Insufficient Memory (2GB RAM)
**Problem:**
- Build failed with `memory allocation of 2097120 bytes failed`
- Error: `STATUS_STACK_BUFFER_OVERRUN` during `windows` crate compilation
- Cargo.toml release profile uses aggressive optimizations:
  - `lto = true` (Link-Time Optimization - very memory intensive)
  - `codegen-units = 1` (single codegen unit)

**Solution:**
- User upgraded system RAM from 2GB to 8GB
- System now has adequate memory for compilation

### Issue 2: Disk Space Exhausted
**Problem:**
- C: drive 100% full (60GB/60GB used, only 50MB free)
- Error: `"There is not enough space on the disk. (os error 112)"`
- Rust compilation needs 10-15GB free space for build artifacts

**Solution:**
- Copied entire project from `C:\RustDesk-build` to `O:\rustdesk-build`
- O: drive has 37GB available space (sufficient)

### Issue 3: VCPKG_ROOT Path Incorrect
**Problem:**
- After moving to O: drive, build failed with: `'opus/opus_multistream.h' file not found`
- VCPKG_ROOT still pointed to old location: `c:\RustDesk-build\vcpkg`
- magnum-opus build script cached the old path

**Solution:**
```bash
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
cd /o/rustdesk-build/rustdesk
cargo clean  # Clean cached build artifacts
```

**Important:** VCPKG_ROOT must be set in each new shell session, or add to ~/.bashrc for persistence

### Issue 4: Rust Version Incompatibility
**Problem:**
- RustDesk requires Rust 1.75.0 (specified in Cargo.toml line 9: `rust-version = "1.75"`)
- System had Rust 1.90.0 installed
- Newer Rust versions have breaking changes incompatible with RustDesk code
- Compilation errors: trait bound `EventToUI: IntoIntoDart<_>` not satisfied in src/flutter.rs:1412

**Solution:**
```bash
rustup install 1.75.0
cd /o/rustdesk-build/rustdesk
rustup override set 1.75.0  # Sets Rust 1.75.0 permanently for this directory
```

**Verification:**
```bash
cd /o/rustdesk-build/rustdesk
rustc --version  # Should show: rustc 1.75.0 (82e1608df 2023-12-21)
```

### Issue 5: Unstable Git Branch (master)
**Problem:**
- Initially on `master` branch (latest commit: d11011896)
- Even with correct Rust version (1.75.0), compilation failed
- Error: `the trait 'IntoIntoDart<_>' is not implemented for 'EventToUI'` in src/flutter.rs
- This is a code bug in the Flutter bridge implementation

**Attempted Solution 1 - Tag 1.4.2:**
```bash
git checkout 1.4.2  # Tag matching version in Cargo.toml
```
- Result: Same compilation error - tag also has broken Flutter bridge

**Attempted Solution 2 - Missing generated_bridge.dart:**
- Checked for `flutter/lib/generated_bridge.dart` - file doesn't exist
- File is not tracked in git (generated file)
- However, this turned out not to be the issue - RustDesk may have changed architecture

**Final Solution - Nightly Branch:**
```bash
cd /o/rustdesk-build/rustdesk
git checkout nightly  # Stable nightly build with latest fixes
# HEAD now at db4296533
```

**Reasoning:**
- Nightly branch has latest bug fixes
- More stable than master for active development
- Should have Flutter bridge fixes

## Current Build Configuration

### Environment Variables (Required for each build session)
```bash
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
```

### Project Location
```
O:\rustdesk-build\
├── rustdesk/          # Main project (git repo on 'nightly' branch)
│   └── sciter.dll     # Sciter UI library (8.0MB, downloaded)
└── vcpkg/             # C++ dependencies
```

### System Requirements
- **RAM:** 8GB minimum (we have 8GB)
- **Disk:** 15GB+ free space (we have 37GB on O:)
- **Rust:** 1.75.0 (set via rustup override)
- **Git Branch:** nightly

### Build Command - SCITER VERSION (Working)
```bash
cd /o/rustdesk-build/rustdesk
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
cargo build --release
```

The executable will be at: `target/release/rustdesk.exe`

### Build Command - FLUTTER VERSION (Currently Broken)
```bash
cd /o/rustdesk-build/rustdesk
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
python build.py --flutter
```

**Note:** Flutter version currently fails due to `EventToUI: IntoIntoDart<_>` trait implementation issue in src/flutter.rs:1412. This affects all branches (master, 1.4.2, nightly).

### Clean Build (if needed)
```bash
cd /o/rustdesk-build/rustdesk
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
cargo clean
cargo build --release  # For Sciter
```

### Issue 6: Flutter Version Unbuildable (All Branches)
**Problem:**
- Flutter version fails to compile on all branches (master, 1.4.2, nightly)
- Error in `src/flutter.rs:1412`: `the trait 'IntoIntoDart<_>' is not implemented for 'EventToUI'`
- Code tries to call: `stream.add(EventToUI::Event("close".to_owned()));`
- The `EventToUI` enum doesn't implement the required `IntoIntoDart` trait from `flutter_rust_bridge-1.80.1`
- This is a fundamental code bug, not a configuration issue

**Attempted Solutions:**
- ✗ Tried master branch - same error
- ✗ Tried tag 1.4.2 - same error
- ✗ Tried nightly branch - same error
- ✗ All use flutter_rust_bridge 1.80.1 which has this incompatibility

**Final Solution - Switch to Sciter:**
```bash
cd /o/rustdesk-build/rustdesk
# Download Sciter DLL
curl -L -o sciter.dll https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.win/x64/sciter.dll
# Build without flutter feature
cargo build --release
```

**Result:**
- Sciter version builds successfully (though deprecated by RustDesk)
- Executable output: `target/release/rustdesk.exe`
- Sciter is the legacy UI, but functional

## Build Timeline

1. **First attempt** (C: drive, 2GB RAM, Rust 1.90): Memory allocation failure
2. **After RAM upgrade** (C: drive, 8GB RAM, Rust 1.90): Disk space error
3. **After move to O: drive** (8GB RAM, Rust 1.90, wrong VCPKG_ROOT): opus header not found
4. **After VCPKG_ROOT fix** (8GB RAM, Rust 1.90, master branch): EventToUI trait error
5. **After Rust downgrade to 1.75** (8GB RAM, master branch): Same EventToUI trait error
6. **After git checkout 1.4.2** (8GB RAM, Rust 1.75): Same EventToUI trait error
7. **After git checkout nightly** (8GB RAM, Rust 1.75): Same EventToUI trait error (Flutter broken)
8. **Switch to Sciter** - Downloaded sciter.dll, built with `cargo build --release`
9. **Sciter build SUCCESS** - Compiled in 13m 25s, produced 27MB executable
10. **Installer created** - Built portable installer (11MB) using generate.py script

## Key Learnings

1. **RustDesk has strict Rust version requirements** - Always use exact version specified in Cargo.toml
2. **VCPKG_ROOT must be set correctly** - Especially important after moving project directories
3. **cargo clean is essential** - After changing paths or Rust versions to clear cached builds
4. **master branch may be unstable** - Use nightly or release tags for building
5. **Build requires significant resources** - 8GB RAM minimum, 15GB disk space minimum
6. **Build time is long** - 30-45 minutes for clean build on 2-core system
7. **Flutter version is currently broken** - Use Sciter (legacy) version instead with `cargo build --release`
8. **Sciter DLL required for Windows** - Must download sciter.dll and place in project root

## Troubleshooting Quick Reference

### If build fails with "file not found" errors:
```bash
echo $VCPKG_ROOT  # Verify correct path
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
cargo clean
```

### If build fails with trait/compilation errors:
```bash
cd /o/rustdesk-build/rustdesk
rustc --version  # Should be 1.75.0
rustup override set 1.75.0
git checkout nightly  # Try different branch
cargo clean
```

### If build runs out of memory:
- Check available RAM: `cat /proc/meminfo | grep MemTotal`
- Close other applications
- Consider building on a system with more RAM

### If build runs out of disk space:
- Check space: `df -h /o`
- Clean old builds: `cargo clean`
- Move to drive with more space

## Successful Build Process Summary

### Final Configuration That Worked
- **Location:** O:\rustdesk-build\rustdesk
- **RAM:** 8GB
- **Disk Space:** 37GB available on O: drive
- **Rust Version:** 1.75.0 (set via `rustup override set 1.75.0`)
- **Git Branch:** nightly
- **UI Framework:** Sciter (not Flutter)
- **VCPKG_ROOT:** /o/rustdesk-build/vcpkg

### Complete Build Steps (From Clean State)

#### 1. Environment Setup
```bash
# Set Rust version (permanent for this directory)
cd /o/rustdesk-build/rustdesk
rustup install 1.75.0
rustup override set 1.75.0

# Verify
rustc --version  # Should show 1.75.0

# Set VCPKG_ROOT (needed for each session)
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
```

#### 2. Download Sciter DLL
```bash
cd /o/rustdesk-build/rustdesk
curl -L -o sciter.dll https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.win/x64/sciter.dll
```
- Downloads 8.0MB sciter.dll to project root
- Required for Sciter UI to work

#### 3. Build RustDesk Executable
```bash
cd /o/rustdesk-build/rustdesk
export VCPKG_ROOT=/o/rustdesk-build/vcpkg
cargo build --release
```
- **Build time:** 13 minutes 25 seconds
- **Output:** target/release/rustdesk.exe (27MB)
- **Warnings:** 29 warnings (non-critical, mostly unused code)

#### 4. Create Portable Installer
```bash
# Prepare resources directory
mkdir -p resources
cp target/release/rustdesk.exe resources/RustDesk.exe
cp sciter.dll resources/

# Install Python dependencies
cd libs/portable
pip3 install -r requirements.txt  # Installs brotli

# Generate installer
python generate.py -f ../../resources -o . -e ../../resources/RustDesk.exe
```
- **Compression level:** 11 (highest)
- **Build time:** 51 seconds
- **Packages:** RustDesk.exe + sciter.dll into single installer

#### 5. Copy Final Installer
```bash
cd /o/rustdesk-build/rustdesk
cp target/release/rustdesk-portable-packer.exe rustdesk-1.4.2-x86_64-sciter.exe
```

### Build Outputs
1. **Executable:** `target/release/rustdesk.exe` (27MB)
   - Requires sciter.dll in same directory to run

2. **Portable Installer:** `rustdesk-1.4.2-x86_64-sciter.exe` (11MB)
   - Self-extracting installer
   - Contains both rustdesk.exe and sciter.dll compressed
   - Ready for distribution

### Critical Success Factors

1. **Correct Rust Version**
   - Must be 1.75.0 (newer versions fail)
   - Use `rustup override` to set permanently for directory

2. **VCPKG_ROOT Must Be Set**
   - Points to vcpkg installation
   - Must be set in every new shell session
   - Add to ~/.bashrc for permanence

3. **Sufficient Resources**
   - 8GB RAM minimum
   - 15GB+ free disk space
   - Clean build after moving directories or changing Rust versions

4. **Use Sciter, Not Flutter**
   - Flutter version has broken trait implementations
   - Sciter is deprecated but functional
   - No --flutter flag to build.py or cargo

5. **Nightly Branch**
   - More stable than master for building
   - Has latest dependency fixes
   - Tag 1.4.2 also has Flutter issues

### One-Line Rebuild Command
```bash
export VCPKG_ROOT=/o/rustdesk-build/vcpkg && cd /o/rustdesk-build/rustdesk && cargo clean && cargo build --release
```

### Time Requirements
- Clean build: ~13-15 minutes
- Installer generation: ~1 minute
- Total: ~15-20 minutes for complete build from scratch

---

## 📦 Patch Distribution Preparation (October 16, 2025)

**Status:** ✅ **COMPLETE - PATCHES READY FOR DISTRIBUTION**

### Session Overview

Generated complete patch set from the mousemux branch for distribution and application to fresh RustDesk forks.

### Patches Generated

**Location:** `C:\Users\Developer\Desktop\test\mousemux-patches-clean/`

**Count:** 49 patches (complete git history from base commit db4296533 to current mousemux branch HEAD)

**Status:** All Claude references removed from commit messages, ready for distribution

### What Was Done

1. **Generated Complete Patch Set**
   - Used `git format-patch db4296533..mousemux`
   - Generated all 49 commits as individual patch files
   - Patches numbered 0001 through 0049 in chronological order

2. **Cleaned Claude References**
   - Removed all `🤖 Generated with [Claude Code]` references
   - Removed all `Co-Authored-By: Claude` lines
   - Preserved all technical content and file changes
   - Verified no Claude references remain in commit messages

3. **Organized Patch Directory**
   - Moved obsolete patch directories to `attic/`:
     - `mousemux-patches/` (45 patches - incomplete)
     - `mousemux-patches-latest/` (1 patch - old)
     - `mousemux-v2.1-patches-clean/` (empty)
     - Loose patch files (3 files - duplicates)
   - Only `mousemux-patches-clean/` remains with current work

4. **Created Documentation**
   - `PATCH_CLEANING_COMPLETE.md` - Cleaning process summary
   - `PATCH_DIRECTORY_GUIDE.md` - Complete patch inventory and usage guide
   - Updated this CLAUDE.md with session information

### Verification

**Patch Count Match:**
```bash
# Git commits on mousemux branch from base
git log --oneline db4296533..mousemux | wc -l
# Output: 49

# Patches in mousemux-patches-clean/
ls mousemux-patches-clean/*.patch | wc -l
# Output: 49

# ✅ PERFECT MATCH - Complete coverage
```

**Claude References Removed:**
```bash
grep -E "(Co-Authored-By: Claude|Generated with.*Claude Code)" \
  mousemux-patches-clean/*.patch
# No results = Success! All references removed.
```

### How to Apply Patches

To apply all 49 patches to a fresh RustDesk fork:

```bash
# 1. Clone fresh RustDesk fork
git clone <your-rustdesk-fork-url>
cd rustdesk

# 2. Checkout base commit (nightly branch base)
git checkout db4296533

# 3. Apply all patches in order
git am C:/Users/Developer/Desktop/test/mousemux-patches-clean/*.patch

# 4. Verify all 49 commits applied
git log --oneline -50 | head -49

# 5. Create mousemux branch
git checkout -b mousemux
```

### Patch Content Summary

The 49 patches chronicle the complete MouseMux integration development:

- **0001-0011:** Initial MouseMux V1 implementation and documentation
- **0012-0019:** MouseMux V2 bidirectional async protocol
- **0020-0028:** MouseMux V2.1 per-connection ID system and IPC synchronization
- **0029-0038:** Window title tracking, protocol refinements, and additional features
- **0039-0049:** Recent fixes including:
  - UAC/update prompt removal for MouseMux Edition
  - Window title showing connected user count
  - Standalone service configuration fixes
  - Build script improvements
  - MouseMux branding and UI customization

### Files Modified Throughout All Patches

**Core Implementation:**
- `src/platform/windows_mousemux.rs` - New file, MouseMux V2.1 protocol
- `src/server/connection.rs` - Connection lifecycle integration
- `src/server/input_service.rs` - ID synchronization and IPC
- `src/server/portable_service.rs` - Portable service IPC handling
- `libs/enigo/src/win/win_impl.rs` - Per-connection ID lookup
- `src/ipc.rs` - MouseMux IPC message types

**Documentation:**
- `.claude/CLAUDE.md` - Complete build and implementation history
- `.claude/MOUSEMUX_V2.1_IMPLEMENTATION.md` - V2.1 technical specification
- `README.md` - MouseMux Edition branding

**Build & Configuration:**
- `build.py` - Build script improvements
- `libs/portable/generate.py` - Installer generation fixes
- `libs/hbb_common` - Submodule update for APP_NAME change

**UI & Branding:**
- UI files for MouseMux Edition customization
- Window title updates for user count display

### Next Steps

1. **Share Patches:** The `mousemux-patches-clean/` directory is ready for:
   - Distribution to team members
   - Application to fresh RustDesk forks
   - Submission as pull requests (if desired)
   - Archival for future reference

2. **Testing:** Once patches are applied to a fresh fork:
   - Build RustDesk with the patches
   - Test with MouseMux application
   - Verify multi-client scenarios
   - Test late-start scenario (MouseMux starts after clients connect)

3. **Known Issues to Debug:**
   - Late-start scenario may need additional investigation (from previous session)
   - Comprehensive logging added but not yet tested with actual MouseMux application

### Session Files

All session documentation located in:
```
C:\Users\Developer\Desktop\test\
├── mousemux-patches-clean/           (49 patches - USE THIS)
├── attic/                            (old patches, archived)
├── MOUSEMUX_V2.1_SESSION_2025-10-16.md  (33KB detailed session history)
├── PATCH_CLEANING_COMPLETE.md        (2KB cleaning summary)
└── PATCH_DIRECTORY_GUIDE.md          (5KB patch inventory)
```

### Git Branch Status

**Current Branch:** mousemux (49 commits ahead of base)
**Base Commit:** db4296533 (nightly branch)
**Branch Status:** Clean, no uncommitted changes

All work is committed and patches are generated. Ready for fresh fork application.

---

**Last Updated:** October 16, 2025
**Session Duration:** ~2 hours (patch generation, cleaning, and documentation)
**Status:** ✅ Complete - Ready for distribution and testing
