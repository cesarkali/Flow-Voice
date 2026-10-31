//! OS glue: clipboard, synthetic paste, focus restore, speaker mute, non-activating overlay.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::time::Duration;

// On Linux the clipboard content dies with the Clipboard object, so keep one alive.
static CLIPBOARD: Lazy<Mutex<Option<arboard::Clipboard>>> = Lazy::new(|| Mutex::new(arboard::Clipboard::new().ok()));

pub fn set_clipboard(text: &str) -> Result<()> {
    with_clipboard(|cb| cb.set_text(text.to_string()))
}

fn with_clipboard<T>(f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, arboard::Error>) -> Result<T> {
    let mut guard = CLIPBOARD.lock();
    if guard.is_none() {
        *guard = arboard::Clipboard::new().ok();
    }
    let cb = guard.as_mut().ok_or_else(|| anyhow!("área de transferência indisponível"))?;
    f(cb).map_err(|e| anyhow!("não foi possível copiar: {e}"))
}

fn clipboard_text() -> Option<String> {
    with_clipboard(|cb| cb.get_text()).ok()
}

/// Puts text on the clipboard only to paste it: kept out of Win+V history and cloud sync.
fn set_clipboard_transient(text: &str) -> Result<()> {
    #[cfg(windows)]
    {
        use arboard::SetExtWindows;
        with_clipboard(|cb| cb.set().exclude_from_history().exclude_from_cloud().text(text.to_string()))
    }
    #[cfg(target_os = "linux")]
    {
        use arboard::SetExtLinux;
        with_clipboard(|cb| cb.set().exclude_from_history().text(text.to_string()))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        set_clipboard(text)
    }
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

/// Gives focus back to `target` and pastes `text` with Ctrl+V (after Ctrl+Z when `replace`).
/// Returns Ok(false) when it could not paste safely; the text is then left on the clipboard for
/// the user to paste. After a successful paste the user's previous clipboard text comes back.
pub fn paste(text: &str, target: isize, replace: bool) -> Result<bool> {
    let manual = |why: &str| -> Result<bool> {
        log::info!("paste skipped: {why}");
        set_clipboard(text)?;
        Ok(false)
    };
    if !can_paste() {
        return manual("no input injection here");
    }
    if replace && is_console(target) {
        // Ctrl+Z suspends the running job in terminals instead of undoing.
        return manual("replace in a terminal");
    }
    if is_elevated(target) {
        // Windows blocks input from a normal app into an admin window (UIPI).
        return manual("target runs as administrator");
    }
    let previous = clipboard_text();
    set_clipboard_transient(text)?;
    wait_modifiers_released();
    focus(target);
    std::thread::sleep(Duration::from_millis(60));
    if !has_focus(target) {
        return manual("could not focus the target window");
    }
    if replace {
        send_ctrl('Z');
        std::thread::sleep(Duration::from_millis(80));
    }
    send_ctrl('V');
    // Give the target time to read the clipboard (slow apps, remote desktops), then restore
    // what the user had copied, unless something else changed the clipboard meanwhile.
    if let Some(prev) = previous.filter(|p| p != text) {
        let ours = text.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            if clipboard_text().as_deref() == Some(ours.as_str()) {
                let _ = set_clipboard(&prev);
            }
        });
    }
    Ok(true)
}

// ---------------------------------------------------------------- selection (Comando)

/// Longest selection "Comando" accepts (the LLM has to send all of it back).
pub const SELECTION_MAX_CHARS: usize = 12_000;

