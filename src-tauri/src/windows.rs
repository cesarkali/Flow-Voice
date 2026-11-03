//! Window management: overlay placement, settings and chat windows.

use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

const MARGIN: f64 = 28.0;

/// The overlay window is driven ONLY through crate::platform's Win32 calls (show, hide,
/// click-through), never through tao/Tauri: tao keeps its own "visible" flag, which is false
/// for a window shown with ShowWindow, and every later tao call on it (set_ignore_cursor_events,
/// hide) re-applies that flag — hiding the overlay whenever the mouse crossed the card, and
/// turning hide() into a no-op that left an invisible window on screen.
pub fn setup_overlay(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        crate::platform::make_non_activating(&w);
        crate::platform::set_click_through(&w, true);
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
    // Test hook: FLOWVOICE_TEST_MONITOR=<index>[,<index>...] forces the monitor of each new
    // appearance in turn (multi-monitor/DPI checks without touching the real mouse).
    if let Ok(list) = std::env::var("FLOWVOICE_TEST_MONITOR") {
        let ids: Vec<usize> = list.split(',').filter_map(|i| i.trim().parse().ok()).collect();
        if !ids.is_empty() {
            let n = TEST_MONITOR_TURN.fetch_add(1, Ordering::SeqCst);
            if let Some(m) = monitors.get(ids[n % ids.len()]) {
                return Some(m.clone());
            }
        }
    }
    if follow { under_focus().or_else(under_mouse) } else { None }
        .or_else(|| w.primary_monitor().ok().flatten())
        .or_else(under_mouse)
}

pub(crate) fn monitor_key(m: &tauri::Monitor) -> (i32, i32) {
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

static TEST_MONITOR_TURN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Test hook (isolated instances only): pretend the mouse is inside/outside the card.
static TEST_POINTER: parking_lot::Mutex<Option<bool>> = parking_lot::Mutex::new(None);

pub fn test_set_pointer(inside: Option<bool>) {
    *TEST_POINTER.lock() = inside;
}

/// Test hook: what the overlay window really is right now.
pub fn test_snapshot(app: &AppHandle) -> serde_json::Value {
    let w = app.get_webview_window("overlay");
    serde_json::json!({
        "native_visible": w.as_ref().map(crate::platform::is_window_visible),
        "rect": w.as_ref().and_then(crate::platform::window_rect),
        "wanted": WANTED.load(Ordering::SeqCst),
        "click_through": w.as_ref().and_then(crate::platform::is_click_through),
        "hitbox": *HITBOX.lock(),
        "anchor": *ANCHOR.lock(),
        "target_rect": *TARGET_RECT.lock(),
        "monitor": *app.state::<AppState>().overlay_monitor.lock(),
        "drag": crate::overlay_drag::test_snapshot(),
    })
}

/// The overlay is meant to be up (see WANTED).
pub(crate) fn is_wanted() -> bool {
    WANTED.load(Ordering::SeqCst)
}

/// The card's rectangle as the page last reported it (logical px relative to the window).
pub(crate) fn hitbox() -> Option<(f64, f64, f64, f64)> {
    *HITBOX.lock()
}

/// Where the window was last put by place_card (physical px).
pub(crate) fn target_rect() -> Option<(i32, i32, i32, i32)> {
    *TARGET_RECT.lock()
}

/// Cancels the pending show-time settle loops (a drag moves the window on purpose).
pub(crate) fn bump_show_gen() {
    SHOW_GEN.fetch_add(1, Ordering::SeqCst);
}

/// Card rectangle reported by the UI (logical px relative to the window); None = nothing visible.
static HITBOX: once_cell::sync::Lazy<parking_lot::Mutex<Option<(f64, f64, f64, f64)>>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));
/// Bumped by every show; a pending delayed hide only runs if no show happened meanwhile.
static SHOW_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The overlay is meant to be up (the native window lingers ~240 ms after a hide for the exit
/// animation, so is_visible alone can't tell a new appearance from a phase change).
static WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Card center, in logical px from the window edge it grows away from, when it sits in its
/// default spot (horizontally centered; the capsule is 56px tall and 22px from the edge, + the
/// window margin 28 in screen space): 22 + 28.
pub(crate) const CARD_EDGE: f64 = 50.0;
/// Keeps the card clear of the work-area edge (capsule half height 28 + 8).
const CARD_INSET_Y: f64 = 36.0;

/// What the overlay page needs to lay the card out inside the window: its offset (logical px)
/// from the default card spot and the direction it grows ("up": anchored near the bottom).
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq)]
pub struct OverlayAnchor {
    pub dx: f64,
    pub dy: f64,
    pub grow: &'static str,
}

