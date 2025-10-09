// MouseMux Protocol V2.1 - Windows Message Window Implementation
// Per-connection ID assignment for multi-user collaboration

use hbb_common::log;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::ffi::CString;
use winapi::{
    shared::{
        minwindef::{LPARAM, LRESULT, UINT, WPARAM},
        windef::HWND,
    },
    um::winuser::{
        CreateWindowExA, DefWindowProcA, DestroyWindow, DispatchMessageA, FindWindowA,
        GetMessageA, PostMessageA, PostQuitMessage, RegisterClassExA, TranslateMessage,
        HWND_MESSAGE, MSG, WNDCLASSEXA, WS_OVERLAPPEDWINDOW, CS_HREDRAW, CS_VREDRAW,
    },
};

// MouseMux Protocol V2.1 Messages
const WM_APP: u32 = 0x8000;
const WM_MOUSEMUX_STARTUP: u32 = WM_APP + 10;        // RustDesk → MouseMux: Startup (version + HWND)
const WM_MOUSEMUX_SHUTDOWN: u32 = WM_APP + 20;       // RustDesk → MouseMux: Shutdown (version + HWND)
const WM_MOUSEMUX_CONN_START: u32 = WM_APP + 30;     // RustDesk → MouseMux: Client connects (conn_id + protocol_version)
const WM_MOUSEMUX_PEER_INFO_CHAR: u32 = WM_APP + 32; // RustDesk → MouseMux: Peer info character (conn_id + char_code)
const WM_MOUSEMUX_PEER_INFO_DONE: u32 = WM_APP + 34; // RustDesk → MouseMux: Peer info complete (conn_id)
const WM_MOUSEMUX_CONN_END: u32 = WM_APP + 40;       // RustDesk → MouseMux: Client disconnects (conn_id)
const WM_MOUSEMUX_MOUSE_ID: u32 = WM_APP + 100;      // MouseMux → RustDesk: Mouse ID assigned (conn_id + mouse_id)
const WM_MOUSEMUX_KEYBOARD_ID: u32 = WM_APP + 110;   // MouseMux → RustDesk: Keyboard ID assigned (conn_id + keyboard_id)

// Protocol version and RustDesk version
const PROTOCOL_VERSION: u32 = 121;  // V2.1 = 121
const RUSTDESK_VERSION: u32 = 142;  // 1.4.2 = 142

// MouseMux window to find
const MOUSEMUX_WINDOW_CLASS: &str = "mousemux.main.window.query\0";

// Window class and title for RustDesk's receiver window
const WINDOW_CLASS_NAME: &str = "rustdesk.mousemux.window.query\0";
const WINDOW_TITLE: &str = "rustdesk.mousemux.window.query\0";

// Peer info max length
const MAX_PEER_INFO_LENGTH: usize = 256;

/// Wrapper for HWND that is Send + Sync safe
/// HWND is just a pointer to a window handle, safe to send between threads
#[derive(Clone, Copy)]
struct SendSyncHwnd(HWND);
unsafe impl Send for SendSyncHwnd {}
unsafe impl Sync for SendSyncHwnd {}

/// Per-connection MouseMux ID assignment
#[derive(Clone, Debug)]
pub struct MouseMuxConnectionIDs {
    pub conn_id: i32,
    pub peer_info: String,
    pub mouse_id: Option<u32>,
    pub keyboard_id: Option<u32>,
}

/// Global state for MouseMux integration (V2.1)
pub struct MouseMuxState {
    pub hwnd: Option<SendSyncHwnd>,  // RustDesk's message window handle
    pub connections: HashMap<i32, MouseMuxConnectionIDs>,  // conn_id → IDs
    pub pending_peer_info: HashMap<i32, String>,  // Temporary storage while receiving WM_APP+32
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

lazy_static::lazy_static! {
    static ref MOUSEMUX_STATE: Arc<Mutex<MouseMuxState>> = Arc::new(Mutex::new(MouseMuxState::new()));
    static ref MESSAGE_LOOP_HANDLE: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
}

/// Window procedure callback - handles messages from MouseMux
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

            log::info!(
                "MouseMux V2.1: Received mouse ID {} for conn_id {}",
                mouse_id,
                conn_id
            );