/// Copies what is selected in `target` with a synthetic Ctrl+C and gives the user's clipboard
/// text back right after. Ok(None) = nothing selected (the app left the clipboard untouched).
pub fn copy_selection(target: isize) -> Result<Option<String>> {
    if !can_paste() {
        return Err(anyhow!("Este sistema não permite ler o texto selecionado (instale o xdotool ou o wtype)."));
    }
    if is_elevated(target) {
        return Err(anyhow!("Esse app roda como administrador — o Windows não deixa o FlowVoice ler a seleção dele."));
    }
    if is_console(target) {
        // Ctrl+C would interrupt the running program.
        return Err(anyhow!("O Comando não funciona em terminais (lá o Ctrl+C interrompe o programa). Use num editor ou campo de texto."));
    }
    let previous = clipboard_text();
    // Windows: the clipboard sequence number tells whether the app copied anything, so the
    // clipboard is left alone (images included) when nothing is selected. Elsewhere: clear it.
    #[cfg(windows)]
    let seq0 = clipboard_seq();
    #[cfg(not(windows))]
    let _ = with_clipboard(|cb| cb.clear());
    wait_modifiers_released();
    focus(target);
    if !has_focus(target) {
        return Err(anyhow!("A janela com o texto selecionado perdeu o foco."));
    }
    send_ctrl('C');
    let t0 = std::time::Instant::now();
    let mut copied = None;
    // Electron/Office apps can take a moment to answer Ctrl+C.
    while t0.elapsed() < Duration::from_millis(400) {
        std::thread::sleep(Duration::from_millis(20));
        #[cfg(windows)]
        let changed = clipboard_seq() != seq0;
        #[cfg(not(windows))]
        let changed = true;
        if changed {
            if let Some(t) = clipboard_text().filter(|t| !t.is_empty()) {
                copied = Some(t);
                break;
            }
        }
    }
    #[cfg(windows)]
    let touched = clipboard_seq() != seq0;
    #[cfg(not(windows))]
    let touched = true;
    if touched {
        match &previous {
            Some(p) => {
                let _ = set_clipboard(p);
            }
            None => {
                let _ = with_clipboard(|cb| cb.clear());
            }
        }
    }
    log::info!("selection: {} chars", copied.as_ref().map(|t| t.chars().count()).unwrap_or(0));
    Ok(copied.filter(|t| !t.trim().is_empty()))
}

#[cfg(windows)]
fn clipboard_seq() -> u32 {
    unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

// ---------------------------------------------------------------- target app

/// The app that owns `target` (Windows) or the active X11 window (Linux, needs xdotool).
pub fn window_app(target: isize) -> Option<crate::apps::AppContext> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
        if target == 0 {
            return None;
        }
        let hwnd = HWND(target as *mut _);
        let pid = window_pid(hwnd)?;
        let mut exe = process_exe(pid)?;
        // Store/UWP apps live inside ApplicationFrameHost: the real process owns a child window.
        if crate::apps::normalize(&exe) == "applicationframehost" {
            if let Some(child) = uwp_child_pid(hwnd, pid).and_then(process_exe) {
                exe = child;
            }
        }
        let mut buf = [0u16; 512];
        let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
        let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        crate::apps::detect(&exe, &title)
    }
    #[cfg(target_os = "linux")]
    {
        let _ = target;
        if is_wayland() || !which("xdotool") {
            return None;
        }
        let run = |args: &[&str]| {
            std::process::Command::new("xdotool")
                .args(args)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        };
        let pid = run(&["getactivewindow", "getwindowpid"])?.parse::<u32>().ok()?;
        let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
            .map(|p| p.to_string_lossy().to_string())
            .ok()
            .or_else(|| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok())?;
        let title = run(&["getactivewindow", "getwindowname"]).unwrap_or_default();
        crate::apps::detect(&exe, &title)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = target;
        None
    }
}

#[cfg(windows)]
fn window_pid(hwnd: windows::Win32::Foundation::HWND) -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

