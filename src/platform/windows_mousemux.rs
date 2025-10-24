// MouseMux Protocol V2.2 - Windows Message Window Implementation
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
        GetMessageA, PostMessageA, PostQuitMessage, RegisterClassExA, SetWindowTextA, TranslateMessage,
        HWND_MESSAGE, MSG, WNDCLASSEXA, WS_OVERLAPPEDWINDOW, CS_HREDRAW, CS_VREDRAW,
    },
};

// MouseMux Protocol V2.2 Messages
const WM_APP: u32 = 0x8000;

// Messages RustDesk sends to MouseMux
const MOUSEMUX_NOTIFY_STARTUP: u32 = WM_APP + 10;        // Notify MouseMux that RustDesk is starting (version + HWND)
const MOUSEMUX_NOTIFY_SHUTDOWN: u32 = WM_APP + 20;       // Notify MouseMux that RustDesk is shutting down (version + HWND)
const MOUSEMUX_REQUEST_CONNECTION: u32 = WM_APP + 30;    // Request connection slot (conn_id + protocol_version)
const MOUSEMUX_SET_CONNECTION_NAME: u32 = WM_APP + 40;   // Send peer name character (conn_id + char_code)
const MOUSEMUX_REQUEST_IDS: u32 = WM_APP + 50;           // Request ID assignment after name sent (conn_id)
const MOUSEMUX_RELEASE_CONNECTION: u32 = WM_APP + 60;    // Release connection and IDs (conn_id)

// Messages MouseMux sends to RustDesk
const MOUSEMUX_STARTUP_BROADCAST: u32 = WM_APP + 100;    // MouseMux startup broadcast - re-register
const MOUSEMUX_MOUSE_ID_ASSIGNED: u32 = WM_APP + 110;    // Mouse ID assigned (conn_id + mouse_id)
const MOUSEMUX_KEYBOARD_ID_ASSIGNED: u32 = WM_APP + 120; // Keyboard ID assigned (conn_id + keyboard_id)
const MOUSEMUX_USER_ADD: u32 = WM_APP + 160;             // User added to MouseMux (user_id + total_count)
const MOUSEMUX_USER_REMOVE: u32 = WM_APP + 170;          // User removed from MouseMux (user_id + total_count)
const MOUSEMUX_REQUEST_EXIT: u32 = WM_APP + 200;         // MouseMux requests RustDesk to exit
const MOUSEMUX_EXITING: u32 = WM_APP + 210;              // MouseMux is exiting (reset user count)

// Protocol version and RustDesk version
const PROTOCOL_VERSION: u32 = 122;  // V2.2 = 122
const RUSTDESK_VERSION: u32 = 143;  // 1.4.3 = 143

// MouseMux window to find
const MOUSEMUX_WINDOW_CLASS: &str = "mousemux.main.window.query\0";

// Window class and title for RustDesk's receiver window
const WINDOW_CLASS_NAME: &str = "rustdesk.mousemux.window.query\0";
const WINDOW_TITLE: &str = "rustdesk.mousemux.window.query\0";

// Peer info max length
const MAX_PEER_INFO_LENGTH: usize = 256;

/// Wrapper for HWND that is Send + Sync safe
/// HWND is just a pointer to a window handle, safe to send between threads
#[derive(Clone, Copy, Debug)]
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

/// Global state for MouseMux integration (V2.2)
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
    static ref MAIN_WINDOW_HWND: Mutex<Option<SendSyncHwnd>> = Mutex::new(None);
    static ref CONNECTED_USERS_COUNT: Mutex<usize> = Mutex::new(0);
}

