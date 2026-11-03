//! Dragging the dictation overlay while it is recording ("Arrastar a janela flutuante").
//!
//! The page tells Rust a primary-button press landed on the card's body (`overlay_drag_arm`); from
//! then on a thread polls the REAL cursor and button (~60 Hz) until the button goes up. Once the
//! cursor has moved a few pixels the card follows it: on every step the card's anchor point (the
//! capsule center, see windows::place_card) is put under the cursor — on the monitor the cursor
//! is on, so the fixed-size window always sits inside ONE monitor's work area (a window spanning
//! two monitors of different DPI is what Windows rescales). Crossing to another monitor hops the
//! card there: it fades out, the window moves (at the new DPI) and it fades back in.
//!
//! The position holds for the rest of that dictation (phase changes never move the window); a new
//! recording puts the card back on its default spot (windows::show asks `take_moved`).
//!
//! Safety: every way the dictation can end or restart mid-drag (hide, new session, setting turned
//! off, button released anywhere, even outside the window) ends the loop; `cancel` is serialized
//! with the loop's placements so no late move can land after the default placement.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::settings::OverlayPoint;
use crate::state::AppState;
use crate::windows::{CARD_EDGE, OVERLAY_H, OVERLAY_W};

/// Pointer travel (logical px) before a press becomes a drag: below it, it stays a click.
const THRESHOLD: f64 = 4.0;
/// Capsule half height: the card's anchor sits this far from the edge the card grows away from.
const CAP_HALF: f64 = 28.0;
/// Keeps the dragged card this far (logical px) from the work-area edge.
const EDGE_GAP: f64 = 8.0;
/// The card fades out this long before the window hops to another monitor (page: moveCard "out").
const HOP_FADE_MS: u64 = 130;

/// Bumped by `arm` and `cancel`: a loop only acts while its own generation is current.
static GEN: AtomicU64 = AtomicU64::new(0);
/// A loop is running (button down after a press on the card).
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// The press turned into a real drag (moved past the threshold).
static DRAGGING: AtomicBool = AtomicBool::new(false);
/// The window was moved off its default spot during this dictation.
static MOVED: AtomicBool = AtomicBool::new(false);
/// A monitor hop's fade-out was sent and its fade-in was not yet.
static HOP_OPEN: AtomicBool = AtomicBool::new(false);
/// Placements by the loop and `cancel` exclude each other.
static PLACE: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
/// Where the card's anchor was last dropped (fractions of that monitor's work area).
static LAST_DROP: parking_lot::Mutex<Option<(f64, f64)>> = parking_lot::Mutex::new(None);
/// Test hook (isolated instances only): a fake cursor (x, y, primary button down) in place of the
/// real one, so tests never touch the user's mouse.
static TEST_CURSOR: parking_lot::Mutex<Option<(i32, i32, bool)>> = parking_lot::Mutex::new(None);

pub fn is_dragging() -> bool {
    DRAGGING.load(Ordering::SeqCst)
}

/// The window was dragged since it was last placed on its default spot; resets the flag.
pub fn take_moved() -> bool {
    MOVED.swap(false, Ordering::SeqCst)
}

pub fn test_snapshot() -> serde_json::Value {
    serde_json::json!({
        "active": ACTIVE.load(Ordering::SeqCst),
        "dragging": DRAGGING.load(Ordering::SeqCst),
        "moved": MOVED.load(Ordering::SeqCst),
        "last_drop": *LAST_DROP.lock(),
    })
}

fn read_cursor() -> Option<(i32, i32, bool)> {
    if let Some(t) = *TEST_CURSOR.lock() {
        return Some(t);
    }
    let (x, y) = crate::platform::cursor_pos()?;
    Some((x, y, crate::platform::primary_button_down()))
}

/// Ends any drag at once (the dictation hid or restarted, the setting went off). After it returns
/// no late step of the old loop can move the window.
pub fn cancel(app: &AppHandle) {
    // Not an unbounded lock: this may be called from a thread the loop's window calls wait on.
    let guard = PLACE.try_lock_for(Duration::from_millis(100));
    GEN.fetch_add(1, Ordering::SeqCst);
    drop(guard);
    end_state(app);
}

fn end_state(app: &AppHandle) {
    let was_dragging = DRAGGING.swap(false, Ordering::SeqCst);
    let was_active = ACTIVE.swap(false, Ordering::SeqCst);
    if HOP_OPEN.swap(false, Ordering::SeqCst) {
        let _ = app.emit_to("overlay", "overlay-move", serde_json::json!({ "phase": "in" }));
    }
    if was_dragging || was_active {
        let _ = app.emit_to("overlay", "overlay-drag", serde_json::json!({ "active": false }));
    }
}