#[cfg(windows)]
fn process_exe(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

#[cfg(windows)]
fn uwp_child_pid(hwnd: windows::Win32::Foundation::HWND, host_pid: u32) -> Option<u32> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::EnumChildWindows;
    struct Search {
        host: u32,
        found: u32,
    }
    unsafe extern "system" fn visit(child: HWND, lp: LPARAM) -> BOOL {
        let s = &mut *(lp.0 as *mut Search);
        match window_pid(child) {
            Some(pid) if pid != s.host => {
                s.found = pid;
                BOOL(0)
            }
            _ => BOOL(1),
        }
    }
    let mut s = Search { host: host_pid, found: 0 };
    unsafe {
        let _ = EnumChildWindows(hwnd, Some(visit), LPARAM(&mut s as *mut Search as isize));
    }
    (s.found != 0).then_some(s.found)
}

// ---------------------------------------------------------------- last app window (tray)

static LAST_APP_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// Remembers the last focused window of another app, so tray actions (the tray menu takes the
/// focus) can still paste where the user was working.
pub fn start_focus_tracker() {
    #[cfg(windows)]
    std::thread::spawn(|| loop {
        let fg = foreground_window();
        if is_app_window(fg) {
            LAST_APP_WINDOW.store(fg, std::sync::atomic::Ordering::Relaxed);
        }
        std::thread::sleep(Duration::from_millis(300));
    });
}

/// Last focused window of another app (0 = unknown: paste goes to whatever has focus).
pub fn last_app_window() -> isize {
    let fg = foreground_window();
    if is_app_window(fg) {
        return fg;
    }
    LAST_APP_WINDOW.load(std::sync::atomic::Ordering::Relaxed)
}

/// A real app window: not FlowVoice's own, not the taskbar/tray/menus.
#[cfg(windows)]
fn is_app_window(hwnd: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
    if hwnd == 0 {
        return false;
    }
    let h = HWND(hwnd as *mut _);
    if window_pid(h) == Some(unsafe { GetCurrentProcessId() }) {
        return false;
    }
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut buf) };
    let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
    ![
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
        "NotifyIconOverflowWindow",
        "TopLevelWindowForOverflowXamlIsland",
        "#32768",
        "Progman",
        "WorkerW",
        "XamlExplorerHostIslandWindow",
        "Windows.UI.Core.CoreWindow",
    ]
    .iter()
    .any(|c| class.eq_ignore_ascii_case(c))
}

#[cfg(not(windows))]
fn is_app_window(_hwnd: isize) -> bool {
    false
}

/// Did the target window really get the focus? (Unknown target = trust the current focus.)
#[cfg(windows)]
fn has_focus(target: isize) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GA_ROOT};
    if target == 0 {
        return true;
    }
    unsafe {
        let fg = GetForegroundWindow();
        fg.0 as isize == target || GetAncestor(fg, GA_ROOT).0 as isize == target
    }
}

#[cfg(not(windows))]
fn has_focus(_target: isize) -> bool {
    true
}

/// Console hosts where Ctrl+Z is not "undo".
#[cfg(windows)]
fn is_console(target: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
    if target == 0 {
        return false;
    }
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(HWND(target as *mut _), &mut buf) };
    let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
    ["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS", "mintty", "VirtualConsoleClass", "PuTTY", "Alacritty", "org.wezfurlong.wezterm"]
        .iter()
        .any(|c| class.eq_ignore_ascii_case(c))
}

#[cfg(not(windows))]
fn is_console(_target: isize) -> bool {
    false
}

/// True when the target window belongs to an elevated process and FlowVoice is not elevated.
#[cfg(windows)]
fn is_elevated(target: isize) -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    fn elevated(process: HANDLE) -> Option<bool> {
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
            let mut info = TOKEN_ELEVATION::default();
            let mut len = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut info as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            );
            let _ = CloseHandle(token);
            ok.ok()?;
            Some(info.TokenIsElevated != 0)
        }
    }
    if target == 0 {
        return false;
    }
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(HWND(target as *mut _), Some(&mut pid));
        if pid == 0 || elevated(GetCurrentProcess()) == Some(true) {
            return false;
        }
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => {
                let r = elevated(h).unwrap_or(false);
                let _ = CloseHandle(h);
                r
            }
            Err(_) => false,
        }
    }
}

