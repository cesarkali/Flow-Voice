//! Global shortcuts: toggle or push-to-talk, plus Esc to cancel while recording.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::pipeline;
use crate::state::{AppState, RecordMode};

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Record(RecordMode),
    PasteLast,
    Cancel,
}

static BINDINGS: Lazy<Mutex<Vec<(Shortcut, Action)>>> = Lazy::new(|| Mutex::new(vec![]));

pub fn parse(accelerator: &str) -> Result<Shortcut> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| anyhow!("atalho inválido \"{accelerator}\": {e}"))
}

pub fn handler(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    let action = BINDINGS.lock().iter().find(|(s, _)| s == shortcut).map(|(_, a)| *a);
    let Some(action) = action else { return };
    log::info!("hotkey {} {:?}", shortcut.into_string(), event.state());
    let hold = app.state::<AppState>().settings.read().hotkey_mode == "hold";
    match (action, event.state()) {
        (Action::Cancel, ShortcutState::Pressed) => {
            if pipeline::is_recording(app) {
                pipeline::cancel(app);
            } else if pipeline::is_busy(app) && matches!(pipeline::overlay_phase().as_str(), "transcribing" | "refining") {
                pipeline::cancel_processing(app);
            } else {
                crate::windows::hide_overlay(app);
            }
        }
        (Action::PasteLast, ShortcutState::Pressed) => {
            // Into whatever has focus now (not where the dictation originally went).
            pipeline::paste_last(app, crate::platform::foreground_window());
        }
        (Action::Record(mode), ShortcutState::Pressed) => {
            if hold {
                if !pipeline::is_recording(app) {
                    pipeline::start_with(app, mode, true);
                }
            } else {
                pipeline::toggle(app, mode);
            }
        }
        (Action::Record(_), ShortcutState::Released) if hold => pipeline::stop(app),
        _ => {}
    }
}

/// (Re)registers the recording shortcuts and "paste last". Returns the ones that failed.
pub fn register_all(app: &AppHandle) -> Vec<String> {
    let gs = app.global_shortcut();
    let mut bindings = BINDINGS.lock();
    for (s, _) in bindings.drain(..) {
        let _ = gs.unregister(s);
    }
    let hk = app.state::<AppState>().settings.read().hotkeys.clone();
    let mut failed = vec![];
    let actions = [
        (hk.dictate, Action::Record(RecordMode::Dictate)),
        (hk.translate, Action::Record(RecordMode::Translate)),
        (hk.ask, Action::Record(RecordMode::Ask)),
        (hk.command, Action::Record(RecordMode::Command)),
        (hk.paste_last, Action::PasteLast),
    ];
    for (acc, action) in actions {
        if acc.trim().is_empty() {
            continue;
        }
        match parse(&acc) {
            Ok(s) if bindings.iter().any(|(b, _)| *b == s) => failed.push(format!("{acc} (repetido)")),
            Ok(s) => match gs.register(s) {
                Ok(()) => {
                    log::info!("hotkey registered: {acc}");
                    bindings.push((s, action))
                }
                Err(e) => {
                    log::warn!("hotkey {acc}: {e}");
                    failed.push(format!("{acc} (em uso por outro programa)"));
                }
            },
            Err(e) => failed.push(e.to_string()),
        }
    }
    failed
}

/// Drops every shortcut (while the user captures a new one in Settings).
pub fn unregister_all(app: &AppHandle) {
    let gs = app.global_shortcut();
    for (s, _) in BINDINGS.lock().drain(..) {
        let _ = gs.unregister(s);
    }
}

/// Esc is only grabbed while the overlay is up so it keeps working in other apps.
pub fn register_escape(app: &AppHandle, on: bool) {
    let Ok(esc) = parse("Escape") else { return };
    let gs = app.global_shortcut();
    let mut bindings = BINDINGS.lock();
    let present = bindings.iter().any(|(s, _)| *s == esc);
    if on && !present {
        if gs.register(esc).is_ok() {
            bindings.push((esc, Action::Cancel));
        }
    } else if !on && present {
        let _ = gs.unregister(esc);
        bindings.retain(|(s, _)| *s != esc);
    }
}

pub fn validate(accelerator: &str) -> Option<String> {
    match parse(accelerator) {
        Err(e) => Some(e.to_string()),
        Ok(s) if s.mods.is_empty() && !accelerator.to_uppercase().starts_with('F') => {
            Some("use pelo menos um modificador (Ctrl, Alt, Shift ou Win) ou uma tecla F".into())
        }
        Ok(_) => None,
    }
}
