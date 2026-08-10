// ============================================================================
// MouseMux compile-time debug logging (Finding 13)
//
// These fire on EVERY keystroke and EVERY mouse event. The October 2025
// keyboard-ID investigation added them at info! level directly in the input hot
// path and they were never dialled back, putting synchronous log I/O on every
// input event - the same path Finding 5's serialization lock now runs through.
//
// Gated on a cargo feature rather than debug_assertions ON PURPOSE: the problems
// that need this tracing (per-connection ID assignment, MouseMux handshake
// timing) only reproduce in release builds, so a debug-only gate would remove
// the logging exactly when it is next needed.
//
//   normal release build:   cargo build --features flutter --lib --release
//   with MouseMux tracing:  cargo build --features flutter,mousemux-debug --lib --release
//
// Errors, warnings, and low-frequency lifecycle events (ID assignment,
// connection register/release, protocol handshake) are deliberately NOT gated -
// they are rare and are what you need to diagnose a field report.
// ============================================================================

#[cfg(feature = "mousemux-debug")]
#[macro_export]
macro_rules! mm_debug {
    ($($arg:tt)*) => { hbb_common::log::info!($($arg)*) };
}

#[cfg(not(feature = "mousemux-debug"))]
#[macro_export]
macro_rules! mm_debug {
    ($($arg:tt)*) => {};
}

mod keyboard;
/// cbindgen:ignore
pub mod platform;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use platform::{
    clip_cursor, get_cursor, get_cursor_data, get_cursor_pos, get_focused_display,
    set_cursor_pos, start_os_service,
};
#[cfg(not(any(target_os = "ios")))]
/// cbindgen:ignore
mod server;
#[cfg(not(any(target_os = "ios")))]
pub use self::server::*;
mod client;
mod lan;
#[cfg(not(any(target_os = "ios")))]
mod rendezvous_mediator;
#[cfg(not(any(target_os = "ios")))]
pub use self::rendezvous_mediator::*;
/// cbindgen:ignore
pub mod common;
#[cfg(not(any(target_os = "ios")))]
pub mod ipc;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    feature = "flutter"
)))]
pub mod ui;
mod version;
pub use version::*;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
mod bridge_generated;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter_ffi;
use common::*;
mod auth_2fa;
#[cfg(not(target_os = "ios"))]
mod clipboard;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod core_main;
mod custom_server;
mod lang;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod port_forward;

#[cfg(all(feature = "flutter", feature = "plugin_framework"))]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod plugin;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod tray;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod whiteboard;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod updater;

mod ui_cm_interface;
mod ui_interface;
mod ui_session_interface;

mod hbbs_http;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub mod clipboard_file;

pub mod privacy_mode;

#[cfg(windows)]
pub mod virtual_display_manager;

mod kcp_stream;
