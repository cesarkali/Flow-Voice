//! Global shortcuts: toggle or push-to-talk, plus Esc to cancel while recording.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState};

use crate::pipeline;
use crate::state::{AppState, RecordMode};

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Record(RecordMode),
    Proofread,
    PasteLast,
    Cancel,
}

static BINDINGS: Lazy<Mutex<Vec<(Shortcut, Action)>>> = Lazy::new(|| Mutex::new(vec![]));

pub fn parse(accelerator: &str) -> Result<Shortcut> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| anyhow!(crate::i18n::trf("atalho inválido \"{}\": {}", &[&accelerator, &e])))
}

pub fn handler(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    let action = BINDINGS.lock().iter().find(|(s, _)| s == shortcut).map(|(_, a)| *a);
    let Some(action) = action else { return };
    log::info!("hotkey {} {:?}", shortcut.into_string(), event.state());
    let hold = app.state::<AppState>().settings.read().hotkey_mode == "hold";
    match (action, event.state()) {
        (Action::Cancel, ShortcutState::Pressed) => pipeline::control(app, pipeline::Ctl::Cancel),
        (Action::PasteLast, ShortcutState::Pressed) => {
            // Into whatever has focus now (not where the dictation originally went); off the
            // main thread (paste waits for modifier keys to be released).
            let target = crate::platform::foreground_window();
            let app = app.clone();
            std::thread::spawn(move || pipeline::paste_last(&app, target));
        }
        (Action::Proofread, ShortcutState::Pressed) => pipeline::control(app, pipeline::Ctl::Proofread),
        (Action::Record(mode), ShortcutState::Pressed) => {
            pipeline::control(app, if hold { pipeline::Ctl::Hold(mode) } else { pipeline::Ctl::Toggle(mode) });
        }
        (Action::Record(_), ShortcutState::Released) if hold => pipeline::control(app, pipeline::Ctl::Stop),
        _ => {}
    }
}

/// (Re)registers the recording shortcuts, "Revisar" and "paste last". Returns the ones that failed.
/// The plugin's register/unregister block until the main thread runs them, so BINDINGS is never
/// held across those calls (a thread waiting on the main thread while holding it, with the main
/// thread waiting for BINDINGS in the hotkey handler, froze the whole app). Esc (Action::Cancel)
/// is managed only by register_escape and survives these rebuilds.
fn take_actions() -> Vec<Shortcut> {
    let mut bindings = BINDINGS.lock();
    let (keep, gone): (Vec<_>, Vec<_>) = bindings.drain(..).partition(|(_, a)| *a == Action::Cancel);
    *bindings = keep;
    gone.into_iter().map(|(s, _)| s).collect()
}

pub fn register_all(app: &AppHandle) -> Vec<String> {
    let gs = app.global_shortcut();
    for s in take_actions() {
        let _ = gs.unregister(s);
    }
    let mut bindings: Vec<(Shortcut, Action)> = vec![];
    let hk = app.state::<AppState>().settings.read().hotkeys.clone();
    let mut failed = vec![];
    let actions = [
        (hk.dictate, Action::Record(RecordMode::Dictate)),
        (hk.translate, Action::Record(RecordMode::Translate)),
        (hk.ask, Action::Record(RecordMode::Ask)),
        (hk.command, Action::Record(RecordMode::Command)),
        (hk.proofread, Action::Proofread),
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
    BINDINGS.lock().extend(bindings);
    failed
}

/// Drops the action shortcuts (while the user captures a new one in Settings).
pub fn unregister_all(app: &AppHandle) {
    let gs = app.global_shortcut();
    for s in take_actions() {
        let _ = gs.unregister(s);
    }
}

/// Esc should be grabbed: only while recording, so it keeps working in other apps the rest of
/// the time (the processing bar and the cards have their own close button).
static ESC_WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Callable from any thread without blocking: records the wish and lets the main thread
/// reconcile it (the last wish wins, whatever order the calls arrive in).
pub fn register_escape(app: &AppHandle, on: bool) {
    ESC_WANTED.store(on, std::sync::atomic::Ordering::SeqCst);
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || sync_escape(&app2));
}

fn sync_escape(app: &AppHandle) {
    let Ok(esc) = parse("Escape") else { return };
    let want = ESC_WANTED.load(std::sync::atomic::Ordering::SeqCst);
    let present = BINDINGS.lock().iter().any(|(_, a)| *a == Action::Cancel);
    if want == present {
        return;
    }
    let gs = app.global_shortcut();
    if want {
        if gs.register(esc).is_ok() {
            BINDINGS.lock().push((esc, Action::Cancel));
        }
    } else {
        let _ = gs.unregister(esc);
        BINDINGS.lock().retain(|(_, a)| *a != Action::Cancel);
    }
}

pub fn validate(accelerator: &str) -> Option<String> {
    match parse(accelerator) {
        Err(e) => Some(e.to_string()),
        // Ctrl, Alt or Win: with Shift alone (only F keys may) that character could no longer be
        // typed anywhere (Shift+A, Shift+Space).
        Ok(s) if !s.mods.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER | Modifiers::META) && !is_f_key(accelerator) => {
            Some(crate::i18n::tr("use Ctrl, Alt ou Win junto (Shift sozinho só com tecla F) ou uma tecla F"))
        }
        Ok(_) => None,
    }
}

/// The key of `accelerator` is F1…F24.
fn is_f_key(accelerator: &str) -> bool {
    let key = accelerator.rsplit('+').next().unwrap_or("").trim();
    key.len() > 1 && key.starts_with(['F', 'f']) && key[1..].chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn shortcuts_need_ctrl_alt_or_win() {
        for ok in ["CommandOrControl+Shift+Space", "Alt+F9", "Super+H", "F9", "Shift+F9"] {
            assert!(validate(ok).is_none(), "{ok}");
        }
        // Shift alone would take that character from typing everywhere.
        for bad in ["Shift+A", "Shift+Space", "A", "F", "Space"] {
            assert!(validate(bad).is_some(), "{bad}");
        }
    }
}