#[tauri::command]
pub fn get_last_overlay_drag() -> Option<OverlayPoint> {
    (*LAST_DROP.lock()).map(|(x, y)| OverlayPoint { x, y })
}

/// A press landed on the card's body (not on a button). Starts the drag loop when dragging is
/// allowed right now; the page treats the press as a plain click otherwise.
#[tauri::command]
pub fn overlay_drag_arm(app: AppHandle) -> bool {
    arm(&app)
}

/// Test hook, isolated instances only (like test_probe): "cursor" sets the fake cursor, anything
/// else (or "cursor_off") goes back to the real one.
#[tauri::command]
pub fn test_drag(app: AppHandle, action: String, x: Option<i32>, y: Option<i32>, down: Option<bool>) -> Result<serde_json::Value, String> {
    if std::env::var_os("FLOWVOICE_DATA_DIR").is_none() {
        return Err("indisponível".into());
    }
    *TEST_CURSOR.lock() = match action.as_str() {
        "cursor" => Some((x.unwrap_or(0), y.unwrap_or(0), down.unwrap_or(false))),
        "snapshot" => *TEST_CURSOR.lock(),
        _ => None,
    };
    let mut v = test_snapshot();
    let monitors = app.get_webview_window("overlay").map(|w| crate::windows::all_monitors(&w)).unwrap_or_default();
    v["monitors"] = monitors
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name,
                "rect": [m.pos.0, m.pos.1, m.size.0, m.size.1],
                "work": [m.work.0, m.work.1, m.work.2, m.work.3],
                "scale": m.scale,
                "key": m.key(),
            })
        })
        .collect();
    Ok(v)
}

fn arm(app: &AppHandle) -> bool {
    if !app.state::<AppState>().settings().overlay_drag {
        return false;
    }
    if !crate::windows::is_wanted() || crate::pipeline::overlay_phase() != "listening" {
        return false;
    }
    let Some(w) = app.get_webview_window("overlay") else { return false };
    if !crate::platform::is_window_visible(&w) {
        return false;
    }
    let Some((x, y, down)) = read_cursor() else { return false };
    if !down {
        return false; // already released: it was a click
    }
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return false; // one at a time
    }
    DRAGGING.store(false, Ordering::SeqCst);
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || run(app, gen, (x, y)));
    true
}

struct Mon {
    m: crate::windows::MonGeom,
    key: (i32, i32),
    /// Full rectangle (physical px).
    rect: (i32, i32, i32, i32),
}

/// Index of the rectangle nearest to `p` (the one containing it, or the closest across a gap).
fn nearest(rects: &[(i32, i32, i32, i32)], p: (i32, i32)) -> Option<usize> {
    let dist = |&(x, y, w, h): &(i32, i32, i32, i32)| {
        let dx = (x - p.0).max(0).max(p.0 - (x + w - 1)) as i64;
        let dy = (y - p.1).max(0).max(p.1 - (y + h - 1)) as i64;
        dx * dx + dy * dy
    };
    rects.iter().enumerate().min_by_key(|(_, r)| dist(r)).map(|(i, _)| i)
}

/// Fractions (of the work area) at which to place the card's anchor so it sits under the cursor,
/// keeping the whole card (`card` = logical width/height) inside the work area. `off` is the
/// cursor's offset from the anchor in logical px (kept from when the drag started).
fn anchor_fraction(cursor: (i32, i32), off: (f64, f64), area: (i32, i32, u32, u32), scale: f64, card: Option<(f64, f64)>) -> (f64, f64) {
    let (aw, ah) = (area.2 as f64 / scale, area.3 as f64 / scale); // logical
    let ax = (cursor.0 - area.0) as f64 / scale - off.0;
    let ay = (cursor.1 - area.1) as f64 / scale - off.1;
    let (cw, ch) = card.unwrap_or((0.0, 2.0 * CAP_HALF));
    let ch = ch.max(2.0 * CAP_HALF);
    let hw = (cw / 2.0 + EDGE_GAP).min(aw / 2.0);
    let ax = ax.clamp(hw, (aw - hw).max(hw));
    // The card grows up from the anchor's capsule when it sits in the lower half, down otherwise
    // (same rule as place_card).
    let up = ay >= ah / 2.0;
    let (lo, hi) = if up {
        (ch - CAP_HALF + EDGE_GAP, ah - CAP_HALF - EDGE_GAP)
    } else {
        (CAP_HALF + EDGE_GAP, ah - ch + CAP_HALF - EDGE_GAP)
    };
    let ay = if lo <= hi { ay.clamp(lo, hi) } else { ay.clamp(CAP_HALF + EDGE_GAP, (ah - CAP_HALF - EDGE_GAP).max(CAP_HALF + EDGE_GAP)) };
    ((ax / aw).clamp(0.0, 1.0), (ay / ah).clamp(0.0, 1.0))
}

