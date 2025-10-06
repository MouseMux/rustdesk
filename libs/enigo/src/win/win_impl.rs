use self::winapi::ctypes::c_int;
use self::winapi::shared::{basetsd::{ULONG_PTR, DWORD_PTR}, minwindef::*, windef::*};
use self::winapi::um::winbase::*;
use self::winapi::um::winuser::*;
use winapi;

use crate::win::keycodes::*;
use crate::{Key, KeyboardControllable, MouseButton, MouseControllable};
use std::mem::*;

extern "system" {
    pub fn GetLastError() -> DWORD;
}

/// The main struct for handling the event emitting
#[derive(Default)]
pub struct Enigo {
    mousemux_input_id: Option<ULONG_PTR>,
}

static mut LAYOUT: HKL = std::ptr::null_mut();

/// The dwExtraInfo value in keyboard and mouse structure that used in SendInput()
pub const ENIGO_INPUT_EXTRA_VALUE: ULONG_PTR = 100;

// MouseMux integration constants
const MOUSEMUX_WINDOW_CLASS: &str = "mousemux.main.window.query";
const MOUSEMUX_MSG_REGISTER: u32 = WM_APP + 20;    // 0x8014
const MOUSEMUX_MSG_UNREGISTER: u32 = WM_APP + 24;  // 0x8018
const MOUSEMUX_ID_MIN: ULONG_PTR = 6000;
const MOUSEMUX_ID_MAX: ULONG_PTR = 6200;
const MOUSEMUX_TIMEOUT_MS: u32 = 5000;

fn mouse_event(flags: u32, data: u32, dx: i32, dy: i32, extra_info: ULONG_PTR) -> DWORD {
    let mut u = INPUT_u::default();
    unsafe {
        *u.mi_mut() = MOUSEINPUT {
            dx,
            dy,
            mouseData: data,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: extra_info,
        };
    }
    let mut input = INPUT {
        type_: INPUT_MOUSE,
        u,
    };
    unsafe { SendInput(1, &mut input as LPINPUT, size_of::<INPUT>() as c_int) }
}

fn keybd_event(mut flags: u32, vk: u16, scan: u16, extra_info: ULONG_PTR) -> DWORD {
    let mut scan = scan;
    unsafe {
        // https://github.com/rustdesk/rustdesk/issues/366
        if scan == 0 {
            if LAYOUT.is_null() {
                let current_window_thread_id =
                    GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
                LAYOUT = GetKeyboardLayout(current_window_thread_id);
            }
            scan = MapVirtualKeyExW(vk as _, 0, LAYOUT) as _;
        }
    }

    if flags & KEYEVENTF_UNICODE == 0 {
        if scan >> 8 == 0xE0 || scan >> 8 == 0xE1 {
            flags |= winapi::um::winuser::KEYEVENTF_EXTENDEDKEY;
        }
    }
    let mut union: INPUT_u = unsafe { std::mem::zeroed() };
    unsafe {
        *union.ki_mut() = KEYBDINPUT {
            wVk: vk,
            wScan: scan,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: extra_info,
        };
    }
    let mut inputs = [INPUT {
        type_: INPUT_KEYBOARD,
        u: union,
    }; 1];
    unsafe {
        SendInput(
            inputs.len() as UINT,
            inputs.as_mut_ptr(),
            size_of::<INPUT>() as c_int,
        )
    }
}

fn get_error() -> String {
    unsafe {
        let buff_size = 256;
        let mut buff: Vec<u16> = Vec::with_capacity(buff_size);
        buff.resize(buff_size, 0);
        let errno = GetLastError();
        let chars_copied = FormatMessageW(
            FORMAT_MESSAGE_IGNORE_INSERTS
                | FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_ARGUMENT_ARRAY,
            std::ptr::null(),
            errno,
            0,
            buff.as_mut_ptr(),
            (buff_size + 1) as u32,
            std::ptr::null_mut(),
        );
        if chars_copied == 0 {
            return "".to_owned();
        }
        let mut curr_char: usize = chars_copied as usize;
        while curr_char > 0 {
            let ch = buff[curr_char];

            if ch >= ' ' as u16 {
                break;
            }
            curr_char -= 1;
        }
        let sl = std::slice::from_raw_parts(buff.as_ptr(), curr_char);
        let err_msg = String::from_utf16(sl);
        return err_msg.unwrap_or("".to_owned());
    }
}

