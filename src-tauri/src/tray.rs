//! System tray icon and menu.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::pipeline;
use crate::state::{AppState, RecordMode};
use crate::windows;

pub const TRAY_ID: &str = "flowvoice";

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let s = app.state::<AppState>().settings();
    let dictate = MenuItem::with_id(app, "dictate", "Ditar agora", true, None::<&str>)?;
    let paste_last = MenuItem::with_id(app, "paste_last", "Colar último ditado", true, None::<&str>)?;
    let styles: Vec<CheckMenuItem<Wry>> = s
        .styles
        .iter()
        .map(|st| CheckMenuItem::with_id(app, format!("style:{}", st.id), &st.name, true, st.id == s.active_style, None::<&str>))
        .collect::<tauri::Result<_>>()?;
    let style_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = styles.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<Wry>).collect();
    let style_menu = Submenu::with_items(app, "Estilo", true, &style_refs)?;
    let settings = MenuItem::with_id(app, "settings", "Configurações", true, None::<&str>)?;
    let history = MenuItem::with_id(app, "history", "Histórico", true, None::<&str>)?;
    let wizard = MenuItem::with_id(app, "wizard", "Refazer configuração inicial", true, None::<&str>)?;
    let feedback = MenuItem::with_id(app, "feedback", "Reportar problema ou sugerir melhoria", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair do FlowVoice", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[
            &dictate,
            &paste_last,
            &style_menu,
            &PredefinedMenuItem::separator(app)?,
            &history,
            &settings,
            &wizard,
            &feedback,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("FlowVoice — ditado por voz")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "dictate" => pipeline::control(app, pipeline::Ctl::Toggle(RecordMode::Dictate)),
                // The menu took the focus: paste into the app the user was in before.
                "paste_last" => pipeline::paste_last(app, crate::platform::last_app_window()),
                "settings" => windows::open_settings(app, Some("general".into())),
                "feedback" => windows::open_settings(app, Some("about".into())),
                "history" => windows::open_settings(app, Some("home".into())),
                "quit" => app.exit(0),
                "wizard" => restart_onboarding(app),
                _ if id.starts_with("style:") => {
                    let st = app.state::<AppState>();
                    let mut s = st.settings();
                    s.active_style = id.trim_start_matches("style:").to_string();
                    if let Ok(saved) = st.save_settings(s) {
                        crate::commands::broadcast_settings(app, &saved, None);
                    }
                    refresh(app);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                windows::open_settings(tray.app_handle(), None);
            }
        })
        .build(app)?;
    Ok(())
}

/// Rebuilds the menu so the style checkmarks follow the settings.
pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// Opens the first-run wizard again (settings and keys are kept).
pub fn restart_onboarding(app: &AppHandle) {
    let st = app.state::<AppState>();
    let mut s = st.settings();
    s.onboarded = false;
    if let Ok(saved) = st.save_settings(s) {
        crate::commands::broadcast_settings(app, &saved, None);
    }
    // The settings window ignores late "onboarded: false" echoes, so say it explicitly.
    let _ = app.emit_to("main", "restart-onboarding", ());
    windows::open_settings(app, None);
}
