// MouseMux Protocol V2 - Windows Message Window Implementation
// Creates a message-only window to receive commands from MouseMux

use hbb_common::log;
use std::sync::{Arc, Mutex, mpsc};
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

// MouseMux Protocol V2 Messages
const WM_APP: u32 = 0x8000;
const WM_MOUSEMUX_STARTUP: u32 = WM_APP + 10;     // RustDesk → MouseMux: "I'm running"
const WM_MOUSEMUX_SHUTDOWN: u32 = WM_APP + 20;    // RustDesk → MouseMux: "I'm exiting"
const WM_MOUSEMUX_REQUEST_IDS: u32 = WM_APP + 30; // RustDesk → MouseMux: "Client connected, need IDs"
const WM_MOUSEMUX_RELEASE_IDS: u32 = WM_APP + 40; // RustDesk → MouseMux: "Client disconnected, release IDs"
const WM_MOUSEMUX_IDS: u32 = WM_APP + 100;        // MouseMux → RustDesk: ID assignment

// MouseMux window to find
const MOUSEMUX_WINDOW_CLASS: &str = "mousemux.main.window.query\0";

// Window class and title
const WINDOW_CLASS_NAME: &str = "rustdesk.mousemux.window.query\0";
const WINDOW_TITLE: &str = "rustdesk.mousemux.window.query\0";

/// Global state for MouseMux integration
pub struct MouseMuxState {
    pub hwnd: Option<HWND>,
    pub mouse_id: Option<u32>,
    pub keyboard_id: Option<u32>,
}

lazy_static::lazy_static! {
    static ref MOUSEMUX_STATE: Arc<Mutex<MouseMuxState>> = Arc::new(Mutex::new(MouseMuxState {
        hwnd: None,
        mouse_id: None,
        keyboard_id: None,
    }));

    // Channel to signal message loop thread to exit
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
        WM_MOUSEMUX_IDS => {
            // MouseMux is assigning us IDs
            let mouse_id = wparam as u32;
            let keyboard_id = lparam as u32;

            log::info!(
                "MouseMux V2: Received ID assignment - Mouse ID: {}, Keyboard ID: {}",
                mouse_id,
                keyboard_id
            );

            // Store IDs in global state
            if let Ok(mut state) = MOUSEMUX_STATE.lock() {
                state.mouse_id = Some(mouse_id);
                state.keyboard_id = Some(keyboard_id);
            }

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

        log::info!("MouseMux V2: Created message window HWND: {:?}", hwnd);
        Ok(hwnd)
    }
}

/// Message loop thread - runs GetMessage loop
fn message_loop_thread(hwnd: HWND) {
    log::info!("MouseMux V2: Starting message loop thread");

    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        // Standard Windows message loop
        while GetMessageA(&mut msg, hwnd, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }

    log::info!("MouseMux V2: Message loop thread exiting");
}

/// Initialize MouseMux message window and start background thread
pub fn init_mousemux_window() -> Result<(), String> {
    log::info!("MouseMux V2: Initializing message window");

    // Create the window
    let hwnd = create_message_window()?;

    // Store HWND in global state
    {
        let mut state = MOUSEMUX_STATE.lock().unwrap();
        state.hwnd = Some(hwnd);
    }

    // Start message loop in background thread
    let handle = thread::spawn(move || {
        message_loop_thread(hwnd);
    });

    // Store thread handle
    *MESSAGE_LOOP_HANDLE.lock().unwrap() = Some(handle);

    log::info!("MouseMux V2: Message window initialized successfully");
    Ok(())
}

/// Shutdown MouseMux message window
pub fn shutdown_mousemux_window() {
    log::info!("MouseMux V2: Shutting down message window");

    // Get HWND
    let hwnd = {
        let state = MOUSEMUX_STATE.lock().unwrap();
        state.hwnd
    };

    if let Some(hwnd) = hwnd {
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
        state.mouse_id = None;
        state.keyboard_id = None;
    }

    log::info!("MouseMux V2: Message window shut down");
}

