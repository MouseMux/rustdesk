// MouseMux Protocol V2.2 - Windows Message Window Implementation
// Per-connection ID assignment for multi-user collaboration

use hbb_common::log;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::ffi::CString;
use winapi::{
    shared::{
        minwindef::{LPARAM, LRESULT, UINT, WPARAM},
        windef::HWND,
    },
    um::winuser::{
        CreateWindowExA, DefWindowProcA, DispatchMessageA, FindWindowExA,
        GetMessageA, PostMessageA, PostQuitMessage, RegisterClassExA, SetWindowTextA, TranslateMessage,
        MSG, WNDCLASSEXA, CS_HREDRAW, CS_VREDRAW, WM_CLOSE, WM_DESTROY,
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
// Reported to MouseMux in NOTIFY_STARTUP/NOTIFY_SHUTDOWN. MouseMux validates this
// as a RANGE (VERS_MIN=100, VERS_MAX=999 in rustdesk_validation.c), not an exact
// match, so a stale value still connects - it just misreports which RustDesk this
// is in MouseMux's own logs and UI. Keep it in step with the upstream base version.
const RUSTDESK_VERSION: u32 = 149;  // 1.4.9 = 149

// MouseMux window classes to look for, in priority order.
//
// MouseMux versions its window class names. The C side builds them as
// VAPI_STRING_PROGRAM_S ".main.window.query", where VAPI_STRING_PROGRAM_S is the
// versioned program name - so MouseMux V3 registers "mousemux-v3.main.window.query",
// NOT the unversioned name this code originally hardcoded.
//
// Established empirically on 2026-08-10: mousemux-common.h defines
// MOUSEMUX_VERSIONED_NAME_DAEMON_WINDOW_32 as VAPI_STRING_PROGRAM_S ".daemon.window.32",
// and the live window on a machine running MouseMux V3 3.0.10 has class
// "mousemux-v3.daemon.window.32". Hence VAPI_STRING_PROGRAM_S == "mousemux-v3".
//
// Both v2 and v3 components can be installed side by side (a mousemux-v2-service
// process was running alongside v3), so try newest first and fall back rather than
// swapping one hardcoded name for another.
//
// NOTE: the RustDesk-side window is deliberately NOT versioned - the C header
// declares it as the shared constant "rustdesk.mousemux.window.query".
const MOUSEMUX_WINDOW_CLASSES: &[&str] = &[
    "mousemux-v3.main.window.query",
    "mousemux-v2.main.window.query",
    "mousemux.main.window.query", // legacy / unversioned
];

// RustDesk's receiver window(s).
//
// Naming standard: <slug>.<component>.<kind>[.<qualifier>] - all lowercase, dots as
// the only separator, hyphens only inside a token, slug always first so that
// "mousemux-v3.*" selects everything belonging to one MouseMux version.
//
// PRIMARY conforms to that standard. LEGACY is the historic name, which put the
// component first and carried no version. It is still registered because MouseMux
// discovers RustDesk BY NAME in three places - rustdesk_receiver.c:122,
// rustdesk_state.c:157, rustdesk_validation.c:245 - all hardcoded to the legacy
// constant MOUSEMUX_SHARED_NAME_RUSTDESK_WINDOW. Registering only the new name
// would leave the currently shipping MouseMux unable to find RustDesk at all.
//
// Both windows route to the same window_proc, so MouseMux may address either. The
// PRIMARY handle is what NOTIFY_STARTUP reports, so MouseMux ends up holding the
// new one either way. Delete LEGACY once those three C call sites are versioned.
//
// Class and title are deliberately identical: MouseMux searches with FindWindowEx
// passing both, and a class-only search does not match these windows.
const WINDOW_CLASS_PRIMARY: &str = "mousemux-v3.rustdesk.window.query\0";
const WINDOW_CLASS_LEGACY: &str = "rustdesk.mousemux.window.query\0";

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
}

impl MouseMuxState {
    pub fn new() -> Self {
        Self {
            hwnd: None,
            connections: HashMap::new(),
        }
    }
}

lazy_static::lazy_static! {
    static ref MOUSEMUX_STATE: Arc<Mutex<MouseMuxState>> = Arc::new(Mutex::new(MouseMuxState::new()));
    static ref MESSAGE_LOOP_HANDLE: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
    static ref MAIN_WINDOW_HWND: Mutex<Option<SendSyncHwnd>> = Mutex::new(None);
    static ref CONNECTED_USERS_COUNT: Mutex<usize> = Mutex::new(0);
}

lazy_static::lazy_static! {
    /// Which MouseMux window class we last matched, so find_mousemux_window() can log
    /// only on transitions rather than on every protocol send.
    static ref LAST_FOUND_CLASS: Mutex<Option<String>> = Mutex::new(None);
}

/// Guards against duplicate re-registration threads (Finding 11).
static REREGISTER_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

// Finding 12 (class registration idempotency) is now handled per class inside
// create_one_window: a window class outlives the windows created from it, so after
// shutdown_mousemux_window() the class is still registered and RegisterClassExA
// returns ERROR_CLASS_ALREADY_EXISTS (1410). That specific error is treated as
// success rather than failing initialisation. No global flag is needed, and a flag
// would in fact be wrong now that there is more than one class.

// ---------------------------------------------------------------------------
// Poison-tolerant lock helpers (Finding 9)
//
// These were `.lock().unwrap()` throughout, which panics if the mutex is poisoned
// - i.e. if any thread ever panicked while holding it. That mattered because
// window_proc is a Windows callback: unwinding a panic out of an `extern "system"`
// function across the FFI boundary is undefined behaviour, so one unrelated panic
// could turn every subsequent MouseMux message into UB.
//
// Recovering the guard is strictly safer than propagating here. The protected data
// is a connection map, a window handle and a counter; a partially-applied update to
// those is benign, and the alternative is aborting the process or invoking UB.
// ---------------------------------------------------------------------------

#[inline]
fn lock_state() -> std::sync::MutexGuard<'static, MouseMuxState> {
    MOUSEMUX_STATE.lock().unwrap_or_else(|e| e.into_inner())
}