impl MouseControllable for Enigo {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn mouse_move_to(&mut self, x: i32, y: i32) {
        let extra_info = self.get_extra_info();
        mouse_event(
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
            0,
            (x - unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) }) * 65535
                / unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
            (y - unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) }) * 65535
                / unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
            extra_info,
        );
    }

    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        let extra_info = self.get_extra_info();
        mouse_event(MOUSEEVENTF_MOVE, 0, x, y, extra_info);
    }

    fn mouse_down(&mut self, button: MouseButton) -> crate::ResultType {
        let extra_info = self.get_extra_info();
        let res = mouse_event(
            match button {
                MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
                MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
                MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
                MouseButton::Back => MOUSEEVENTF_XDOWN,
                MouseButton::Forward => MOUSEEVENTF_XDOWN,
                _ => {
                    log::info!("Unsupported button {:?}", button);
                    return Ok(());
                }
            },
            match button {
                MouseButton::Back => XBUTTON1 as u32,
                MouseButton::Forward => XBUTTON2 as u32,
                _ => 0,
            },
            0,
            0,
            extra_info,
        );
        if res == 0 {
            let err = get_error();
            if !err.is_empty() {
                return Err(err.into());
            }
        }
        Ok(())
    }

    fn mouse_up(&mut self, button: MouseButton) {
        let extra_info = self.get_extra_info();
        mouse_event(
            match button {
                MouseButton::Left => MOUSEEVENTF_LEFTUP,
                MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
                MouseButton::Right => MOUSEEVENTF_RIGHTUP,
                MouseButton::Back => MOUSEEVENTF_XUP,
                MouseButton::Forward => MOUSEEVENTF_XUP,
                _ => {
                    log::info!("Unsupported button {:?}", button);
                    return;
                }
            },
            match button {
                MouseButton::Back => XBUTTON1 as _,
                MouseButton::Forward => XBUTTON2 as _,
                _ => 0,
            },
            0,
            0,
            extra_info,
        );
    }

    fn mouse_click(&mut self, button: MouseButton) {
        self.mouse_down(button).ok();
        self.mouse_up(button);
    }

    fn mouse_scroll_x(&mut self, length: i32) {
        let extra_info = self.get_extra_info();
        mouse_event(MOUSEEVENTF_HWHEEL, length as _, 0, 0, extra_info);
    }

    fn mouse_scroll_y(&mut self, length: i32) {
        let extra_info = self.get_extra_info();
        mouse_event(MOUSEEVENTF_WHEEL, length as _, 0, 0, extra_info);
    }
}

impl KeyboardControllable for Enigo {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn key_sequence(&mut self, sequence: &str) {
        let mut buffer = [0; 2];

        for c in sequence.chars() {
            // Windows uses uft-16 encoding. We need to check
            // for variable length characters. As such some
            // characters can be 32 bit long and those are
            // encoded in such called hight and low surrogates
            // each 16 bit wide that needs to be send after
            // another to the SendInput function without
            // being interrupted by "keyup"
            let result = c.encode_utf16(&mut buffer);
            if result.len() == 1 {
                self.unicode_key_click(result[0]);
            } else {
                for utf16_surrogate in result {
                    self.unicode_key_down(utf16_surrogate.clone());
                }
                // do i need to produce a keyup?
                // self.unicode_key_up(0);
            }
        }
    }