/// Window procedure callback - handles messages from MouseMux
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Log ALL Windows messages received (for debugging mode switching issues)
    if msg >= WM_APP {
        log::debug!(
            "MouseMux window_proc: Received Windows message 0x{:04X} (WM_APP+{}), wparam=0x{:X} ({}), lparam=0x{:X} ({})",
            msg,
            msg - WM_APP,
            wparam,
            wparam,
            lparam,
            lparam
        );
    }

    match msg {
        MOUSEMUX_STARTUP_BROADCAST => {  // WM_APP+100 - MouseMux startup broadcast
            log::info!("MouseMux v2.2 protocol: Received MOUSEMUX_STARTUP_BROADCAST - MouseMux is available");

            // Re-register RustDesk with MouseMux
            std::thread::spawn(|| {
                // Small delay to let MouseMux finish initialization
                std::thread::sleep(std::time::Duration::from_millis(100));
                notify_startup();

                // Re-request IDs for all active connections
                re_request_all_active_connections();
            });
            0
        }

        MOUSEMUX_MOUSE_ID_ASSIGNED => {  // WM_APP+110 - Mouse ID assigned
            let conn_id = wparam as i32;
            let mouse_id = lparam as u32;

            log::info!(
                "MouseMux v2.2 protocol: Received MOUSEMUX_MOUSE_ID_ASSIGNED - Mouse ID 0x{:X} ({}) for conn_id {}",
                mouse_id,
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

                log::info!(
                    "MouseMux v2.2 protocol: Stored mouse ID 0x{:X} in HashMap for conn_id {}",
                    mouse_id,
                    conn_id
                );
            }

            // Sync to Enigo
            log::info!(
                "MouseMux v2.2 protocol: Calling sync_mousemux_ids() for conn_id {} after receiving mouse ID",
                conn_id
            );
            crate::server::input_service::sync_mousemux_ids(conn_id);
            0
        }

        MOUSEMUX_KEYBOARD_ID_ASSIGNED => {  // WM_APP+120 - Keyboard ID assigned
            let conn_id = wparam as i32;
            let keyboard_id = lparam as u32;

            log::info!(
                "MouseMux v2.2 protocol: Received MOUSEMUX_KEYBOARD_ID_ASSIGNED - Keyboard ID 0x{:X} ({}) for conn_id {}",
                keyboard_id,
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

                log::info!(
                    "MouseMux v2.2 protocol: Stored keyboard ID 0x{:X} in HashMap for conn_id {}",
                    keyboard_id,
                    conn_id
                );
            }

            // Sync to Enigo
            log::info!(
                "MouseMux v2.2 protocol: Calling sync_mousemux_ids() for conn_id {} after receiving keyboard ID",
                conn_id
            );
            crate::server::input_service::sync_mousemux_ids(conn_id);
            0
        }

        MOUSEMUX_USER_ADD => {  // WM_APP+160 - User added to MouseMux
            let user_id = wparam as i32;
            let mousemux_total = lparam as usize;

            log::info!(
                "MouseMux v2.2 protocol: MOUSEMUX_USER_ADD - User {} added, MouseMux total: {}",
                user_id,
                mousemux_total
            );

            increment_connected_users();

            // Verify count matches MouseMux
            let our_count = get_connected_users_count();
            if our_count != mousemux_total {
                log::warn!(
                    "MouseMux v2.2 protocol: User count mismatch after ADD - RustDesk: {}, MouseMux: {}",
                    our_count,
                    mousemux_total
                );
            }
            0
        }

        MOUSEMUX_USER_REMOVE => {  // WM_APP+170 - User removed from MouseMux
            let user_id = wparam as i32;
            let mousemux_total = lparam as usize;

            log::info!(
                "MouseMux v2.2 protocol: MOUSEMUX_USER_REMOVE - User {} removed, MouseMux total: {}",
                user_id,
                mousemux_total
            );

            decrement_connected_users();

            // Verify count matches MouseMux
            let our_count = get_connected_users_count();
            if our_count != mousemux_total {
                log::warn!(
                    "MouseMux v2.2 protocol: User count mismatch after REMOVE - RustDesk: {}, MouseMux: {}",
                    our_count,
                    mousemux_total
                );
            }
            0
        }

        MOUSEMUX_EXITING => {  // WM_APP+210 - MouseMux is exiting
            log::info!("MouseMux v2.2 protocol: MOUSEMUX_EXITING - MouseMux is shutting down, resetting user count");

            // Reset user count to 0 as safety measure
            {
                let mut count = CONNECTED_USERS_COUNT.lock().unwrap();
                *count = 0;
            }
            update_main_window_title();
            0
        }

        MOUSEMUX_REQUEST_EXIT => {  // WM_APP+200 - MouseMux requests RustDesk to exit
            log::info!("MouseMux v2.2 protocol: Received MOUSEMUX_REQUEST_EXIT - Exiting RustDesk");

            // Exit the process
            // This will trigger cleanup handlers and gracefully shut down
            std::process::exit(0);
        }

        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

/// Create hidden top-level window for receiving MouseMux messages
/// NOTE: Not a message-only window (HWND_MESSAGE) because MouseMux needs to find it via FindWindowA
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

        // Create hidden top-level window (NOT message-only, so MouseMux can find it)
        // CRITICAL: MouseMux needs to find this window using FindWindowA, which cannot
        // locate message-only windows (HWND_MESSAGE parent). Therefore, we create a
        // normal hidden window (parent = NULL) that is findable but not visible.
        let window_title = WINDOW_TITLE.as_ptr() as *const i8;
        let hwnd = CreateWindowExA(
            0,                      // dwExStyle
            class_name,             // lpClassName
            window_title,           // lpWindowName
            0,                      // dwStyle (hidden window, no WS_VISIBLE)
            0, 0, 0, 0,             // x, y, width, height
            std::ptr::null_mut(),   // hWndParent (NULL = top-level, findable by FindWindowA)
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

        log::info!("MouseMux v2.2 protocol: Created message window HWND: {:?}", hwnd);
        Ok(hwnd)
    }
}