#[cfg(not(windows))]
fn is_elevated(_target: isize) -> bool {
    false
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
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
        VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    let mods = [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN];
    let is_held = |vk: &VIRTUAL_KEY| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0;
    for _ in 0..60 {
        if !mods.iter().any(is_held) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    // Still "held" after 1.5 s (stuck key state): release them so the paste is a plain Ctrl+V,
    // not Ctrl+Shift+V (which runs other commands in some apps).
    let ups: Vec<INPUT> = mods
        .iter()
        .filter(|vk| is_held(vk))
        .map(|vk| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: *vk, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 } },
        })
        .collect();
    unsafe {
        SendInput(&ups, std::mem::size_of::<INPUT>() as i32);
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

/// Current outer rectangle (physical px) as (x, y, width, height).
pub fn window_rect(window: &tauri::WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, RECT};
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let hwnd = window.hwnd().ok()?;
        let mut r = RECT::default();
        unsafe { GetWindowRect(HWND(hwnd.0 as *mut _), &mut r) }.ok()?;
        return Some((r.left, r.top, r.right - r.left, r.bottom - r.top));
    }
    #[allow(unreachable_code)]
    {
        let p = window.outer_position().ok()?;
        let s = window.outer_size().ok()?;
        Some((p.x, p.y, s.width as i32, s.height as i32))
    }
}

/// Origin (physical px) of the monitor showing the focused window — the one Windows itself
/// assigns it to (largest overlap). None when the focus is the desktop, the taskbar or a
/// FlowVoice window, so the caller can fall back to the monitor under the mouse.
pub fn foreground_monitor_origin(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL};
        use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};
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
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(fg, &mut buf) };
        let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        if ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"].contains(&class.as_str()) {
            return None;
        }
        let mon = unsafe { MonitorFromWindow(fg, MONITOR_DEFAULTTONULL) };
        if mon.is_invalid() {
            return None;
        }
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !unsafe { GetMonitorInfoW(mon, &mut info) }.as_bool() {
            return None;
        }
        return Some((info.rcMonitor.left, info.rcMonitor.top));
    }
    #[allow(unreachable_code)]
    {
        let _ = app;
        None
    }
}

/// Sets position and size (physical px) without activating the window. Two steps: first move it
/// onto the target monitor (Windows may rescale it there if that monitor has another DPI), then
/// apply the exact size, so a DPI change can't leave it bigger and off-center.
pub fn set_bounds_no_activate(window: &tauri::WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE};
        if let Ok(hwnd) = window.hwnd() {
            let hwnd = HWND(hwnd.0 as *mut _);
            unsafe {
                let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOACTIVATE | SWP_NOSIZE);
                let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
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

/// Hides a window shown with show_without_focus (see windows::setup_overlay for why the overlay
/// never goes through tao's hide()).
pub fn hide_window(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0 as *mut _), SW_HIDE);
            }
            return;
        }
    }
    let _ = window.hide();
}

pub fn is_window_visible(window: &tauri::WebviewWindow) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
        if let Ok(hwnd) = window.hwnd() {
            return unsafe { IsWindowVisible(HWND(hwnd.0 as *mut _)) }.as_bool();
        }
    }
    window.is_visible().unwrap_or(false)
}

/// Mouse passes through the window (true) or is caught by it (false). Same styles tao uses
/// (WS_EX_TRANSPARENT | WS_EX_LAYERED), set directly so nothing else about the window changes.
pub fn set_click_through(window: &tauri::WebviewWindow, on: bool) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, SWP_NOZORDER, WS_EX_LAYERED, WS_EX_TRANSPARENT,
        };
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let hwnd = HWND(hwnd.0 as *mut _);
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let bits = (WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0) as isize;
                let next = if on { style | bits } else { style & !bits };
                if next != style {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    );
                }
            }
            return;
        }
    }
    let _ = window.set_ignore_cursor_events(on);
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
