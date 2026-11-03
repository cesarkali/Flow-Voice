//! OS glue: clipboard, synthetic paste, focus restore, speaker mute, non-activating overlay.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::time::Duration;

// On Linux the clipboard content dies with the Clipboard object, so keep one alive.
static CLIPBOARD: Lazy<Mutex<Option<arboard::Clipboard>>> = Lazy::new(|| Mutex::new(arboard::Clipboard::new().ok()));

/// One synthetic keyboard sequence at a time (a background paste must not interleave its
/// Ctrl+V with a Command's Ctrl+C, nor two pastes with each other).
static INPUT_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub fn set_clipboard(text: &str) -> Result<()> {
    #[cfg(windows)]
    let text = &with_crlf(text);
    with_clipboard(|cb| cb.set_text(text.to_string()))
}

/// Windows' line breaks: classic edit controls (Notepad's older versions, WinForms, many dialogs)
/// paste a text with bare "\n" on a single line.
#[cfg_attr(not(windows), allow(dead_code))]
fn with_crlf(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.matches('\n').count());
    let mut prev = '\0';
    for c in text.chars() {
        if c == '\n' && prev != '\r' {
            out.push('\r');
        }
        out.push(c);
        prev = c;
    }
    out
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

/// Where the last dictation was pasted, to tell whether the next one goes right after it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PasteMark {
    window: isize,
    focus: isize,
    at: Option<std::time::Instant>,
    last: Option<char>,
    /// The caret a moment after the paste, when the app uses Windows' caret (browsers and many
    /// editors don't): window + rect.
    caret: Option<(isize, [i32; 4])>,
}

static PASTE_MARK: Lazy<Mutex<PasteMark>> = Lazy::new(|| Mutex::new(PasteMark::default()));
/// A dictation more than this after the previous one is not its continuation.
const JOIN_WINDOW: Duration = Duration::from_secs(60);

/// The control with the keyboard focus in `window`'s thread, and the caret there (if any).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FocusNow {
    focus: isize,
    caret: Option<(isize, [i32; 4])>,
}

#[cfg(windows)]
fn focus_now(window: isize) -> FocusNow {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO};
    if window == 0 {
        return FocusNow::default();
    }
    unsafe {
        let thread = GetWindowThreadProcessId(HWND(window as *mut _), None);
        let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        if thread == 0 || GetGUIThreadInfo(thread, &mut info).is_err() {
            return FocusNow::default();
        }
        let r = info.rcCaret;
        FocusNow {
            focus: info.hwndFocus.0 as isize,
            caret: (!info.hwndCaret.0.is_null()).then_some((info.hwndCaret.0 as isize, [r.left, r.top, r.right, r.bottom])),
        }
    }
}

#[cfg(not(windows))]
fn focus_now(_window: isize) -> FocusNow {
    FocusNow::default()
}

/// Still where the last paste left off: same window and focused control, and the caret where
/// that paste left it (moved = the user clicked elsewhere, typed or pressed Enter; only known in
/// apps that use Windows' caret).
fn same_place(mark: &PasteMark, target: isize, now: &FocusNow) -> bool {
    target != 0
        && mark.window == target
        && mark.focus == now.focus
        && mark.at.is_some()
        && match (mark.caret, now.caret) {
            (Some(before), Some(after)) => before == after,
            _ => true,
        }
}

/// A new dictation into `target` continues the previous one: in the same place, less than a
/// minute later.
fn continues_mark(mark: &PasteMark, target: isize, now: &FocusNow) -> bool {
    same_place(mark, target, now) && mark.at.is_some_and(|at| at.elapsed() < JOIN_WINDOW)
}

/// What is pasted: a new dictation (spaced from the previous one when it continues it), a text
/// the card's "Substituir" may undo later (a Command's result, the card's "Colar"), or anything
/// else ("Colar de novo").
#[derive(Clone, Copy, PartialEq)]
enum PasteFor {
    Other,
    Dictation,
    Replaceable,
}

/// Gives focus back to `target` and pastes `text` with Ctrl+V. Returns Ok(false) when it could
/// not paste safely; the text is then left on the clipboard for the user to paste. Either way
/// the dictation is on the clipboard afterwards.
pub fn paste(text: &str, target: isize) -> Result<bool> {
    paste_with(text, target, false, PasteFor::Other)
}

/// `paste` for a new dictation (not a Command, "Colar de novo" or "Substituir"): spaced from the
/// previous one when it continues it in the same field.
pub fn paste_dictation(text: &str, target: isize) -> Result<bool> {
    paste_with(text, target, false, PasteFor::Dictation)
}

