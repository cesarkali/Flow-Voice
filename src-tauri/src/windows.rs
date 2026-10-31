//! Window management: overlay placement, settings and chat windows.

use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

const MARGIN: f64 = 28.0;

pub fn setup_overlay(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        crate::platform::make_non_activating(&w);
        let _ = w.set_ignore_cursor_events(true);
    }
}

/// Monitor for a NEW overlay. "Monitor em uso" mode (overlay_follow_mouse): the screen of the
/// focused window (where the text will be pasted), or the one under the mouse when the focus is
/// the desktop/taskbar or FlowVoice itself; otherwise the primary monitor. Chosen once per
/// appearance: the overlay never follows the mouse or the focus while it is up.
fn target_monitor(app: &AppHandle, w: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    let follow = app.state::<AppState>().settings().overlay_follow_mouse;
    let monitors = w.available_monitors().unwrap_or_default();
    let under_mouse = || {
        app.cursor_position()
            .ok()
            .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
    };
    let under_focus = || {
        let (x, y) = crate::platform::foreground_monitor_origin(app)?;
        monitors.iter().find(|m| m.position().x == x && m.position().y == y).cloned()
    };
    // Test hook: FLOWVOICE_TEST_MONITOR=<index> forces a monitor (multi-monitor/DPI checks).
    if let Some(m) = std::env::var("FLOWVOICE_TEST_MONITOR").ok().and_then(|i| i.parse::<usize>().ok()).and_then(|i| monitors.get(i)) {
        return Some(m.clone());
    }
    if follow { under_focus().or_else(under_mouse) } else { None }
        .or_else(|| w.primary_monitor().ok().flatten())
        .or_else(under_mouse)
}

fn monitor_key(m: &tauri::Monitor) -> (i32, i32) {
    let p = m.work_area().position;
    (p.x, p.y)
}

/// The monitor the overlay is pinned to (chosen when it was shown), so resizes never make it
/// jump to another screen mid-dictation.
fn pinned_monitor(app: &AppHandle, w: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    let key = *app.state::<AppState>().overlay_monitor.lock();
    key.and_then(|k| w.available_monitors().ok()?.into_iter().find(|m| monitor_key(m) == k))
        .or_else(|| target_monitor(app, w))
}

/// The overlay window never resizes (resizing a transparent WebView2 window flickers): it is a
/// fixed transparent canvas anchored to the screen edge, click-through except over the card.
pub const OVERLAY_W: f64 = 760.0;
pub const OVERLAY_H: f64 = 560.0;

/// Card rectangle reported by the UI (logical px relative to the window); None = nothing visible.
static HITBOX: once_cell::sync::Lazy<parking_lot::Mutex<Option<(f64, f64, f64, f64)>>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));
/// Bumped by every show; a pending delayed hide only runs if no show happened meanwhile.
static SHOW_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The overlay is meant to be up (the native window lingers ~240 ms after a hide for the exit
/// animation, so is_visible alone can't tell a new appearance from a phase change).
static WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Places the fixed-size overlay bottom/top-center on its pinned monitor.
fn place_overlay(app: &AppHandle) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let top = app.state::<AppState>().settings().overlay_position == "top";
    let Some(m) = pinned_monitor(app, &w) else { return };
    let scale = m.scale_factor();
    let area = m.work_area();
    let width = (OVERLAY_W * scale).round() as i32;
    let height = (OVERLAY_H * scale).round() as i32;
    let x = area.position.x + (area.size.width as i32 - width) / 2;
    let margin = (MARGIN * scale) as i32;
    let y = if top {
        area.position.y + margin
    } else {
        area.position.y + area.size.height as i32 - height - margin
    };
    log::info!(
        "overlay on {} ({}x{} @{}x{}, scale {scale}) at {x},{y} {width}x{height}",
        m.name().map(String::as_str).unwrap_or("?"),
        m.size().width,
        m.size().height,
        m.position().x,
        m.position().y,
    );
    crate::platform::set_bounds_no_activate(&w, x, y, width, height);
    *TARGET_RECT.lock() = Some((x, y, width, height));
}

/// Where the overlay window must be (physical px), re-applied while it settles.
static TARGET_RECT: once_cell::sync::Lazy<parking_lot::Mutex<Option<(i32, i32, i32, i32)>>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));