/// Message loop thread - runs GetMessage loop
fn message_loop_thread(_hwnd: HWND) {
    log::info!("MouseMux v2.2 protocol: Starting message loop thread");

    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        // Standard Windows message loop
        // IMPORTANT: Pass NULL (not hwnd) to receive ALL messages for this thread
        while GetMessageA(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }

    log::info!("MouseMux v2.2 protocol: Message loop thread exiting");
}

/// Initialize MouseMux message window and start background thread
pub fn init_mousemux_window() -> Result<(), String> {
    log::info!("MouseMux v2.2 protocol: Initializing message window");

    // CRITICAL: Window must be created on THE SAME THREAD that runs the message loop!
    // Windows delivers PostMessage to the thread that created the window.
    // Create window + run message loop in the same background thread.

    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel::<Result<(), String>>();

    let handle = thread::spawn(move || {
        // Create window on this thread
        let hwnd = match create_message_window() {
            Ok(h) => h,
            Err(e) => {
                tx.send(Err(e)).ok();
                return;
            }
        };

        log::info!("MouseMux v2.2 protocol: Window created on message loop thread: {:?}", hwnd);

        // Store HWND and signal success
        {
            let mut state = MOUSEMUX_STATE.lock().unwrap();
            state.hwnd = Some(SendSyncHwnd(hwnd));
        }
        tx.send(Ok(())).ok();

        // Run message loop on this same thread
        message_loop_thread(hwnd);
    });

    // Wait for window creation to complete
    match rx.recv() {
        Ok(Ok(())) => {
            let hwnd = MOUSEMUX_STATE.lock().unwrap().hwnd;
            log::info!("MouseMux v2.2 protocol: Message window initialized successfully: {:?}", hwnd);
        }
        Ok(Err(e)) => {
            return Err(e);
        }
        Err(_) => {
            return Err("Failed to receive window creation result".to_string());
        }
    }

    // Store thread handle
    *MESSAGE_LOOP_HANDLE.lock().unwrap() = Some(handle);

    Ok(())
}