#[inline]
fn lock_users_count() -> std::sync::MutexGuard<'static, usize> {
    CONNECTED_USERS_COUNT.lock().unwrap_or_else(|e| e.into_inner())
}

#[inline]
fn lock_main_hwnd() -> std::sync::MutexGuard<'static, Option<SendSyncHwnd>> {
    MAIN_WINDOW_HWND.lock().unwrap_or_else(|e| e.into_inner())
}

#[inline]
fn lock_loop_handle() -> std::sync::MutexGuard<'static, Option<thread::JoinHandle<()>>> {
    MESSAGE_LOOP_HANDLE.lock().unwrap_or_else(|e| e.into_inner())
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

            // Finding 11: this used to spawn a thread unconditionally. MouseMux
            // broadcasts to every listener, and nothing stops it broadcasting
            // repeatedly (restart loop, multiple instances), so each one spawned
            // another sleeping thread that then re-registered every connection.
            // Collapse concurrent broadcasts into a single in-flight re-registration.
            if REREGISTER_IN_FLIGHT.swap(true, Ordering::SeqCst) {
                log::info!("MouseMux v2.2 protocol: Re-registration already in flight, ignoring duplicate broadcast");
                return 0;
            }

            std::thread::spawn(|| {
                // Small delay to let MouseMux finish initialization
                std::thread::sleep(std::time::Duration::from_millis(100));
                notify_startup();

                // Re-request IDs for all active connections
                re_request_all_active_connections();

                REREGISTER_IN_FLIGHT.store(false, Ordering::SeqCst);
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
            {
                // lock_state() recovers from poisoning; the previous
                // `if let Ok(..)` silently skipped ID assignment instead.
                let mut state = lock_state();
                // Get peer_info before mutable borrow
                // request_ids() always inserts the connection (with its peer_info)
                // before the IDs can come back, so this or_insert is only a guard
                // against a disconnect racing the reply - in which case an empty
                // name is correct anyway.
                let peer_info = String::new();
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
            {
                // lock_state() recovers from poisoning; the previous
                // `if let Ok(..)` silently skipped ID assignment instead.
                let mut state = lock_state();
                // Get peer_info before mutable borrow
                // request_ids() always inserts the connection (with its peer_info)
                // before the IDs can come back, so this or_insert is only a guard
                // against a disconnect racing the reply - in which case an empty
                // name is correct anyway.
                let peer_info = String::new();
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

            // MouseMux owns the real user table; we only mirror it. Previously a
            // mismatch was merely logged, so a single missed or duplicated message
            // left our counter permanently wrong with no way to recover. Reconcile
            // to the authoritative value MouseMux just sent.
            let our_count = get_connected_users_count();
            if our_count != mousemux_total {
                log::warn!(
                    "MouseMux v2.2 protocol: User count mismatch after ADD - RustDesk: {}, MouseMux: {} - reconciling to MouseMux",
                    our_count,
                    mousemux_total
                );
                set_connected_users_count(mousemux_total);
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

            // See MOUSEMUX_USER_ADD: MouseMux is authoritative, so reconcile rather
            // than just warn.
            let our_count = get_connected_users_count();
            if our_count != mousemux_total {
                log::warn!(
                    "MouseMux v2.2 protocol: User count mismatch after REMOVE - RustDesk: {}, MouseMux: {} - reconciling to MouseMux",
                    our_count,
                    mousemux_total
                );
                set_connected_users_count(mousemux_total);
            }
            0
        }

        MOUSEMUX_EXITING => {  // WM_APP+210 - MouseMux is exiting
            log::info!("MouseMux v2.2 protocol: MOUSEMUX_EXITING - MouseMux is shutting down, resetting user count");

            // Reset user count to 0 as safety measure
            {
                let mut count = lock_users_count();
                *count = 0;
            }
            update_main_window_title();
            0
        }

        MOUSEMUX_REQUEST_EXIT => {  // WM_APP+200 - MouseMux requests RustDesk to exit
            log::info!("MouseMux v2.2 protocol: Received MOUSEMUX_REQUEST_EXIT - Exiting RustDesk");

            // Finding 8: this used to call std::process::exit(0) directly here, with
            // a comment claiming it "will trigger cleanup handlers and gracefully
            // shut down". It does neither - process::exit runs no destructors - and
            // it exited from inside a window procedure, so MouseMux was never told
            // RustDesk had gone and its connection slot leaked until it noticed.
            //
            // Do the courtesy notify off this thread, then exit. It must not run
            // inline: we are ON the message-loop thread, and notify_shutdown() posts
            // to MouseMux and expects the loop to keep pumping.
            std::thread::spawn(|| {
                notify_shutdown();
                log::info!("MouseMux v2.2 protocol: Shutdown notified, exiting process");
                std::process::exit(0);
            });
            0
        }

        WM_DESTROY => {
            // Runs on the message-loop thread, which is the only thread allowed to
            // end that loop. shutdown_mousemux_window() posts WM_CLOSE from another
            // thread; DefWindowProcA turns that into DestroyWindow, landing here.
            log::info!("MouseMux v2.2 protocol: WM_DESTROY - posting WM_QUIT to end message loop");
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

/// Create hidden top-level window for receiving MouseMux messages
/// NOTE: Not a message-only window (HWND_MESSAGE) because MouseMux needs to find it
/// via FindWindowEx. Class and title are deliberately the same string - MouseMux
/// searches with both, and a class-only search does not match these windows.
fn create_message_window() -> Result<HWND, String> {
    // The primary window's HWND is the one reported to MouseMux in NOTIFY_STARTUP.
    let primary = create_one_window(WINDOW_CLASS_PRIMARY, true)?;
    // Best-effort: a failure here only costs compatibility with older MouseMux,
    // so log it rather than failing initialisation outright.
    if let Err(e) = create_one_window(WINDOW_CLASS_LEGACY, false) {
        log::warn!("MouseMux v2.2 protocol: legacy alias window not created: {}", e);
    }
    Ok(primary)
}

/// Register (once) and create one receiver window for the given class string.
/// `class` must be NUL-terminated; class and title are the same string.
fn create_one_window(class: &str, is_primary: bool) -> Result<HWND, String> {
    unsafe {
        let class_name = class.as_ptr() as *const i8;

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

        // Finding 12: register the class only once per process. A window class
        // outlives the windows created from it, so after shutdown_mousemux_window()
        // the class is still registered and a second RegisterClassExA fails with
        // ERROR_CLASS_ALREADY_EXISTS (1410) - which made re-initialisation fail
        // outright rather than reusing the perfectly good existing class.
        let atom = RegisterClassExA(&wnd_class);
        if atom == 0 {
            const ERROR_CLASS_ALREADY_EXISTS: i32 = 1410;
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(ERROR_CLASS_ALREADY_EXISTS) {
                return Err(format!("Failed to register window class '{}', error: {}",
                                   class.trim_end_matches('\0'), err));
            }
            // Already registered by an earlier init in this process - reuse it.
        }

        // Create hidden top-level window (NOT message-only, so MouseMux can find it)
        // CRITICAL: MouseMux needs to find this window using FindWindowEx, which cannot
        // locate message-only windows (HWND_MESSAGE parent). Therefore, we create a
        // normal hidden window (parent = NULL) that is findable but not visible.
        let window_title = class_name; // title == class, see the note on the constants
        let hwnd = CreateWindowExA(
            0,                      // dwExStyle
            class_name,             // lpClassName
            window_title,           // lpWindowName
            0,                      // dwStyle (hidden window, no WS_VISIBLE)
            0, 0, 0, 0,             // x, y, width, height
            std::ptr::null_mut(),   // hWndParent (NULL = top-level, findable by FindWindowEx)
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

        log::info!(
            "MouseMux v2.2 protocol: Created {} receiver window '{}' HWND: {:?}",
            if is_primary { "primary" } else { "legacy-alias" },
            class.trim_end_matches('\0'),
            hwnd
        );
        Ok(hwnd)
    }
}

/// Message loop thread - runs GetMessage loop
fn message_loop_thread() {
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
            let mut state = lock_state();
            state.hwnd = Some(SendSyncHwnd(hwnd));
        }
        tx.send(Ok(())).ok();

        // Run message loop on this same thread
        message_loop_thread();
    });

    // Wait for window creation to complete
    match rx.recv() {
        Ok(Ok(())) => {
            let hwnd = lock_state().hwnd;
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
    *lock_loop_handle() = Some(handle);

    Ok(())
}

/// Shutdown MouseMux message window
pub fn shutdown_mousemux_window() {
    log::info!("MouseMux v2.2 protocol: Shutting down message window");

    // Get HWND
    let hwnd = {
        let state = lock_state();
        state.hwnd
    };

    if let Some(SendSyncHwnd(hwnd)) = hwnd {
        unsafe {
            // Must be PostMessage, not PostQuitMessage/DestroyWindow: this runs on a
            // different thread than the message loop. PostQuitMessage would queue
            // WM_QUIT to *this* thread, and Windows refuses DestroyWindow on a window
            // owned by another thread - so the loop would never exit and the join()
            // below would block forever. WM_CLOSE routes through DefWindowProcA, which
            // destroys the window and delivers WM_DESTROY on the loop thread.
            PostMessageA(hwnd, WM_CLOSE, 0, 0);
        }
    }

    // Wait for thread to exit
    if let Some(handle) = lock_loop_handle().take() {
        handle.join().ok();
    }

    // Clear state
    {
        let mut state = lock_state();
        state.hwnd = None;
        state.connections.clear();
    }

    log::info!("MouseMux v2.2 protocol: Message window shut down");
}

/// Get RustDesk's message window HWND
pub fn get_rustdesk_hwnd() -> Option<HWND> {
    lock_state().hwnd.map(|SendSyncHwnd(h)| h)
}

/// Get IDs for a specific connection
pub fn get_ids_for_connection(conn_id: i32) -> Option<(u32, u32)> {
    let state = lock_state();
    state.connections.get(&conn_id).and_then(|conn| {
        match (conn.mouse_id, conn.keyboard_id) {
            (Some(m), Some(k)) => Some((m, k)),
            _ => None,
        }
    })
}

/// Clear IDs for a specific connection
pub fn clear_ids_for_connection(conn_id: i32) {
    let mut state = lock_state();
    if state.connections.remove(&conn_id).is_some() {
        log::info!("MouseMux v2.2 protocol: Cleared IDs for conn_id {}", conn_id);
        drop(state); // Release lock before syncing

        // Sync cleared state to Enigo
        crate::server::input_service::sync_mousemux_ids(conn_id);
    }
}

/// Check if a connection has IDs assigned
pub fn has_ids_for_connection(conn_id: i32) -> bool {
    let state = lock_state();
    state.connections.get(&conn_id)
        .map(|conn| conn.mouse_id.is_some() && conn.keyboard_id.is_some())
        .unwrap_or(false)
}

/// Check if ANY connection currently has IDs assigned (for UI status)
pub fn has_ids() -> bool {
    let state = lock_state();
    state.connections.iter().any(|(_, conn)| {
        conn.mouse_id.is_some() && conn.keyboard_id.is_some()
    })
}

/// Set the main Sciter window HWND (call this from UI initialization)
pub fn set_main_window_hwnd(hwnd: HWND) {
    *lock_main_hwnd() = Some(SendSyncHwnd(hwnd));
    log::info!("MouseMux: Main window HWND set to {:?}", hwnd);
}

/// Update the main window title (no user count displayed here anymore)
fn update_main_window_title() {
    let hwnd = *lock_main_hwnd();

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
    let mut count = lock_users_count();
    *count += 1;
    drop(count);  // Release lock before updating title
    update_main_window_title();
}

/// Decrement connected users count and update window title
pub fn decrement_connected_users() {
    let mut count = lock_users_count();
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
        for class in MOUSEMUX_WINDOW_CLASSES {
            let Ok(class_name) = CString::new(*class) else {
                continue;
            };
            // MUST pass the window TITLE as well as the class.
            //
            // Searching by class alone returns NULL for these windows - verified on a
            // live system for BOTH MouseMux's window and our own:
            //     FindWindowEx(NULL, NULL, "mousemux-v3.main.window.query", NULL)  -> 0
            //     FindWindowEx(NULL, NULL, "mousemux-v3.main.window.query",
            //                              "mousemux-v3.main.window.query")        -> found
            // The previous code called FindWindowA(class, NULL), so it could never
            // locate MouseMux regardless of which class name it tried.
            //
            // MouseMux's own C code does the same thing (rustdesk_receiver.c):
            //     FindWindowEx(NULL, NULL, MOUSEMUX_VERSIONED_NAME_MAIN_WINDOW_QUERY,
            //                              MOUSEMUX_VERSIONED_NAME_MAIN_WINDOW_QUERY)
            // Both sides name the window identically to its class, so the same string
            // is passed twice.
            let hwnd = FindWindowExA(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                class_name.as_ptr(),
                class_name.as_ptr(),
            );
            if !hwnd.is_null() {
                // Only log on a change, otherwise this fires on every protocol send.
                let mut last = LAST_FOUND_CLASS.lock().unwrap_or_else(|e| e.into_inner());
                if last.as_deref() != Some(*class) {
                    log::info!(
                        "MouseMux v2.2 protocol: found MouseMux window, class '{}' hwnd {:?}",
                        class,
                        hwnd
                    );
                    *last = Some((*class).to_string());
                }
                return Some(hwnd);
            }
        }

        // Not found under any known class. Log once per transition to avoid spamming;
        // MouseMux simply not running is a normal, supported state.
        let mut last = LAST_FOUND_CLASS.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some() {
            log::info!(
                "MouseMux v2.2 protocol: MouseMux window no longer found (tried {:?})",
                MOUSEMUX_WINDOW_CLASSES
            );
            *last = None;
        }
        None
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
    // Truncate to MAX_PEER_INFO_LENGTH *characters*, not bytes.
    // Byte-slicing a &str at a non-char-boundary panics, and peer names are
    // attacker-supplied; the protocol also sends one message per character,
    // so characters are the correct unit here.
    let peer_info: String = peer_info.chars().take(MAX_PEER_INFO_LENGTH).collect();

    // CRITICAL: Store connection in HashMap FIRST, before checking if MouseMux is running
    // This ensures the connection is tracked even if MouseMux isn't running yet
    // When MouseMux starts later and sends WM_APP+100, re_request_all_active_connections()
    // will find this connection in the HashMap and request IDs for it
    {
        let mut state = lock_state();
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

        // 2. Send MOUSEMUX_SET_CONNECTION_NAME (WM_APP+40): peer info as UTF-32
        //    little-endian, FOUR messages per character, low byte first.
        //
        //    MouseMux accumulates four messages into one code point
        //    (rustdesk_handler.c: `utf32 |= (byte & 0xFF) << (bytes * 8)`), and
        //    separately validates every value against 0..=127
        //    (rustdesk_validation.c: NAME_CHAR_MAX). Those two rules only agree for
        //    ASCII, so non-ASCII is replaced with '?' rather than being rejected by
        //    the validator and dropped.
        for ch in peer_info.chars() {
            let cp = if ch.is_ascii() { ch as u32 } else { b'?' as u32 };

            for shift in 0..4 {
                let byte = (cp >> (shift * 8)) & 0xFF;

                let result = PostMessageA(
                    mousemux_hwnd,
                    MOUSEMUX_SET_CONNECTION_NAME,
                    conn_id as WPARAM,
                    byte as LPARAM,
                );

                if result == 0 {
                    log::error!(
                        "MouseMux v2.2 protocol: MOUSEMUX_SET_CONNECTION_NAME - Failed to post char byte {} for conn_id {}, error: {}",
                        shift,
                        conn_id,
                        std::io::Error::last_os_error()
                    );
                    return false;
                }
            }
        }

        // 3. Send null terminator: four zero bytes, so MouseMux assembles UTF-32 0
        //    and runs its end-of-name conversion. A single zero would only fill one
        //    byte of the accumulator and would never terminate the name.
        for shift in 0..4 {
            let result = PostMessageA(
                mousemux_hwnd,
                MOUSEMUX_SET_CONNECTION_NAME,
                conn_id as WPARAM,
                0,
            );

            if result == 0 {
                log::error!(
                    "MouseMux v2.2 protocol: MOUSEMUX_SET_CONNECTION_NAME - Failed to post null byte {} for conn_id {}, error: {}",
                    shift,
                    conn_id,
                    std::io::Error::last_os_error()
                );
                return false;
            }
        }

        log::info!(
            "MouseMux v2.2 protocol: MOUSEMUX_SET_CONNECTION_NAME - Posted peer info '{}' for conn_id {} ({} chars + null)",
            peer_info,
            conn_id,
            peer_info.chars().count()
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
    // Check if we have IDs to release
    let has_ids = {
        let state = lock_state();
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

    // Clear IDs from local state FIRST. This must happen even when MouseMux is not
    // running - otherwise the dead connection stays in the map forever, gets
    // re-registered by re_request_all_active_connections() on the next MouseMux
    // start, and keeps has_ids() reporting a stale connection to the UI.
    clear_ids_for_connection(conn_id);

    // Local state is now clean; notify MouseMux only if it is actually running.
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!(
                "MouseMux v2.2 protocol: MOUSEMUX_RELEASE_CONNECTION - MouseMux window not found for conn_id {}, local state cleared",
                conn_id
            );
            return false;
        }
    };

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
        let state = lock_state();
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
    *lock_users_count()
}

/// Force the user count to MouseMux's authoritative value.
///
/// Used to reconcile after a USER_ADD/USER_REMOVE mismatch: MouseMux owns the real
/// user table, so its number wins rather than our incrementally-maintained one.
fn set_connected_users_count(count: usize) {
    *lock_users_count() = count;
    update_main_window_title();
}