static ANCHOR: parking_lot::Mutex<OverlayAnchor> =
    parking_lot::Mutex::new(OverlayAnchor { dx: 0.0, dy: 0.0, grow: "up" });

#[tauri::command]
pub fn get_overlay_anchor() -> OverlayAnchor {
    *ANCHOR.lock()
}

/// Card center (fractions of the work area) of the two built-in positions on monitor `m`.
fn default_center(m: &tauri::Monitor, top: bool) -> (f64, f64) {
    let scale = m.scale_factor();
    let area = m.work_area();
    let h = area.size.height as f64;
    let margin = (MARGIN * scale).trunc();
    let edge = CARD_EDGE * scale;
    // Bottom: window flush `margin` above the work-area bottom, the card CARD_EDGE above the
    // window bottom. Top: mirrored.
    let cy = if top { margin + edge } else { h - margin - edge };
    (0.5, cy / h)
}

/// Positions the fixed-size overlay window so the card's center lands on `center` (fractions
/// 0..1 of monitor `m`'s work area), keeping the WHOLE window inside that work area (a window
/// spilling onto another monitor makes Windows rescale it). When the window can't move further
/// the card shifts inside it instead: the page gets `overlay-anchor` {dx, dy, grow}.
pub fn place_card(app: &AppHandle, m: &tauri::Monitor, center: (f64, f64)) {
    place_card_geom(app, &MonGeom::from(m), center);
}

/// What the placement needs to know about a monitor (physical px). Besides the real ones it can
/// describe the test hook's fake monitor, see `fake_monitor`.
#[derive(Clone, Debug)]
pub struct MonGeom {
    pub name: String,
    pub size: (u32, u32),
    pub pos: (i32, i32),
    /// Work area: x, y, width, height.
    pub work: (i32, i32, u32, u32),
    pub scale: f64,
}

impl From<&tauri::Monitor> for MonGeom {
    fn from(m: &tauri::Monitor) -> Self {
        let (p, s, a) = (m.position(), m.size(), m.work_area());
        MonGeom {
            name: m.name().cloned().unwrap_or_else(|| "?".into()),
            size: (s.width, s.height),
            pos: (p.x, p.y),
            work: (a.position.x, a.position.y, a.size.width, a.size.height),
            scale: m.scale_factor(),
        }
    }
}

impl MonGeom {
    pub fn key(&self) -> (i32, i32) {
        (self.work.0, self.work.1)
    }
}

/// Test hook (isolated instances only, FLOWVOICE_DATA_DIR set): FLOWVOICE_TEST_FAKE_MONITOR=
/// "x,y,w,h,scale" adds a monitor that doesn't exist, so a dragged card can hop to it on a PC with
/// a single screen (the window just lands off-screen there; Windows keeps its real DPI).
pub fn fake_monitor() -> Option<MonGeom> {
    std::env::var_os("FLOWVOICE_DATA_DIR")?;
    let v = std::env::var("FLOWVOICE_TEST_FAKE_MONITOR").ok()?;
    let n: Vec<f64> = v.split(',').filter_map(|i| i.trim().parse().ok()).collect();
    if n.len() != 5 {
        return None;
    }
    Some(MonGeom {
        name: "FAKE".into(),
        size: (n[2] as u32, n[3] as u32),
        pos: (n[0] as i32, n[1] as i32),
        work: (n[0] as i32, n[1] as i32, n[2] as u32, n[3] as u32),
        scale: n[4],
    })
}

/// Every monitor (plus the fake one of the test hook).
pub fn all_monitors(w: &tauri::WebviewWindow) -> Vec<MonGeom> {
    let mut v: Vec<MonGeom> = w.available_monitors().unwrap_or_default().iter().map(MonGeom::from).collect();
    v.extend(fake_monitor());
    v
}