/// Shutdown MouseMux message window
pub fn shutdown_mousemux_window() {
    log::info!("MouseMux v2.2 protocol: Shutting down message window");

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

    log::info!("MouseMux v2.2 protocol: Message window shut down");
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
        log::info!("MouseMux v2.2 protocol: Cleared IDs for conn_id {}", conn_id);
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

/// Set the main Sciter window HWND (call this from UI initialization)
pub fn set_main_window_hwnd(hwnd: HWND) {
    *MAIN_WINDOW_HWND.lock().unwrap() = Some(SendSyncHwnd(hwnd));
    log::info!("MouseMux: Main window HWND set to {:?}", hwnd);
}

/// Update the main window title (no user count displayed here anymore)
fn update_main_window_title() {
    let hwnd = *MAIN_WINDOW_HWND.lock().unwrap();

    if let Some(SendSyncHwnd(hwnd)) = hwnd {
        unsafe {
            let title = CString::new("RustDesk (MouseMux compliant edition)").unwrap();
            SetWindowTextA(hwnd, title.as_ptr());
            log::info!("MouseMux: Window title set to default (user count displayed in UI only)");
        }
    }
}

/// Increment connected users count and update window title
pub fn increment_connected_users() {
    let mut count = CONNECTED_USERS_COUNT.lock().unwrap();
    *count += 1;
    drop(count);  // Release lock before updating title
    update_main_window_title();
}

/// Decrement connected users count and update window title
pub fn decrement_connected_users() {
    let mut count = CONNECTED_USERS_COUNT.lock().unwrap();
    if *count > 0 {
        *count -= 1;
    }
    drop(count);  // Release lock before updating title
    update_main_window_title();
}

// ============================================================================
// MouseMux Protocol V2.2 - Communication Functions
// ============================================================================

/// Log outgoing Windows message
fn log_outgoing_message(msg_name: &str, msg_id: u32, hwnd: HWND, wparam: WPARAM, lparam: LPARAM, success: bool) {
    if success {
        log::info!(
            "MouseMux SEND: {} (0x{:04X} / WM_APP+{}) to {:?}, wparam=0x{:X} ({}), lparam=0x{:X} ({}) - SUCCESS",
            msg_name,
            msg_id,
            msg_id - WM_APP,
            hwnd,
            wparam,
            wparam,
            lparam,
            lparam
        );
    } else {
        log::error!(
            "MouseMux SEND: {} (0x{:04X} / WM_APP+{}) to {:?}, wparam=0x{:X} ({}), lparam=0x{:X} ({}) - FAILED: {}",
            msg_name,
            msg_id,
            msg_id - WM_APP,
            hwnd,
            wparam,
            wparam,
            lparam,
            lparam,
            std::io::Error::last_os_error()
        );
    }
}

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

/// Send MOUSEMUX_NOTIFY_STARTUP (WM_APP+10): RustDesk startup notification
/// wParam: RustDesk version (142)
/// lParam: RustDesk's window HWND for callbacks
pub fn notify_startup() -> bool {
    let rustdesk_hwnd = match get_rustdesk_hwnd() {
        Some(hwnd) => hwnd,
        None => {
            log::warn!("MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_STARTUP - Cannot notify, RustDesk window not created yet");
            return false;
        }
    };

    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_STARTUP - MouseMux window not found");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            MOUSEMUX_NOTIFY_STARTUP,
            RUSTDESK_VERSION as WPARAM,
            rustdesk_hwnd as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_STARTUP - Failed to post message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_STARTUP - Posted to MouseMux window {:?} (version={}, our_hwnd={:?})",
                mousemux_hwnd,
                RUSTDESK_VERSION,
                rustdesk_hwnd
            );
            true
        }
    }
}

/// Send MOUSEMUX_NOTIFY_SHUTDOWN (WM_APP+20): RustDesk shutdown notification
/// wParam: RustDesk version (142)
/// lParam: RustDesk's window HWND
pub fn notify_shutdown() -> bool {
    let rustdesk_hwnd = match get_rustdesk_hwnd() {
        Some(hwnd) => hwnd,
        None => {
            log::warn!("MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_SHUTDOWN - No RustDesk window");
            return false;
        }
    };

    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_SHUTDOWN - MouseMux window not found");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            MOUSEMUX_NOTIFY_SHUTDOWN,
            RUSTDESK_VERSION as WPARAM,
            rustdesk_hwnd as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_SHUTDOWN - Failed to post message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux v2.2 protocol: MOUSEMUX_NOTIFY_SHUTDOWN - Posted to MouseMux window {:?} (version={}, our_hwnd={:?})",
                mousemux_hwnd,
                RUSTDESK_VERSION,
                rustdesk_hwnd
            );
            true
        }
    }
}