    fn key_click(&mut self, key: Key) {
        let vk = self.key_to_keycode(key);
        let extra_info = self.get_extra_info();
        keybd_event(0, vk, 0, extra_info);
        keybd_event(KEYEVENTF_KEYUP, vk, 0, extra_info);
    }

    fn key_down(&mut self, key: Key) -> crate::ResultType {
        let extra_info = self.get_extra_info();
        match &key {
            Key::Layout(c) => {
                // to-do: dup code
                // https://github.com/rustdesk/rustdesk/blob/1bc0dd791ed8344997024dc46626bd2ca7df73d2/src/server/input_service.rs#L1348
                let code = self.get_layoutdependent_keycode(*c);
                if code as u16 != 0xFFFF {
                    let vk = code & 0x00FF;
                    let flag = code >> 8;
                    let modifiers = [Key::Shift, Key::Control, Key::Alt];
                    let mod_len = modifiers.len();
                    for pos in 0..mod_len {
                        if flag & (0x0001 << pos) != 0 {
                            self.key_down(modifiers[pos])?;
                        }
                    }

                    let res = keybd_event(0, vk, 0, extra_info);
                    let err = if res == 0 { get_error() } else { "".to_owned() };

                    for pos in 0..mod_len {
                        let rpos = mod_len - 1 - pos;
                        if flag & (0x0001 << rpos) != 0 {
                            self.key_up(modifiers[pos]);
                        }
                    }

                    if !err.is_empty() {
                        return Err(err.into());
                    }
                } else {
                    return Err(format!("Failed to get keycode of {}", c).into());
                }
            }
            _ => {
                let code = self.key_to_keycode(key);
                if code == 0 || code == 65535 {
                    return Err("".into());
                }
                let res = keybd_event(0, code, 0, extra_info);
                if res == 0 {
                    let err = get_error();
                    if !err.is_empty() {
                        return Err(err.into());
                    }
                }
            }
        }
        Ok(())
    }

    fn key_up(&mut self, key: Key) {
        let extra_info = self.get_extra_info();
        keybd_event(KEYEVENTF_KEYUP, self.key_to_keycode(key), 0, extra_info);
    }

    fn get_key_state(&mut self, key: Key) -> bool {
        let keycode = self.key_to_keycode(key);
        let x = unsafe { GetKeyState(keycode as _) };
        if key == Key::CapsLock || key == Key::NumLock || key == Key::Scroll {
            return (x & 0x1) == 0x1;
        }
        return (x as u16 & 0x8000) == 0x8000;
    }
}

impl Enigo {
    /// Get the extra info value to use for input injection
    /// Returns MouseMux ID if enabled and valid, otherwise ENIGO_INPUT_EXTRA_VALUE
    fn get_extra_info(&self) -> ULONG_PTR {
        self.mousemux_input_id.unwrap_or(ENIGO_INPUT_EXTRA_VALUE)
    }

    /// Enable MouseMux integration
    /// Attempts to find the MouseMux window and register with it
    /// Returns true if successfully registered, false otherwise
    pub fn enable_mousemux(&mut self, rustdesk_version: u32) -> bool {
        unsafe {
            // Find the MouseMux query window
            let window_class = std::ffi::CString::new(MOUSEMUX_WINDOW_CLASS).unwrap();
            let hwnd = FindWindowA(window_class.as_ptr() as *const i8, std::ptr::null());

            if hwnd.is_null() {
                log::warn!("MouseMux: Window not found");
                self.mousemux_input_id = None;
                return false;
            }

            // Send registration message with timeout
            let mut result: DWORD_PTR = 0;
            let send_result = SendMessageTimeoutA(
                hwnd,
                MOUSEMUX_MSG_REGISTER,
                rustdesk_version as WPARAM,
                0,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                MOUSEMUX_TIMEOUT_MS,
                &mut result as *mut DWORD_PTR,
            );

            if send_result == 0 {
                log::warn!("MouseMux: SendMessageTimeout failed");
                self.mousemux_input_id = None;
                return false;
            }

            // Check if result is in valid range
            let result_ulong = result as ULONG_PTR;
            if result_ulong > MOUSEMUX_ID_MIN && result_ulong < MOUSEMUX_ID_MAX {
                log::info!("MouseMux: Registered with ID {}", result_ulong);
                self.mousemux_input_id = Some(result_ulong);
                true
            } else {
                log::warn!("MouseMux: Invalid ID returned: {}", result_ulong);
                self.mousemux_input_id = None;
                false
            }
        }
    }