pub fn place_card_geom(app: &AppHandle, m: &MonGeom, center: (f64, f64)) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let scale = m.scale;
    let (ax, ay) = (m.work.0, m.work.1);
    let (aw, ah) = (m.work.2 as f64, m.work.3 as f64);
    let width = (OVERLAY_W * scale).round() as i32;
    let height = (OVERLAY_H * scale).round() as i32;
    let fx = if center.0.is_finite() { center.0.clamp(0.0, 1.0) } else { 0.5 };
    let fy = if center.1.is_finite() { center.1.clamp(0.0, 1.0) } else { 0.5 };
    let inset = CARD_INSET_Y * scale;
    let cx = fx * aw;
    let cy = (fy * ah).clamp(inset.min(ah / 2.0), (ah - inset).max(ah / 2.0));
    // Grows up from the bottom edge when the card is in the lower half, down otherwise.
    let up = cy >= ah / 2.0;
    let edge = CARD_EDGE * scale;
    let def_x = width as f64 / 2.0;
    let def_y = if up { height as f64 - edge } else { edge };
    // Ideal window origin (relative to the work area), then clamped inside it.
    let ideal_x = (cx - def_x + 1e-6).floor();
    let ideal_y = (cy - def_y).round();
    let max_x = (aw - width as f64).max(0.0);
    let max_y = (ah - height as f64).max(0.0);
    let rx = ideal_x.clamp(0.0, max_x);
    let ry = ideal_y.clamp(0.0, max_y);
    let x = ax + rx as i32;
    let y = ay + ry as i32;
    let anchor = OverlayAnchor {
        dx: ((cx - (rx + def_x)) / scale * 100.0).round() / 100.0,
        dy: ((cy - (ry + def_y)) / scale * 100.0).round() / 100.0,
        grow: if up { "up" } else { "down" },
    };
    if !crate::overlay_drag::is_dragging() {
        // (not once per frame of a drag)
        log::info!(
            "overlay on {} ({}x{} @{}x{}, scale {scale}) at {x},{y} {width}x{height} anchor {anchor:?}",
            m.name, m.size.0, m.size.1, m.pos.0, m.pos.1,
        );
    }
    *ANCHOR.lock() = anchor;
    let _ = app.emit_to("overlay", "overlay-anchor", anchor);
    crate::platform::set_bounds_no_activate(&w, x, y, width, height);
    *TARGET_RECT.lock() = Some((x, y, width, height));
}

static PREVIEW: parking_lot::Mutex<Option<(String, Option<crate::settings::OverlayPoint>)>> =
    parking_lot::Mutex::new(None);

/// "Testar" (settings > Janela flutuante): shows the capsule for ~2 s at the position being
/// edited, through the same show/hide paths as a real dictation. Does nothing during one.
#[tauri::command]
pub fn test_overlay_position(app: AppHandle, position: String, point: Option<crate::settings::OverlayPoint>) {
    if crate::pipeline::is_recording(&app) || WANTED.load(Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        *PREVIEW.lock() = Some((position, point));
        show_overlay_for_recording(&app);
        let style = app.state::<AppState>().settings().active_style;
        crate::pipeline::emit_overlay(
            &app,
            serde_json::json!({ "phase": "listening", "mode": "dictate", "style_id": style, "target": null, "live": false }),
        );
        std::thread::sleep(std::time::Duration::from_millis(2200));
        *PREVIEW.lock() = None;
        if !crate::pipeline::is_recording(&app) && crate::pipeline::overlay_phase() == "listening" {
            hide_overlay(&app);
        }
    });
}

/// Places the fixed-size overlay on its pinned monitor: bottom/top-center or the custom point.
fn place_overlay(app: &AppHandle) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    let Some(m) = pinned_monitor(app, &w) else { return };
    let st = app.state::<AppState>().settings();
    // "Testar" in the settings: the position being edited, not the saved one.
    let (position, custom) = PREVIEW.lock().clone().unwrap_or((st.overlay_position.clone(), st.overlay_custom));
    let center = match (position.as_str(), custom) {
        ("custom", Some(p)) => (p.x, p.y),
        ("top", _) => default_center(&m, true),
        _ => default_center(&m, false),
    };
    place_card(app, &m, center);
}

/// Where the overlay window must be (physical px), re-applied while it settles.
static TARGET_RECT: once_cell::sync::Lazy<parking_lot::Mutex<Option<(i32, i32, i32, i32)>>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));

