//! Window management: overlay placement, settings and chat windows.

use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

const MARGIN: f64 = 28.0;

pub fn setup_overlay(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        crate::platform::make_non_activating(&w);
        let _ = w.set_ignore_cursor_events(false);
    }
}

/// Monitor the overlay should use right now: the one under the mouse (follow mode) or the primary.
fn target_monitor(app: &AppHandle, w: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    let follow = app.state::<AppState>().settings().overlay_follow_mouse;
    let under_mouse = || {
        app.cursor_position()
            .ok()
            .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
    };
    if follow { under_mouse() } else { None }
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

/// Places the overlay bottom/top-center on its pinned monitor. `size` is the logical size just
/// requested, since outer_size() may still report the previous one right after set_size().
fn place_overlay(app: &AppHandle, size: Option<(f64, f64)>) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let top = app.state::<AppState>().settings().overlay_position == "top";
    let Some(m) = pinned_monitor(app, &w) else { return };
    let scale = m.scale_factor();
    let area = m.work_area();
    let (width, height) = match size {
        Some((lw, lh)) => ((lw * scale).round() as i32, (lh * scale).round() as i32),
        None => match w.outer_size() {
            Ok(sz) => (sz.width as i32, sz.height as i32),
            Err(_) => return,
        },
    };
    let x = area.position.x + (area.size.width as i32 - width) / 2;
    let margin = (MARGIN * scale) as i32;
    let y = if top {
        area.position.y + margin
    } else {
        area.position.y + area.size.height as i32 - height - margin
    };
    if size.is_some() {
        // Move + resize in one native call so the anchored edge doesn't flicker for a frame.
        crate::platform::set_bounds_no_activate(&w, x, y, width, height);
    } else {
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

pub fn show_overlay(app: &AppHandle) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let Some(target) = target_monitor(app, &w) else { return };
    let key = monitor_key(&target);
    let visible = w.is_visible().unwrap_or(false);
    let previous = app.state::<AppState>().overlay_monitor.lock().replace(key);
    if visible && previous.is_some() && previous != Some(key) {
        // Changing screens while visible: fade out, move, fade back in (no hard jump).
        let _ = app.emit_to("overlay", "overlay-move", serde_json::json!({ "phase": "out" }));
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(140));
            place_overlay(&app, None);
            let _ = app.emit_to("overlay", "overlay-move", serde_json::json!({ "phase": "in" }));
        });
        return;
    }
    place_overlay(app, None);
    if !visible {
        crate::platform::show_without_focus(&w);
    }
}

pub fn hide_overlay(app: &AppHandle) {
    crate::hotkeys::register_escape(app, false);
    let st = app.state::<AppState>();
    st.overlay_hold.store(false, Ordering::SeqCst);
    st.overlay_gen.fetch_add(1, Ordering::SeqCst);
    let _ = app.emit_to("overlay", "overlay-state", serde_json::json!({ "phase": "hidden" }));
    // Let the UI play its exit animation before the native window disappears.
    let app = app.clone();
    let gen = st.overlay_gen.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(240));
        let st = app.state::<AppState>();
        if st.overlay_gen.load(Ordering::SeqCst) == gen && st.session.lock().is_none() {
            if let Some(w) = app.get_webview_window("overlay") {
                let _ = w.hide();
            }
        }
    });
}

/// The UI reports its content size so the transparent window never blocks clicks around it.
pub fn resize_overlay(app: &AppHandle, width: f64, height: f64) {
    let width = width.clamp(120.0, 760.0).ceil();
    let height = height.clamp(40.0, 560.0).ceil();
    place_overlay(app, Some((width, height)));
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