/// Windows rescales a window that lands on a monitor with another DPI (sometimes only after it
/// is shown), which would leave the card off-center or below the screen edge. For a moment after
/// showing, put the window back on its exact rectangle whenever that happens.
fn settle_overlay(app: &AppHandle) {
    let app = app.clone();
    let gen = SHOW_GEN.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        for _ in 0..8 {
            std::thread::sleep(std::time::Duration::from_millis(40));
            if SHOW_GEN.load(Ordering::SeqCst) != gen {
                return;
            }
            let Some(w) = app.get_webview_window("overlay") else { return };
            let Some((x, y, width, height)) = *TARGET_RECT.lock() else { return };
            let now = crate::platform::window_rect(&w);
            if now.is_some_and(|r| r != (x, y, width, height)) {
                log::info!("overlay rect drifted to {now:?}, restoring {x},{y} {width}x{height}");
                crate::platform::set_bounds_no_activate(&w, x, y, width, height);
            }
        }
    });
}

pub fn show_overlay(app: &AppHandle) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    SHOW_GEN.fetch_add(1, Ordering::SeqCst);
    let was_up = WANTED.swap(true, Ordering::SeqCst);
    let visible = w.is_visible().unwrap_or(false);
    if visible && was_up {
        return; // already showing (phase change): it stays on its monitor
    }
    // New appearance (or a show during the exit animation of the previous one): pick the
    // monitor now and keep it until the overlay hides.
    let Some(target) = target_monitor(app, &w) else { return };
    *app.state::<AppState>().overlay_monitor.lock() = Some(monitor_key(&target));
    place_overlay(app);
    if !visible {
        // Click-through until the UI reports where the card is.
        let _ = w.set_ignore_cursor_events(true);
        crate::platform::show_without_focus(&w);
    }
    settle_overlay(app);
}

pub fn hide_overlay(app: &AppHandle) {
    crate::hotkeys::register_escape(app, false);
    let st = app.state::<AppState>();
    st.overlay_hold.store(false, Ordering::SeqCst);
    st.overlay_gen.fetch_add(1, Ordering::SeqCst);
    WANTED.store(false, Ordering::SeqCst);
    *HITBOX.lock() = None;
    let _ = app.emit_to("overlay", "overlay-state", serde_json::json!({ "phase": "hidden" }));
    // Let the UI play its exit animation before the native window disappears.
    let app = app.clone();
    let gen = SHOW_GEN.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(240));
        let st = app.state::<AppState>();
        if SHOW_GEN.load(Ordering::SeqCst) == gen && st.session.lock().is_none() {
            if let Some(w) = app.get_webview_window("overlay") {
                let _ = w.set_ignore_cursor_events(true);
                let _ = w.hide();
            }
        }
    });
}

pub fn set_overlay_hitbox(rect: Option<(f64, f64, f64, f64)>) {
    *HITBOX.lock() = rect.filter(|r| r.2 > 0.0 && r.3 > 0.0);
}

/// Makes the overlay catch the mouse only over the card and tells the UI when the pointer
/// enters/leaves it (DOM hover is unreliable on a click-through window).
pub fn start_pointer_watcher(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut inside_prev: Option<bool> = None;
        loop {
            // ~60 Hz while the overlay is up (hover follows the card's morph), almost idle otherwise.
            let up = WANTED.load(Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(if up { 16 } else { 250 }));
            let Some(w) = app.get_webview_window("overlay") else { continue };
            if !up || !w.is_visible().unwrap_or(false) {
                if inside_prev == Some(true) {
                    let _ = w.set_ignore_cursor_events(true);
                }
                inside_prev = None;
                continue;
            }
            let inside = (|| {
                let hb = (*HITBOX.lock())?;
                let cur = app.cursor_position().ok()?;
                let pos = w.outer_position().ok()?;
                let scale = w.scale_factor().ok()?;
                let (x, y) = ((cur.x - pos.x as f64) / scale, (cur.y - pos.y as f64) / scale);
                Some(x >= hb.0 && y >= hb.1 && x <= hb.0 + hb.2 && y <= hb.1 + hb.3)
            })()
            .unwrap_or(false);
            if inside_prev != Some(inside) {
                inside_prev = Some(inside);
                let _ = w.set_ignore_cursor_events(!inside);
                let _ = app.emit_to("overlay", "overlay-pointer", serde_json::json!({ "inside": inside }));
            }
        }
    });
}

pub fn open_settings(app: &AppHandle, page: Option<String>) {
    let Some(w) = app.get_webview_window("main") else { return };
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
    if let Some(page) = page {
        let _ = app.emit_to("main", "navigate", serde_json::json!({ "page": page }));
    }
}

pub fn open_chat(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("chat") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "chat", WebviewUrl::App("chat.html".into()))
        .title("FlowVoice — Assistente")
        .inner_size(460.0, 620.0)
        .min_inner_size(360.0, 420.0)
        .decorations(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .build();
    if let Err(e) = built {
        log::error!("chat window: {e}");
    }
}