/// Windows rescales a window that lands on a monitor with another DPI (sometimes only after it
/// is shown), which would leave the card off-center or below the screen edge. For a moment after
/// showing, put the window back on its exact rectangle whenever that happens.
pub(crate) fn settle_overlay(app: &AppHandle) {
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

/// Shows the overlay for a later phase of the current dictation (processing, result, error):
/// if it is already up it stays exactly where it is.
pub fn show_overlay(app: &AppHandle) {
    show(app, false);
}

/// Shows the overlay for a NEW recording: it goes to the monitor in use right now, even if the
/// card of the previous dictation is still showing on another screen.
pub fn show_overlay_for_recording(app: &AppHandle) {
    show(app, true);
}

fn show(app: &AppHandle, new_session: bool) {
    let Some(w) = app.get_webview_window("overlay") else { return };
    SHOW_GEN.fetch_add(1, Ordering::SeqCst);
    let was_up = WANTED.swap(true, Ordering::SeqCst);
    let visible = crate::platform::is_window_visible(&w);
    if visible && was_up && !new_session {
        return; // phase change: it stays where it is (also where the user dragged it)
    }
    // A new appearance: a drag in progress ends here and the card goes back to its default spot.
    crate::overlay_drag::cancel(app);
    let moved = crate::overlay_drag::take_moved();
    let Some(target) = target_monitor(app, &w) else { return };
    let key = monitor_key(&target);
    let previous = app.state::<AppState>().overlay_monitor.lock().replace(key);
    if visible && previous == Some(key) && !moved {
        // Same screen (a new recording right after the last card, or a show during the exit
        // animation): the card simply morphs into its new phase. Dragged away from its default
        // spot, it takes the full path below (blank, move back, show) instead.
        return;
    }
    if visible {
        // Another screen: blank the card at once (no exit animation) and let one frame paint
        // before hiding, so Windows never re-shows the old card on the new monitor.
        crate::pipeline::set_overlay_phase("hidden");
        let _ = app.emit_to("overlay", "overlay-state", serde_json::json!({ "phase": "hidden", "instant": true }));
        std::thread::sleep(std::time::Duration::from_millis(60));
        crate::platform::hide_window(&w);
    }
    let dpi_change = w.scale_factor().map(|s| (s - target.scale_factor()).abs() > 0.01).unwrap_or(false);
    place_overlay(app);
    if dpi_change {
        // The webview re-rasterizes for the new scale; showing it right away would flash one
        // frame at the old scale.
        std::thread::sleep(std::time::Duration::from_millis(90));
    }
    // Click-through until the UI reports where the card is.
    crate::platform::set_click_through(&w, true);
    crate::platform::show_without_focus(&w);
    settle_overlay(app);
}

pub fn hide_overlay(app: &AppHandle) {
    crate::hotkeys::register_escape(app, false);
    let st = app.state::<AppState>();
    st.overlay_hold.store(false, Ordering::SeqCst);
    st.overlay_gen.fetch_add(1, Ordering::SeqCst);
    WANTED.store(false, Ordering::SeqCst);
    *HITBOX.lock() = None;
    crate::overlay_drag::cancel(app);
    crate::pipeline::set_overlay_phase("hidden");
    let _ = app.emit_to("overlay", "overlay-state", serde_json::json!({ "phase": "hidden" }));
    // Let the UI play its exit animation before the native window disappears.
    let app = app.clone();
    let gen = SHOW_GEN.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(240));
        let st = app.state::<AppState>();
        if SHOW_GEN.load(Ordering::SeqCst) == gen && st.session.lock().is_none() {
            if let Some(w) = app.get_webview_window("overlay") {
                crate::platform::set_click_through(&w, true);
                crate::platform::hide_window(&w);
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
            if !up || !crate::platform::is_window_visible(&w) {
                if inside_prev == Some(true) {
                    crate::platform::set_click_through(&w, true);
                }
                inside_prev = None;
                continue;
            }
            // While the card is being dragged the window keeps catching the mouse (the card trails
            // the cursor by a frame).
            let forced = crate::overlay_drag::is_dragging().then_some(true).or(*TEST_POINTER.lock());
            let inside = forced.or_else(|| {
                let hb = (*HITBOX.lock())?;
                let cur = app.cursor_position().ok()?;
                let pos = w.outer_position().ok()?;
                let scale = w.scale_factor().ok()?;
                let (x, y) = ((cur.x - pos.x as f64) / scale, (cur.y - pos.y as f64) / scale);
                Some(x >= hb.0 && y >= hb.1 && x <= hb.0 + hb.2 && y <= hb.1 + hb.3)
            })
            .unwrap_or(false);
            if inside_prev != Some(inside) {
                inside_prev = Some(inside);
                crate::platform::set_click_through(&w, !inside);
                let _ = app.emit_to("overlay", "overlay-pointer", serde_json::json!({ "inside": inside }));
            }
        }
    });
}

/// The capsule's glass on the settings window too (`window_glass`): Windows 11's acrylic (the
/// system's own blur of what's behind) under a page whose background turns translucent. The
/// window is transparent (tauri.conf.json); solid, the page paints it opaque as before. The
/// window's dark/light mode follows the app's theme so the acrylic's own tint matches it.
pub fn apply_window_glass(app: &AppHandle, s: &crate::settings::Settings) {
    use tauri::window::{Effect, EffectsBuilder};
    let Some(w) = app.get_webview_window("main") else { return };
    let theme = match s.theme.as_str() {
        "dark" => Some(tauri::Theme::Dark),
        "light" => Some(tauri::Theme::Light),
        _ => None,
    };
    let _ = w.set_theme(theme);
    let res = if s.window_glass && s.overlay_glass != "solid" {
        w.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
    } else {
        w.set_effects(None)
    };
    if let Err(e) = res {
        log::warn!("window glass: {e}");
    }
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