fn run(app: AppHandle, gen: u64, start: (i32, i32)) {
    let Some(w) = app.get_webview_window("overlay") else {
        end_if_current(&app, gen);
        return;
    };
    let mons: Vec<Mon> = crate::windows::all_monitors(&w)
        .into_iter()
        .map(|m| Mon { key: m.key(), rect: (m.pos.0, m.pos.1, m.size.0 as i32, m.size.1 as i32), m })
        .collect();
    let rects: Vec<_> = mons.iter().map(|m| m.rect).collect();
    let mut offset: Option<(f64, f64)> = None;
    let mut last: Option<((i32, i32), (f64, f64))> = None; // (monitor key, fraction) last placed
    loop {
        std::thread::sleep(Duration::from_millis(16));
        if GEN.load(Ordering::SeqCst) != gen {
            return; // cancel() already cleaned up
        }
        let Some((cx, cy, down)) = read_cursor() else { break };
        let settings_on = app.state::<AppState>().settings().overlay_drag;
        if !down || !settings_on || !crate::windows::is_wanted() || !crate::platform::is_window_visible(&w) {
            break;
        }
        let Some((wx, wy, ww, _)) = crate::platform::window_rect(&w) else { break };
        let scale = ww as f64 / OVERLAY_W; // the window's real scale (logical size is fixed)
        if offset.is_none() {
            let moved = ((cx - start.0) as f64).hypot((cy - start.1) as f64);
            if moved < THRESHOLD * scale {
                continue;
            }
            // Where the card's anchor point is now (same definition as place_card).
            let a = crate::windows::get_overlay_anchor();
            let ax = wx as f64 + (OVERLAY_W / 2.0 + a.dx) * scale;
            let ay = wy as f64 + ((if a.grow == "up" { OVERLAY_H - CARD_EDGE } else { CARD_EDGE }) + a.dy) * scale;
            offset = Some(((start.0 as f64 - ax) / scale, (start.1 as f64 - ay) / scale));
            crate::windows::bump_show_gen(); // the show-time settle loop must not fight the drag
            DRAGGING.store(true, Ordering::SeqCst);
            MOVED.store(true, Ordering::SeqCst);
            let _ = app.emit_to("overlay", "overlay-drag", serde_json::json!({ "active": true }));
        }
        let off = offset.unwrap();
        let Some(i) = nearest(&rects, (cx, cy)) else { break };
        let mon = &mons[i];
        let cur_key = *app.state::<AppState>().overlay_monitor.lock();
        if cur_key != Some(mon.key) {
            if !hop(&app, &w, gen, mon, off, (cx, cy)) {
                return;
            }
            last = None;
            continue;
        }
        let card = crate::windows::hitbox().map(|h| (h.2, h.3));
        let frac = anchor_fraction((cx, cy), off, mon.m.work, mon.m.scale, card);
        // Skip a step only when nothing changed AND the window really is where it was put
        // (Windows may rescale it behind our back, a settle loop may have restored a stale rect).
        let in_place = crate::windows::target_rect() == crate::platform::window_rect(&w);
        if last == Some((mon.key, frac)) && in_place {
            continue;
        }
        let guard = PLACE.lock();
        if GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        crate::windows::place_card_geom(&app, &mon.m, frac);
        drop(guard);
        crate::glass::follow(&app);
        last = Some((mon.key, frac));
        *LAST_DROP.lock() = Some(frac);
    }
    end_if_current(&app, gen);
}