/// Request ID assignment from MouseMux for a new connection
/// Sends MOUSEMUX_REQUEST_CONNECTION, MOUSEMUX_SET_CONNECTION_NAME (char-by-char), and MOUSEMUX_REQUEST_IDS
/// Called when a client connects
///
/// conn_id: RustDesk's internal connection ID
/// peer_info: Peer identification string (format: "{name}@{id}")
pub fn request_ids(conn_id: i32, peer_info: &str) -> bool {
    // Truncate peer_info to max length
    let peer_info = if peer_info.len() > MAX_PEER_INFO_LENGTH {
        &peer_info[..MAX_PEER_INFO_LENGTH]
    } else {
        peer_info
    };

    // CRITICAL: Store connection in HashMap FIRST, before checking if MouseMux is running
    // This ensures the connection is tracked even if MouseMux isn't running yet
    // When MouseMux starts later and sends WM_APP+100, re_request_all_active_connections()
    // will find this connection in the HashMap and request IDs for it
    {
        let mut state = MOUSEMUX_STATE.lock().unwrap();
        state.pending_peer_info.insert(conn_id, peer_info.to_string());

        // Create or update connection entry with peer_info
        state.connections
            .entry(conn_id)
            .or_insert(MouseMuxConnectionIDs {
                conn_id,
                peer_info: peer_info.to_string(),
                mouse_id: None,
                keyboard_id: None,
            })
            .peer_info = peer_info.to_string();  // Update peer_info if entry already exists

        log::info!("MouseMux v2.2 protocol: Registered connection {} with peer_info '{}' in HashMap", conn_id, peer_info);
    }

    // Now check if MouseMux is running
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux v2.2 protocol: MouseMux window not found, connection {} tracked for later", conn_id);
            return false;  // Connection is tracked, but can't send messages yet
        }
    };

    unsafe {
        // 1. Send MOUSEMUX_REQUEST_CONNECTION (WM_APP+30): Connection start
        let result = PostMessageA(
            mousemux_hwnd,
            MOUSEMUX_REQUEST_CONNECTION,
            conn_id as WPARAM,
            PROTOCOL_VERSION as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux v2.2 protocol: MOUSEMUX_REQUEST_CONNECTION - Failed to post for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            return false;
        }

        log::info!(
            "MouseMux v2.2 protocol: MOUSEMUX_REQUEST_CONNECTION - Posted to MouseMux window {:?} for conn_id {} (protocol={})",
            mousemux_hwnd,
            conn_id,
            PROTOCOL_VERSION
        );

        // 2. Send MOUSEMUX_SET_CONNECTION_NAME (WM_APP+40): Peer info character-by-character
        for ch in peer_info.chars() {
            let result = PostMessageA(
                mousemux_hwnd,
                MOUSEMUX_SET_CONNECTION_NAME,
                conn_id as WPARAM,
                ch as u32 as LPARAM,
            );

            if result == 0 {
                log::error!(
                    "MouseMux v2.2 protocol: MOUSEMUX_SET_CONNECTION_NAME - Failed to post char for conn_id {}, error: {}",
                    conn_id,
                    std::io::Error::last_os_error()
                );
                return false;
            }
        }

        // 3. Send null terminator
        let result = PostMessageA(
            mousemux_hwnd,
            MOUSEMUX_SET_CONNECTION_NAME,
            conn_id as WPARAM,
            0,  // null terminator
        );

        if result == 0 {
            log::error!(
                "MouseMux v2.2 protocol: MOUSEMUX_SET_CONNECTION_NAME - Failed to post null for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            return false;
        }

        log::info!(
            "MouseMux v2.2 protocol: MOUSEMUX_SET_CONNECTION_NAME - Posted peer info '{}' for conn_id {} ({} chars + null)",
            peer_info,
            conn_id,
            peer_info.len()
        );

        // 4. Send MOUSEMUX_REQUEST_IDS (WM_APP+50): Trigger ID generation
        // Get RustDesk's HWND to pass to MouseMux so it knows where to send the response
        let rustdesk_hwnd = match get_rustdesk_hwnd() {
            Some(hwnd) => hwnd as LPARAM,
            None => {
                log::error!("MouseMux v2.2 protocol: MOUSEMUX_REQUEST_IDS - No RustDesk window for conn_id {}", conn_id);
                return false;
            }
        };

        let result = PostMessageA(
            mousemux_hwnd,
            MOUSEMUX_REQUEST_IDS,
            conn_id as WPARAM,
            rustdesk_hwnd,  // Pass RustDesk's HWND so MouseMux knows where to send IDs
        );

        if result == 0 {
            log::error!(
                "MouseMux v2.2 protocol: MOUSEMUX_REQUEST_IDS - Failed to post for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            return false;
        }

        log::info!(
            "MouseMux v2.2 protocol: MOUSEMUX_REQUEST_IDS - Posted to MouseMux window {:?} for conn_id {} (rustdesk_hwnd={:?})",
            mousemux_hwnd,
            conn_id,
            rustdesk_hwnd
        );

        true
    }
}

/// Send MOUSEMUX_RELEASE_CONNECTION (WM_APP+60): Release IDs back to MouseMux
/// Called when a client disconnects
///
/// conn_id: RustDesk's internal connection ID
pub fn release_ids(conn_id: i32) -> bool {
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux v2.2 protocol: MOUSEMUX_RELEASE_CONNECTION - MouseMux window not found for conn_id {}", conn_id);
            return false;
        }
    };

    // Check if we have IDs to release
    let has_ids = {
        let state = MOUSEMUX_STATE.lock().unwrap();
        state.connections.contains_key(&conn_id)
    };

    if !has_ids {
        log::warn!("MouseMux v2.2 protocol: MOUSEMUX_RELEASE_CONNECTION - No IDs for conn_id {}", conn_id);
        return false;
    }

    // CRITICAL: Reset IDs to 100 BEFORE sending disconnect message
    // This prevents latent SendInput calls from using invalid IDs after disconnect
    log::info!(
        "MouseMux v2.2 protocol: MOUSEMUX_RELEASE_CONNECTION - Resetting IDs to 100 for conn_id {} before sending disconnect",
        conn_id
    );

    // Clear IDs from local state FIRST
    clear_ids_for_connection(conn_id);

    // Also remove from pending_peer_info
    MOUSEMUX_STATE.lock().unwrap().pending_peer_info.remove(&conn_id);

    // NOW send the disconnect message to MouseMux
    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            MOUSEMUX_RELEASE_CONNECTION,
            conn_id as WPARAM,
            0,  // lParam unused in V2.2
        );

        if result == 0 {
            log::error!(
                "MouseMux v2.2 protocol: MOUSEMUX_RELEASE_CONNECTION - Failed to post for conn_id {}, error: {}",
                conn_id,
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux v2.2 protocol: MOUSEMUX_RELEASE_CONNECTION - Posted to MouseMux window {:?} for conn_id {}",
                mousemux_hwnd,
                conn_id
            );
            true
        }
    }
}

