mod apps;
mod audio;
mod cloud;
mod commands;
mod heal;
mod hotkeys;
mod keys;
#[cfg(test)]
mod live_tests;
mod local_stt;
mod pipeline;
mod platform;
mod preview;
mod settings;
mod state;
mod text;
mod tray;
mod windows;

use std::io::Write;
use std::path::PathBuf;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use state::AppState;

/// Minimal file logger (flowvoice.log next to settings.json, capped at ~1 MB).
struct FileLogger {
    file: parking_lot::Mutex<std::fs::File>,
}

impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info && (m.target().starts_with("flowvoice") || m.level() <= log::Level::Warn)
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let line = format!("{} {:<5} {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), r.level(), r.args());
            let _ = self.file.lock().write_all(line.as_bytes());
            #[cfg(debug_assertions)]
            eprint!("{line}");
        }
    }
    fn flush(&self) {}
}

fn init_logger(dir: &PathBuf) {
    let path = dir.join("flowvoice.log");
    if std::fs::metadata(&path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = log::set_boxed_logger(Box::new(FileLogger { file: parking_lot::Mutex::new(file) }));
        log::set_max_level(log::LevelFilter::Info);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Test hook: FLOWVOICE_DATA_DIR runs an isolated instance (own settings/history, no
    // single-instance lock) next to the user's real one.
    let test_dir = std::env::var_os("FLOWVOICE_DATA_DIR").map(std::path::PathBuf::from);
    let mut builder = tauri::Builder::default();
    if test_dir.is_none() {
        // A second launch talks to the running app: `flowvoice --toggle` (also --translate,
        // --ask, --cancel) lets desktops without global hotkeys (Wayland) or tools like Stream
        // Deck/AutoHotkey drive dictation; `--hidden` (autostart racing a running app) is a no-op.
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let has = |flag: &str| args.iter().any(|a| a == flag);
            if has("--toggle") || has("--dictate") {
                pipeline::control(app, pipeline::Ctl::Toggle(state::RecordMode::Dictate));
            } else if has("--translate") {
                pipeline::control(app, pipeline::Ctl::Toggle(state::RecordMode::Translate));
            } else if has("--ask") {
                pipeline::control(app, pipeline::Ctl::Toggle(state::RecordMode::Ask));
            } else if has("--command") {
                pipeline::control(app, pipeline::Ctl::Toggle(state::RecordMode::Command));
            } else if has("--paste-last") {
                pipeline::paste_last(app, platform::last_app_window());
            } else if has("--cancel") {
                pipeline::control(app, pipeline::Ctl::Cancel);
            } else if !has("--hidden") {
                windows::open_settings(app, None);
            }
        }));
    }
    let app = builder
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| hotkeys::handler(app, shortcut, event))
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let config_dir = match &test_dir {
                Some(d) => d.clone(),
                None => app.path().app_config_dir()?,
            };
            let models_dir = app.path().app_local_data_dir()?.join("models");
            std::fs::create_dir_all(&config_dir)?;
            init_logger(&config_dir);
            log::info!("FlowVoice {} starting", app.package_info().version);

            let first_run = !settings::settings_path(&config_dir).exists();
            let state = AppState::new(config_dir.clone(), models_dir);
            if first_run {
                let n = state.keys.import_legacy();
                if n > 0 {
                    log::info!("imported {n} key(s) from FlowVoice 1.x");
                }
                let s = state.settings();
                let _ = settings::save(&config_dir, &s);
            }
            let s = state.settings();
            app.manage(state);

            let handle = app.handle().clone();
            windows::setup_overlay(&handle);
            windows::start_pointer_watcher(&handle);
            pipeline::start_control(&handle);
            platform::start_focus_tracker();
            tray::setup(&handle)?;
            let failed = hotkeys::register_all(&handle);
            if !failed.is_empty() {
                log::warn!("hotkeys failed: {failed:?}");
            }
            commands::set_autostart(&handle, s.start_with_os);

            pipeline::ensure_preview_model(&handle);
            heal::start_background(&handle);
            // Local models load on demand (when a dictation starts) and unload when idle.
            handle.state::<AppState>().local.start_idle_unloader();

            let hidden = std::env::args().any(|a| a == "--hidden");
            if !s.onboarded || !hidden {
                windows::open_settings(&handle, None);
            }
            if !failed.is_empty() {
                let _ = handle.emit("hotkey-error", failed);
            }

            // Quiet background update check, shortly after launch and then every 6 h (the app
            // lives in the tray for weeks).
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                loop {
                    match commands::check_update(h.clone()).await {
                        Ok(info) if info.available => {
                            let _ = h.emit("update-available", &info);
                        }
                        Ok(_) => {}
                        Err(e) => log::warn!("update check: {e}"),
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings or chat window only hides it; the app lives in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "overlay" {
                    api.prevent_close();
                    windows::hide_overlay(window.app_handle());
                } else if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::reset_styles,
            commands::validate_hotkey,
            commands::list_input_devices,
            commands::suspend_hotkeys,
            commands::get_key_status,
            commands::set_keys,
            commands::reveal_keys,
            commands::test_provider,
            commands::list_provider_models,
            commands::list_local_models,
            commands::download_local_model,
            commands::cancel_local_model_download,
            commands::delete_local_model,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::reprocess,
            commands::reprocess_history,
            commands::paste_text,
            commands::copy_text,
            commands::overlay_hold,
            commands::overlay_resize,
            commands::overlay_hitbox,
            commands::dismiss_overlay,
            commands::test_probe,
            commands::get_history,
            commands::delete_history_item,
            commands::clear_history,
            commands::get_chat,
            commands::chat_send,
            commands::chat_toggle_recording,
            commands::chat_reset,
            commands::open_settings,
            commands::app_info,
            commands::check_update,
            commands::install_update,
            commands::open_url,
            commands::quit_app,
            commands::author_avatar,
            commands::take_model_notices,
        ])
        .build(tauri::generate_context!())
        .expect("error while building FlowVoice");

    app.run(|_app, event| {
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            // Hiding the last window must not quit the tray app.
            api.prevent_exit();
        }
    });
}