/// The cursor crossed to another monitor: fade the card out, move the window there (at that
/// monitor's DPI), fade it back in. False when the drag was cancelled meanwhile.
fn hop(app: &AppHandle, w: &tauri::WebviewWindow, gen: u64, mon: &Mon, off: (f64, f64), cur: (i32, i32)) -> bool {
    if GEN.load(Ordering::SeqCst) != gen {
        return false;
    }
    HOP_OPEN.store(true, Ordering::SeqCst);
    let _ = app.emit_to("overlay", "overlay-move", serde_json::json!({ "phase": "out" }));
    std::thread::sleep(Duration::from_millis(HOP_FADE_MS));
    let dpi_change = w.scale_factor().map(|s| (s - mon.m.scale).abs() > 0.01).unwrap_or(false);
    {
        let _guard = PLACE.lock();
        if GEN.load(Ordering::SeqCst) != gen {
            return hop_abandoned(app);
        }
        // Where the cursor is NOW (it kept moving during the fade).
        let (cx, cy) = read_cursor().map(|c| (c.0, c.1)).unwrap_or(cur);
        let card = crate::windows::hitbox().map(|h| (h.2, h.3));
        let frac = anchor_fraction((cx, cy), off, mon.m.work, mon.m.scale, card);
        *app.state::<AppState>().overlay_monitor.lock() = Some(mon.key);
        crate::windows::place_card_geom(app, &mon.m, frac);
        *LAST_DROP.lock() = Some(frac);
    }
    crate::glass::follow(app);
    if dpi_change {
        // The webview re-rasterizes for the new scale while the card is still invisible.
        std::thread::sleep(Duration::from_millis(90));
    }
    if GEN.load(Ordering::SeqCst) != gen {
        return hop_abandoned(app);
    }
    crate::windows::settle_overlay(app);
    HOP_OPEN.store(false, Ordering::SeqCst);
    let _ = app.emit_to("overlay", "overlay-move", serde_json::json!({ "phase": "in" }));
    true
}

/// The drag was cancelled while the card was faded out for a hop: never leave it invisible.
fn hop_abandoned(app: &AppHandle) -> bool {
    HOP_OPEN.store(false, Ordering::SeqCst);
    let _ = app.emit_to("overlay", "overlay-move", serde_json::json!({ "phase": "in" }));
    false
}

fn end_if_current(app: &AppHandle, gen: u64) {
    if GEN.load(Ordering::SeqCst) == gen {
        end_state(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: (i32, i32, u32, u32) = (0, 0, 1920, 1000);

    #[test]
    fn anchor_follows_the_cursor_minus_the_grab_offset() {
        let (fx, fy) = anchor_fraction((960, 500), (0.0, 0.0), AREA, 1.0, Some((388.0, 56.0)));
        assert!((fx - 0.5).abs() < 1e-9 && (fy - 0.5).abs() < 1e-9);
        let (fx, fy) = anchor_fraction((960, 500), (10.0, -20.0), AREA, 1.0, Some((388.0, 56.0)));
        assert!((fx - 950.0 / 1920.0).abs() < 1e-9 && (fy - 520.0 / 1000.0).abs() < 1e-9);
    }

    #[test]
    fn card_never_leaves_the_work_area() {
        let card = Some((388.0, 56.0));
        // Far top-left: the capsule's center stops half a card + gap from the corner.
        let (fx, fy) = anchor_fraction((-500, -500), (0.0, 0.0), AREA, 1.0, card);
        assert!((fx * 1920.0 - (194.0 + EDGE_GAP)).abs() < 1e-6, "{fx}");
        assert!((fy * 1000.0 - (CAP_HALF + EDGE_GAP)).abs() < 1e-6, "{fy}");
        // Far bottom-right.
        let (fx, fy) = anchor_fraction((9000, 9000), (0.0, 0.0), AREA, 1.0, card);
        assert!((fx * 1920.0 - (1920.0 - 194.0 - EDGE_GAP)).abs() < 1e-6);
        assert!((fy * 1000.0 - (1000.0 - CAP_HALF - EDGE_GAP)).abs() < 1e-6);
    }

    #[test]
    fn tall_card_keeps_its_far_edge_on_screen() {
        // A 200px card growing up in a 300px-high work area: its top stays 8px under the edge.
        let (_, fy) = anchor_fraction((960, 160), (0.0, 0.0), (0, 0, 1920, 300), 1.0, Some((500.0, 200.0)));
        assert!((fy * 300.0 - (200.0 - CAP_HALF + EDGE_GAP)).abs() < 1e-6, "{fy}");
    }

    #[test]
    fn scaled_monitor_uses_logical_offsets() {
        // 125%: a cursor 125 physical px right of the work-area origin is 100 logical px.
        let (fx, _) = anchor_fraction((125, 500), (0.0, 0.0), (0, 0, 2400, 1250), 1.25, Some((0.0, 56.0)));
        assert!((fx * 1920.0 - 100.0).abs() < 1e-6, "{fx}");
    }

    #[test]
    fn nearest_monitor_picks_the_one_under_the_cursor_or_the_closest() {
        // Laid out like the user's two screens: 1 on top, 2 right below.
        let mons = [(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
        assert_eq!(nearest(&mons, (500, 500)), Some(0));
        assert_eq!(nearest(&mons, (500, 1500)), Some(1));
        assert_eq!(nearest(&mons, (-300, 1090)), Some(1));
        assert_eq!(nearest(&mons, (5000, 5000)), Some(1));
    }
}