/// `paste` for a text the card's "Substituir" may undo afterwards: a Command's result, or the
/// card's "Colar" (`replace` false), and "Substituir" itself (`replace` true). Ctrl+Z is only
/// sent while the field is as that paste left it; otherwise the new version is only copied.
pub fn paste_replaceable(text: &str, target: isize, replace: bool) -> Result<bool> {
    paste_with(text, target, replace, PasteFor::Replaceable)
}

fn paste_with(text: &str, target: isize, replace: bool, what: PasteFor) -> Result<bool> {
    let dictation = what == PasteFor::Dictation;
    let _input = INPUT_LOCK.lock();
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
    // The dictation stays on the clipboard afterwards: pressing Ctrl+V again must paste THIS
    // text (restoring the previous clipboard made a later Ctrl+V paste an older dictation).
    set_clipboard(text)?;
    wait_modifiers_released();
    focus(target);
    std::thread::sleep(Duration::from_millis(60));
    if !has_focus(target) {
        return manual("could not focus the target window");
    }
    let now = focus_now(target);
    let mut mark = PASTE_MARK.lock();
    if replace {
        // Ctrl+Z would undo what the user did since (typed, moved), not the paste.
        if !same_place(&mark, target, &now) {
            return manual("replace: the field changed since the paste");
        }
        send_ctrl('Z');
        std::thread::sleep(Duration::from_millis(80));
    }
    // Typed, not on the clipboard: a "Substituir" (Ctrl+Z) undoes only the paste.
    if dictation && !is_console(target) && continues_mark(&mark, target, &now) && crate::text::needs_join_space(mark.last, text) {
        send_char(' ');
    }
    send_ctrl('V');
    if what != PasteFor::Other {
        // (A "Substituir" keeps the place: the next dictation continues it.)
        if !replace {
            *mark = PasteMark { window: target, focus: now.focus, ..Default::default() };
        }
        let at = std::time::Instant::now();
        mark.at = Some(at);
        // (After a Command or the card's "Colar", the next dictation isn't spaced from it.)
        mark.last = if dictation || replace { text.chars().last() } else { None };
        mark.caret = None;
        drop(mark);
        note_caret_after_paste(target, now.focus, at);
    } else {
        *mark = PasteMark::default();
    }
    Ok(true)
}

/// A moment after a dictation's (or a Command's) paste: where the app put the caret once it
/// handled the Ctrl+V (moved by the next dictation = the user went elsewhere: no space, and no
/// Ctrl+Z for "Substituir").
fn note_caret_after_paste(target: isize, focus: isize, at: std::time::Instant) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        let now = focus_now(target);
        let mut mark = PASTE_MARK.lock();
        if mark.at == Some(at) && now.focus == focus {
            mark.caret = now.caret;
        }
    });
}

// ---------------------------------------------------------------- selection (Comando)

/// Longest selection "Comando" accepts (the LLM has to send all of it back).
pub const SELECTION_MAX_CHARS: usize = 12_000;

/// Copies what is selected in `target` with a synthetic Ctrl+C and gives the user's clipboard
/// text back right after. Ok(None) = nothing selected (the app left the clipboard untouched).
pub fn copy_selection(target: isize) -> Result<Option<String>> {
    let _input = INPUT_LOCK.lock();
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

/// Types one character, whatever the keyboard layout.
#[cfg(windows)]
fn send_char(c: char) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY,
    };
    let mut units = [0u16; 2];
    let inputs: Vec<INPUT> = c
        .encode_utf16(&mut units)
        .iter()
        .flat_map(|&u| [false, true].map(|up| (u, up)))
        .map(|(u, up)| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: u,
                    dwFlags: if up { KEYEVENTF_UNICODE | KEYEVENTF_KEYUP } else { KEYEVENTF_UNICODE },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
        .collect();
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(not(windows))]
fn send_char(_c: char) {}

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
                // Already the exact size (a drag moves it 60x a second): nothing more to do.
                if window_rect(window).map(|r| (r.2, r.3)) != Some((w, h)) {
                    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
                }
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

/// Real cursor position (physical px, virtual-screen coordinates).
pub fn cursor_pos() -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut p = POINT::default();
        return unsafe { GetCursorPos(&mut p) }.ok().map(|_| (p.x, p.y));
    }
    #[allow(unreachable_code)]
    None
}

/// Whether the PRIMARY mouse button (the one that clicks: the right one when the user swapped the
/// buttons) is physically down right now, whichever window has the mouse.
pub fn primary_button_down() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON};
        // GetAsyncKeyState reports PHYSICAL buttons, regardless of the swap setting.
        let vk = if unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0 { VK_RBUTTON } else { VK_LBUTTON };
        return unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0;
    }
    #[allow(unreachable_code)]
    false
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