/// Get RustDesk's message window HWND
pub fn get_rustdesk_hwnd() -> Option<HWND> {
    MOUSEMUX_STATE.lock().unwrap().hwnd
}

/// Get current mouse ID (if assigned)
pub fn get_mouse_id() -> Option<u32> {
    MOUSEMUX_STATE.lock().unwrap().mouse_id
}

/// Get current keyboard ID (if assigned)
pub fn get_keyboard_id() -> Option<u32> {
    MOUSEMUX_STATE.lock().unwrap().keyboard_id
}

/// Clear assigned IDs (called on client disconnect)
pub fn clear_ids() {
    let mut state = MOUSEMUX_STATE.lock().unwrap();
    state.mouse_id = None;
    state.keyboard_id = None;
    log::info!("MouseMux V2: Cleared assigned IDs");
}

/// Check if IDs are currently assigned
pub fn has_ids() -> bool {
    let state = MOUSEMUX_STATE.lock().unwrap();
    state.mouse_id.is_some() && state.keyboard_id.is_some()
}

// ============================================================================
// MouseMux Protocol V2 - Communication Functions
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
/// wParam: RustDesk version number
/// lParam: RustDesk's window HWND for verification
pub fn notify_startup(version: u32) -> bool {
    let rustdesk_hwnd = match get_rustdesk_hwnd() {
        Some(hwnd) => hwnd,
        None => {
            log::warn!("MouseMux V2: Cannot notify startup - RustDesk window not created yet");
            return false;
        }
    };

    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2: MouseMux window not found, not running");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_STARTUP,
            version as WPARAM,
            rustdesk_hwnd as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux V2: Failed to post startup message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux V2: Posted startup notification (version={}, hwnd={:?})",
                version,
                rustdesk_hwnd
            );
            true
        }
    }
}

/// Send WM_APP+20: RustDesk shutdown notification
pub fn notify_shutdown() -> bool {
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2: MouseMux window not found on shutdown");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(mousemux_hwnd, WM_MOUSEMUX_SHUTDOWN, 0, 0);

        if result == 0 {
            log::error!(
                "MouseMux V2: Failed to post shutdown message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!("MouseMux V2: Posted shutdown notification");
            true
        }
    }
}

/// Send WM_APP+30: Request ID assignment from MouseMux
/// Called when a client connects
pub fn request_ids() -> bool {
    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2: MouseMux window not found, cannot request IDs");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(mousemux_hwnd, WM_MOUSEMUX_REQUEST_IDS, 0, 0);

        if result == 0 {
            log::error!(
                "MouseMux V2: Failed to post request IDs message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!("MouseMux V2: Posted request for ID assignment");
            true
        }
    }
}

/// Send WM_APP+40: Release IDs back to MouseMux
/// Called when a client disconnects
/// wParam: Mouse ID
/// lParam: Keyboard ID
pub fn release_ids() -> bool {
    let (mouse_id, keyboard_id) = {
        let state = MOUSEMUX_STATE.lock().unwrap();
        match (state.mouse_id, state.keyboard_id) {
            (Some(m), Some(k)) => (m, k),
            _ => {
                log::warn!("MouseMux V2: No IDs to release");
                return false;
            }
        }
    };

    let mousemux_hwnd = match find_mousemux_window() {
        Some(hwnd) => hwnd,
        None => {
            log::info!("MouseMux V2: MouseMux window not found on release");
            return false;
        }
    };

    unsafe {
        let result = PostMessageA(
            mousemux_hwnd,
            WM_MOUSEMUX_RELEASE_IDS,
            mouse_id as WPARAM,
            keyboard_id as LPARAM,
        );

        if result == 0 {
            log::error!(
                "MouseMux V2: Failed to post release IDs message, error: {}",
                std::io::Error::last_os_error()
            );
            false
        } else {
            log::info!(
                "MouseMux V2: Posted release IDs (mouse={}, keyboard={})",
                mouse_id,
                keyboard_id
            );

            // Clear IDs after releasing
            clear_ids();
            true
        }
    }
}