            // Store mouse ID in connection state
            if let Ok(mut state) = MOUSEMUX_STATE.lock() {
                // Get peer_info before mutable borrow
                let peer_info = state.pending_peer_info.get(&conn_id).cloned().unwrap_or_default();
                let entry = state.connections
                    .entry(conn_id)
                    .or_insert(MouseMuxConnectionIDs {
                        conn_id,
                        peer_info,
                        mouse_id: None,
                        keyboard_id: None,
                    });
                entry.mouse_id = Some(mouse_id);
            }

            // Sync to Enigo
            crate::server::input_service::sync_mousemux_ids(conn_id);
            0
        }

        WM_MOUSEMUX_KEYBOARD_ID => {  // WM_APP+110
            let conn_id = wparam as i32;
            let keyboard_id = lparam as u32;

            log::info!(
                "MouseMux V2.1: Received keyboard ID {} for conn_id {}",
                keyboard_id,
                conn_id
            );

            // Store keyboard ID in connection state
            if let Ok(mut state) = MOUSEMUX_STATE.lock() {
                // Get peer_info before mutable borrow
                let peer_info = state.pending_peer_info.get(&conn_id).cloned().unwrap_or_default();
                let entry = state.connections
                    .entry(conn_id)
                    .or_insert(MouseMuxConnectionIDs {
                        conn_id,
                        peer_info,
                        mouse_id: None,
                        keyboard_id: None,
                    });
                entry.keyboard_id = Some(keyboard_id);
            }

            // Sync to Enigo
            crate::server::input_service::sync_mousemux_ids(conn_id);
            0
        }

        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

/// Create message-only window for receiving MouseMux messages
fn create_message_window() -> Result<HWND, String> {
    unsafe {
        // Register window class
        let class_name = WINDOW_CLASS_NAME.as_ptr() as *const i8;

        let wnd_class = WNDCLASSEXA {
            cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: std::ptr::null_mut(),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name,
            hIconSm: std::ptr::null_mut(),
        };

        let atom = RegisterClassExA(&wnd_class);
        if atom == 0 {
            return Err(format!(
                "Failed to register window class, error: {}",
                std::io::Error::last_os_error()
            ));
        }

        // Create message-only window (parent = HWND_MESSAGE)
        let window_title = WINDOW_TITLE.as_ptr() as *const i8;
        let hwnd = CreateWindowExA(
            0,                      // dwExStyle
            class_name,             // lpClassName
            window_title,           // lpWindowName
            WS_OVERLAPPEDWINDOW,    // dwStyle
            0, 0, 0, 0,             // x, y, width, height (ignored for message-only)
            HWND_MESSAGE,           // hWndParent (message-only window)
            std::ptr::null_mut(),   // hMenu
            std::ptr::null_mut(),   // hInstance
            std::ptr::null_mut(),   // lpParam
        );

        if hwnd.is_null() {
            return Err(format!(
                "Failed to create window, error: {}",
                std::io::Error::last_os_error()
            ));
        }

        log::info!("MouseMux V2.1: Created message window HWND: {:?}", hwnd);
        Ok(hwnd)
    }
}

/// Message loop thread - runs GetMessage loop
fn message_loop_thread(hwnd: HWND) {
    log::info!("MouseMux V2.1: Starting message loop thread");

    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        // Standard Windows message loop
        while GetMessageA(&mut msg, hwnd, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }

    log::info!("MouseMux V2.1: Message loop thread exiting");
}

/// Initialize MouseMux message window and start background thread
pub fn init_mousemux_window() -> Result<(), String> {
    log::info!("MouseMux V2.1: Initializing message window");

    // Create the window
    let hwnd = create_message_window()?;

    // Store HWND in global state
    {
        let mut state = MOUSEMUX_STATE.lock().unwrap();
        state.hwnd = Some(SendSyncHwnd(hwnd));
    }

    // Start message loop in background thread
    let hwnd_raw = hwnd as usize;
    let handle = thread::spawn(move || {
        let hwnd = hwnd_raw as HWND;
        message_loop_thread(hwnd);
    });

    // Store thread handle
    *MESSAGE_LOOP_HANDLE.lock().unwrap() = Some(handle);

    log::info!("MouseMux V2.1: Message window initialized successfully");
    Ok(())
}