/// Whether the window currently lets the mouse through (see set_click_through). Test probes.
pub fn is_click_through(window: &tauri::WebviewWindow) -> Option<bool> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT};
        let hwnd = window.hwnd().ok()?;
        let style = unsafe { GetWindowLongPtrW(HWND(hwnd.0 as *mut _), GWL_EXSTYLE) };
        return Some(style & WS_EX_TRANSPARENT.0 as isize != 0);
    }
    #[allow(unreachable_code)]
    None
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

// ---------------------------------------------------------------- system info

/// Human-readable OS version for bug reports, e.g. "Windows 11 Pro 24H2 (build 26100)".
pub fn os_version() -> String {
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
        let read = |name: &str| -> Option<String> {
            let mut buf = [0u16; 128];
            let mut len = (buf.len() * 2) as u32;
            let key = HSTRING::from(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
            let ok = unsafe {
                RegGetValueW(
                    HKEY_LOCAL_MACHINE,
                    &key,
                    &HSTRING::from(name),
                    RRF_RT_REG_SZ,
                    None,
                    Some(buf.as_mut_ptr() as *mut _),
                    Some(&mut len),
                )
            };
            ok.is_ok().then(|| String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]))
        };
        let build: u32 = read("CurrentBuild").and_then(|b| b.parse().ok()).unwrap_or(0);
        // ProductName still says "Windows 10" on Windows 11: the build number tells them apart.
        let mut name = read("ProductName").unwrap_or_else(|| "Windows".into());
        if build >= 22000 {
            name = name.replacen("Windows 10", "Windows 11", 1);
        }
        let release = read("DisplayVersion").map(|d| format!(" {d}")).unwrap_or_default();
        return format!("{name}{release} (build {build})");
    }
    #[allow(unreachable_code)]
    std::env::consts::OS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn mark(caret: Option<(isize, [i32; 4])>) -> PasteMark {
        PasteMark { window: 10, focus: 20, at: Some(Instant::now()), last: Some('.'), caret }
    }

    #[test]
    fn next_dictation_continues_only_in_place() {
        let caret = Some((20, [100, 40, 101, 58]));
        let here = FocusNow { focus: 20, caret };
        assert!(continues_mark(&mark(caret), 10, &here));
        // Another window, another control in it, or no window known.
        assert!(!continues_mark(&mark(caret), 11, &here));
        assert!(!continues_mark(&mark(caret), 10, &FocusNow { focus: 21, caret }));
        assert!(!continues_mark(&mark(caret), 0, &here));
        // The caret moved since (a click elsewhere, Enter).
        assert!(!continues_mark(&mark(caret), 10, &FocusNow { focus: 20, caret: Some((20, [100, 60, 101, 78])) }));
        // Apps without Windows' caret: only the time rule.
        assert!(continues_mark(&mark(None), 10, &FocusNow { focus: 20, caret: None }));
        assert!(continues_mark(&mark(caret), 10, &FocusNow { focus: 20, caret: None }));
        // Over a minute later, or nothing pasted yet.
        let old = PasteMark { at: Instant::now().checked_sub(JOIN_WINDOW + Duration::from_secs(1)), ..mark(caret) };
        assert!(old.at.is_some());
        assert!(!continues_mark(&old, 10, &here));
        assert!(!continues_mark(&PasteMark::default(), 0, &FocusNow::default()));
    }

    #[test]
    fn replace_undoes_only_where_the_paste_left_off() {
        let caret = Some((20, [100, 40, 101, 58]));
        // Still in place, even minutes later: Ctrl+Z undoes the paste.
        let old = PasteMark { at: Instant::now().checked_sub(JOIN_WINDOW * 3), ..mark(caret) };
        assert!(same_place(&old, 10, &FocusNow { focus: 20, caret }));
        // Typed or clicked since (the caret moved), another control, or nothing pasted there.
        assert!(!same_place(&mark(caret), 10, &FocusNow { focus: 20, caret: Some((20, [130, 40, 131, 58])) }));
        assert!(!same_place(&mark(caret), 10, &FocusNow { focus: 21, caret }));
        assert!(!same_place(&PasteMark::default(), 10, &FocusNow { focus: 0, caret: None }));
    }

    #[test]
    fn clipboard_text_gets_windows_line_breaks() {
        assert_eq!(with_crlf("a\nb\r\nc\n"), "a\r\nb\r\nc\r\n");
        assert_eq!(with_crlf("sem quebra"), "sem quebra");
    }
}
