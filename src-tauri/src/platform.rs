//! OS glue: clipboard, synthetic paste, focus restore, speaker mute, non-activating overlay.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::time::Duration;

// On Linux the clipboard content dies with the Clipboard object, so keep one alive.
static CLIPBOARD: Lazy<Mutex<Option<arboard::Clipboard>>> = Lazy::new(|| Mutex::new(arboard::Clipboard::new().ok()));

pub fn set_clipboard(text: &str) -> Result<()> {
    let mut guard = CLIPBOARD.lock();
    if guard.is_none() {
        *guard = arboard::Clipboard::new().ok();
    }
    let cb = guard.as_mut().ok_or_else(|| anyhow!("área de transferência indisponível"))?;
    cb.set_text(text.to_string()).map_err(|e| anyhow!("não foi possível copiar: {e}"))
}

/// Handle of the window that had focus when recording started (0 = unknown).
pub fn foreground_window() -> isize {
    #[cfg(windows)]
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// Center of the focused window (physical px), ignoring FlowVoice's own windows.
pub fn foreground_center(app: &tauri::AppHandle) -> Option<(f64, f64)> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
        let fg = unsafe { GetForegroundWindow() };
        if fg.0.is_null() {
            return None;
        }
        for label in ["main", "overlay", "chat"] {
            if let Some(w) = app.get_webview_window(label) {
                if w.hwnd().map(|h| h.0 as isize == fg.0 as isize).unwrap_or(false) {
                    return None;
                }
            }
        }
        let mut r = RECT::default();
        unsafe { GetWindowRect(fg, &mut r) }.ok()?;
        if r.right <= r.left || r.bottom <= r.top {
            return None;
        }
        return Some((((r.left + r.right) / 2) as f64, ((r.top + r.bottom) / 2) as f64));
    }
    #[allow(unreachable_code)]
    {
        let _ = app;
        None
    }
}

/// Can we synthesize Ctrl+V here? (Not on Wayland without wtype.)
pub fn can_paste() -> bool {
    #[cfg(target_os = "linux")]
    {
        if is_wayland() {
            return which("wtype");
        }
        return which("xdotool");
    }
    #[allow(unreachable_code)]
    true
}

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn which(bin: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {bin}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Copies `text`, gives focus back to `target` and sends Ctrl+V (after Ctrl+Z when `replace`).
pub fn paste(text: &str, target: isize, replace: bool) -> Result<bool> {
    set_clipboard(text)?;
    if !can_paste() {
        return Ok(false);
    }
    wait_modifiers_released();
    focus(target);
    std::thread::sleep(Duration::from_millis(60));
    if replace {
        send_ctrl('Z');
        std::thread::sleep(Duration::from_millis(80));
    }
    send_ctrl('V');
    Ok(true)
}

#[cfg(windows)]
fn focus(target: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow};
    if target == 0 {
        return;
    }
    unsafe {
        let hwnd = HWND(target as *mut _);
        if !IsWindow(hwnd).as_bool() {
            return;
        }
        let fg = GetForegroundWindow();
        if fg == hwnd {
            return;
        }
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
    }
}

#[cfg(not(windows))]
fn focus(_target: isize) {}

#[cfg(windows)]
fn wait_modifiers_released() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};
    for _ in 0..60 {
        let held = [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            .any(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0);
        if !held {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(not(windows))]
fn wait_modifiers_released() {
    std::thread::sleep(Duration::from_millis(120));
}

#[cfg(windows)]
fn send_ctrl(letter: char) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
    };
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let letter = VIRTUAL_KEY(letter as u16);
    let inputs = [key(VK_CONTROL, false), key(letter, false), key(letter, true), key(VK_CONTROL, true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "linux")]
fn send_ctrl(letter: char) {
    let l = letter.to_ascii_lowercase().to_string();
    let _ = if is_wayland() {
        std::process::Command::new("wtype").args(["-M", "ctrl", &l, "-m", "ctrl"]).status()
    } else {
        std::process::Command::new("xdotool").args(["key", "--clearmodifiers", &format!("ctrl+{l}")]).status()
    };
}

#[cfg(target_os = "macos")]
fn send_ctrl(_letter: char) {}

// ---------------------------------------------------------------- speaker mute

/// Mutes the default output; returns the previous state so it can be restored.
pub fn mute_speakers(mute: bool) -> Option<bool> {
    #[cfg(windows)]
    {
        return win_mute(mute).map_err(|e| log::warn!("mute failed: {e}")).ok();
    }
    #[cfg(target_os = "linux")]
    {
        let prev = std::process::Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("yes"))?;
        let _ = std::process::Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", if mute { "1" } else { "0" }])
            .status();
        return Some(prev);
    }
    #[allow(unreachable_code)]
    {
        let _ = mute;
        None
    }
}

#[cfg(windows)]
fn win_mute(mute: bool) -> windows::core::Result<bool> {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let prev = volume.GetMute()?.as_bool();
        volume.SetMute(BOOL::from(mute), std::ptr::null())?;
        Ok(prev)
    }
}

// ---------------------------------------------------------------- overlay window

/// Sets position and size together (physical px) without activating the window.
pub fn set_bounds_no_activate(window: &tauri::WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE};
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let _ = SetWindowPos(HWND(hwnd.0 as *mut _), HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
            }
            return;
        }
    }
    let _ = window.set_size(tauri::PhysicalSize::new(w.max(1) as u32, h.max(1) as u32));
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Shows a window on top without activating it (focus stays in the app being dictated into).
pub fn show_without_focus(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE,
        };
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let hwnd = HWND(hwnd.0 as *mut _);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
            }
            return;
        }
    }
    let _ = window.show();
}

/// Lets the overlay be clicked without stealing focus from the app being dictated into.
pub fn make_non_activating(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW};
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let hwnd = HWND(hwnd.0 as *mut _);
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = window;
    }
}