/// Shutdown MouseMux message window
pub fn shutdown_mousemux_window() {
    log::info!("MouseMux V2.1: Shutting down message window");

    // Get HWND
    let hwnd = {
        let state = MOUSEMUX_STATE.lock().unwrap();
        state.hwnd
    };

    if let Some(SendSyncHwnd(hwnd)) = hwnd {
        unsafe {
            // Post WM_QUIT to message loop
            PostQuitMessage(0);

            // Destroy window
            DestroyWindow(hwnd);
        }
    }

    // Wait for thread to exit
    if let Some(handle) = MESSAGE_LOOP_HANDLE.lock().unwrap().take() {
        handle.join().ok();
    }

    // Clear state
    {
        let mut state = MOUSEMUX_STATE.lock().unwrap();
        state.hwnd = None;
        state.connections.clear();
        state.pending_peer_info.clear();
    }

    log::info!("MouseMux V2.1: Message window shut down");
}

/// Get RustDesk's message window HWND
pub fn get_rustdesk_hwnd() -> Option<HWND> {
    MOUSEMUX_STATE.lock().unwrap().hwnd.map(|SendSyncHwnd(h)| h)
}

/// Get IDs for a specific connection
pub fn get_ids_for_connection(conn_id: i32) -> Option<(u32, u32)> {
    let state = MOUSEMUX_STATE.lock().unwrap();
    state.connections.get(&conn_id).and_then(|conn| {
        match (conn.mouse_id, conn.keyboard_id) {
            (Some(m), Some(k)) => Some((m, k)),
            _ => None,
        }
    })
}

/// Clear IDs for a specific connection
pub fn clear_ids_for_connection(conn_id: i32) {
    let mut state = MOUSEMUX_STATE.lock().unwrap();
    if state.connections.remove(&conn_id).is_some() {
        log::info!("MouseMux V2.1: Cleared IDs for conn_id {}", conn_id);
        drop(state); // Release lock before syncing

        // Sync cleared state to Enigo
        crate::server::input_service::sync_mousemux_ids(conn_id);
    }
}

/// Check if a connection has IDs assigned
pub fn has_ids_for_connection(conn_id: i32) -> bool {
    let state = MOUSEMUX_STATE.lock().unwrap();
    state.connections.get(&conn_id)
        .map(|conn| conn.mouse_id.is_some() && conn.keyboard_id.is_some())
        .unwrap_or(false)
}

/// Check if ANY connection currently has IDs assigned (for UI status)
pub fn has_ids() -> bool {
    let state = MOUSEMUX_STATE.lock().unwrap();
    state.connections.iter().any(|(_, conn)| {
        conn.mouse_id.is_some() && conn.keyboard_id.is_some()
    })
}

// ============================================================================
// MouseMux Protocol V2.1 - Communication Functions
// ============================================================================

/// Find MouseMux window
fn find_mousemux_window() -> Option<HWND> {
    unsafe {
        let class_name = CString::new(MOUSEMUX_WINDOW_CLASS.trim_end_matches('\0')).ok()?;
        let hwnd = FindWindowA(class_name.as_ptr(), std::ptr::null());

        if hwnd.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }
}

/// Send WM_APP+10: RustDesk startup notification
/// wParam: RustDesk version (142)
/// lParam: RustDesk's window HWND for callbacks
pub fn notify_startup() -> bool {
    let rustdesk_hwnd = match get_rustdesk_hwnd() {
        Some(hwnd) => hwnd,
        None => {
            log::warn!("MouseMux V2.1: Cannot notify startup - RustDesk window not created yet");
            return false;
        }
    };

    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2.1: MouseMux window not found, not running");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_STARTUP,
            RUSTDESK_VERSION as WPARAM,
            rustdesk_hwnd as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux V2.1: Failed to post startup message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux V2.1: Posted startup notification (version={}, hwnd={:?})",
                RUSTDESK_VERSION,
                rustdesk_hwnd
            );
            true
        }
    }
}