/// Re-request IDs for all active connections
/// Called when MouseMux restarts (RUSTDESK_SELF_START received)
fn re_request_all_active_connections() {
    log::info!("MouseMux v2.2 protocol: Re-requesting IDs for all active connections");
    
    // Get list of connections that have peer_info
    let connections_to_reregister: Vec<(i32, String)> = {
        let state = MOUSEMUX_STATE.lock().unwrap();
        state.connections
            .iter()
            .map(|(conn_id, conn)| (*conn_id, conn.peer_info.clone()))
            .collect()
    };
    
    if connections_to_reregister.is_empty() {
        log::info!("MouseMux v2.2 protocol: No active connections to re-register");
        return;
    }
    
    log::info!(
        "MouseMux v2.2 protocol: Re-registering {} active connection(s)",
        connections_to_reregister.len()
    );
    
    // Re-request IDs for each connection
    for (conn_id, peer_info) in connections_to_reregister {
        log::info!(
            "MouseMux v2.2 protocol: Re-requesting IDs for conn_id {} (peer: {})",
            conn_id,
            peer_info
        );
        request_ids(conn_id, &peer_info);
    }
}

/// Get the current number of connected users for display in UI
pub fn get_connected_users_count() -> usize {
    *CONNECTED_USERS_COUNT.lock().unwrap()
}