    /// Disable MouseMux integration
    /// Sends unregistration message and clears the stored ID
    pub fn disable_mousemux(&mut self) {
        if let Some(input_id) = self.mousemux_input_id {
            unsafe {
                // Find the MouseMux query window
                let window_class = std::ffi::CString::new(MOUSEMUX_WINDOW_CLASS).unwrap();
                let hwnd = FindWindowA(window_class.as_ptr() as *const i8, std::ptr::null());

                if !hwnd.is_null() {
                    // Send unregistration message
                    let mut result: DWORD_PTR = 0;
                    SendMessageTimeoutA(
                        hwnd,
                        MOUSEMUX_MSG_UNREGISTER,
                        input_id as WPARAM,
                        0,
                        SMTO_ABORTIFHUNG | SMTO_BLOCK,
                        MOUSEMUX_TIMEOUT_MS,
                        &mut result as *mut DWORD_PTR,
                    );
                    log::info!("MouseMux: Unregistered ID {}", input_id);
                }
            }
        }
        self.mousemux_input_id = None;
    }

    /// Check if MouseMux is currently enabled
    pub fn is_mousemux_enabled(&self) -> bool {
        self.mousemux_input_id.is_some()
    }

    /// Gets the (width, height) of the main display in screen coordinates
    /// (pixels).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use enigo::*;
    /// let mut size = Enigo::main_display_size();
    /// ```
    pub fn main_display_size() -> (usize, usize) {
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) as usize };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) as usize };
        (w, h)
    }

    /// Gets the location of mouse in screen coordinates (pixels).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use enigo::*;
    /// let mut location = Enigo::mouse_location();
    /// ```
    pub fn mouse_location() -> (i32, i32) {
        let mut point = POINT { x: 0, y: 0 };
        let result = unsafe { GetCursorPos(&mut point) };
        if result != 0 {
            (point.x, point.y)
        } else {
            (0, 0)
        }
    }

    fn unicode_key_click(&self, unicode_char: u16) {
        self.unicode_key_down(unicode_char);
        self.unicode_key_up(unicode_char);
    }

    fn unicode_key_down(&self, unicode_char: u16) {
        let extra_info = self.get_extra_info();
        keybd_event(KEYEVENTF_UNICODE, 0, unicode_char, extra_info);
    }

    fn unicode_key_up(&self, unicode_char: u16) {
        let extra_info = self.get_extra_info();
        keybd_event(KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, 0, unicode_char, extra_info);
    }

    fn key_to_keycode(&self, key: Key) -> u16 {
        // do not use the codes from crate winapi they're
        // wrongly typed with i32 instead of i16 use the
        // ones provided by win/keycodes.rs that are prefixed
        // with an 'E' infront of the original name
        #[allow(deprecated)]
        // I mean duh, we still need to support deprecated keys until they're removed
        match key {
            Key::Alt => EVK_MENU,
            Key::Backspace => EVK_BACK,
            Key::CapsLock => EVK_CAPITAL,
            Key::Control => EVK_LCONTROL,
            Key::Delete => EVK_DELETE,
            Key::DownArrow => EVK_DOWN,
            Key::End => EVK_END,
            Key::Escape => EVK_ESCAPE,
            Key::F1 => EVK_F1,
            Key::F10 => EVK_F10,
            Key::F11 => EVK_F11,
            Key::F12 => EVK_F12,
            Key::F2 => EVK_F2,
            Key::F3 => EVK_F3,
            Key::F4 => EVK_F4,
            Key::F5 => EVK_F5,
            Key::F6 => EVK_F6,
            Key::F7 => EVK_F7,
            Key::F8 => EVK_F8,
            Key::F9 => EVK_F9,
            Key::Home => EVK_HOME,
            Key::LeftArrow => EVK_LEFT,
            Key::Option => EVK_MENU,
            Key::PageDown => EVK_NEXT,
            Key::PageUp => EVK_PRIOR,
            Key::Return => EVK_RETURN,
            Key::RightArrow => EVK_RIGHT,
            Key::Shift => EVK_SHIFT,
            Key::Space => EVK_SPACE,
            Key::Tab => EVK_TAB,
            Key::UpArrow => EVK_UP,
            Key::Numpad0 => EVK_NUMPAD0,
            Key::Numpad1 => EVK_NUMPAD1,
            Key::Numpad2 => EVK_NUMPAD2,
            Key::Numpad3 => EVK_NUMPAD3,
            Key::Numpad4 => EVK_NUMPAD4,
            Key::Numpad5 => EVK_NUMPAD5,
            Key::Numpad6 => EVK_NUMPAD6,
            Key::Numpad7 => EVK_NUMPAD7,
            Key::Numpad8 => EVK_NUMPAD8,
            Key::Numpad9 => EVK_NUMPAD9,
            Key::Cancel => EVK_CANCEL,
            Key::Clear => EVK_CLEAR,
            Key::Pause => EVK_PAUSE,
            Key::Kana => EVK_KANA,
            Key::Hangul => EVK_HANGUL,
            Key::Junja => EVK_JUNJA,
            Key::Final => EVK_FINAL,
            Key::Hanja => EVK_HANJA,
            Key::Kanji => EVK_KANJI,
            Key::Convert => EVK_CONVERT,
            Key::Select => EVK_SELECT,
            Key::Print => EVK_PRINT,
            Key::Execute => EVK_EXECUTE,
            Key::Snapshot => EVK_SNAPSHOT,
            Key::Insert => EVK_INSERT,
            Key::Help => EVK_HELP,
            Key::Sleep => EVK_SLEEP,
            Key::Separator => EVK_SEPARATOR,
            Key::Mute => EVK_VOLUME_MUTE,
            Key::VolumeDown => EVK_VOLUME_DOWN,
            Key::VolumeUp => EVK_VOLUME_UP,
            Key::Scroll => EVK_SCROLL,
            Key::NumLock => EVK_NUMLOCK,
            Key::RWin => EVK_RWIN,
            Key::Apps => EVK_APPS,
            Key::Add => EVK_ADD,
            Key::Multiply => EVK_MULTIPLY,
            Key::Decimal => EVK_DECIMAL,
            Key::Subtract => EVK_SUBTRACT,
            Key::Divide => EVK_DIVIDE,
            Key::NumpadEnter => EVK_RETURN,
            Key::Equals => '=' as _,
            Key::RightShift => EVK_RSHIFT,
            Key::RightControl => EVK_RCONTROL,
            Key::RightAlt => EVK_RMENU,

            Key::Raw(raw_keycode) => raw_keycode,
            Key::Super | Key::Command | Key::Windows | Key::Meta => EVK_LWIN,
            Key::Layout(..) => {
                // unreachable
                0
            }
        }
    }

    fn get_layoutdependent_keycode(&self, chr: char) -> u16 {
        unsafe {
            LAYOUT = std::ptr::null_mut();
        }
        // NOTE VkKeyScanW uses the current keyboard LAYOUT
        // to specify a LAYOUT use VkKeyScanExW and GetKeyboardLayout
        // or load one with LoadKeyboardLayoutW
        let current_window_thread_id =
            unsafe { GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut()) };
        unsafe { LAYOUT = GetKeyboardLayout(current_window_thread_id) };
        unsafe { VkKeyScanExW(chr as _, LAYOUT) as _ }
    }
}