/// Send WM_APP+20: RustDesk shutdown notification
/// wParam: RustDesk version (142) - consistent with WM_APP+10
/// lParam: RustDesk's window HWND - consistent with WM_APP+10
pub fn notify_shutdown() -> bool {
    let rustdesk_hwnd = match get_rustdesk_hwnd() {
        Some(hwnd) => hwnd,
        None => {
            log::warn!("MouseMux V2.1: No RustDesk window on shutdown");
            return false;
        }
    };

    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2.1: MouseMux window not found on shutdown");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_SHUTDOWN,
            RUSTDESK_VERSION as WPARAM,
            rustdesk_hwnd as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux V2.1: Failed to post shutdown message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux V2.1: Posted shutdown notification (version={}, hwnd={:?})",
                RUSTDESK_VERSION,
                rustdesk_hwnd
            );
            true
        }
    }
}

/// Send WM_APP+30/32/34: Request ID assignment from MouseMux
/// Called when a client connects
///
/// conn_id: RustDesk's internal connection ID
/// peer_info: Peer identification string (format: "{name}@{id}")
pub fn request_ids(conn_id: i32, peer_info: &str) -> bool {
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2.1: MouseMux window not found, cannot request IDs for conn_id {}", conn_id);
            return false;
        }
    };

    // Truncate peer_info to max length
    let peer_info = if peer_info.len() > MAX_PEER_INFO_LENGTH {
        &peer_info[..MAX_PEER_INFO_LENGTH]
    } else {
        peer_info
    };

    // Store peer_info in pending state
    {
        let mut state = MOUSEMUX_STATE.lock().unwrap();
        state.pending_peer_info.insert(conn_id, peer_info.to_string());
    }

    unsafe {
        // 1. Send WM_APP+30: Connection start
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_CONN_START,
            conn_id as WPARAM,
            PROTOCOL_VERSION as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux V2.1: Failed to post connection start for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            return false;
        }

        log::info!(
            "MouseMux V2.1: Posted connection start for conn_id {} (protocol={})",
            conn_id,
            PROTOCOL_VERSION
        );

        // 2. Send WM_APP+32: Peer info character-by-character
        for ch in peer_info.chars() {
            let result = PostMessageA(
                mousemux_hwnd,
                WM_MOUSEMUX_PEER_INFO_CHAR,
                conn_id as WPARAM,
                ch as u32 as LPARAM,
            );

            if result == 0 {
                log::error!(
                    "MouseMux V2.1: Failed to post peer info char for conn_id {}, error: {}",
                    conn_id,
                    std::io::Error::last_os_error()
                );
                return false;
            }
        }

        // 3. Send null terminator
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_PEER_INFO_CHAR,
            conn_id as WPARAM,
            0,  // null terminator
        );

        if result == 0 {
            log::error!(
                "MouseMux V2.1: Failed to post null terminator for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            return false;
        }

        log::info!(
            "MouseMux V2.1: Posted peer info '{}' for conn_id {} ({} chars + null)",
            peer_info,
            conn_id,
            peer_info.len()
        );

        // 4. Send WM_APP+34: Trigger ID generation
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_PEER_INFO_DONE,
            conn_id as WPARAM,
            0,
        );

        if result == 0 {
            log::error!(
                "MouseMux V2.1: Failed to post peer info done for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            return false;
        }

        log::info!(
            "MouseMux V2.1: Posted request for ID generation for conn_id {}",
            conn_id
        );

        true
    }
}

/// Send WM_APP+40: Release IDs back to MouseMux
/// Called when a client disconnects
///
/// conn_id: RustDesk's internal connection ID
pub fn release_ids(conn_id: i32) -> bool {
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2.1: MouseMux window not found on release for conn_id {}", conn_id);
            return false;
        }
    };

    // Check if we have IDs to release
    let has_ids = {
        let state = MOUSEMUX_STATE.lock().unwrap();
        state.connections.contains_key(&conn_id)
    };

    if !has_ids {
        log::warn!("MouseMux V2.1: No IDs to release for conn_id {}", conn_id);
        return false;
    }

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_CONN_END,
            conn_id as WPARAM,
            0,  // lParam unused in V2.1
        );

        if result == 0 {
            log::error!(
                "MouseMux V2.1: Failed to post release IDs for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux V2.1: Posted release IDs for conn_id {}",
                conn_id
            );

            // Clear IDs from local state
            clear_ids_for_connection(conn_id);

            // Also remove from pending_peer_info
            MOUSEMUX_STATE.lock().unwrap().pending_peer_info.remove(&conn_id);

            true
        }
    }
}
