mod apps;
mod audio;
mod cloud;
mod commands;
mod feedback;
mod glass;
mod heal;
mod hotkeys;
mod i18n;
mod keys;
#[cfg(test)]
mod live_tests;
mod local_stt;
mod overlay_drag;
mod pipeline;
mod platform;
mod preview;
mod settings;
mod startup;
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
    // Started by the logon task while startup is turned off in Task Manager: respect that.
    if std::env::args().any(|a| a == startup::TASK_ARG) && startup::disabled_in_task_manager("FlowVoice") {
        return;
    }
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
            } else if has("--proofread") {
                pipeline::control(app, pipeline::Ctl::Proofread);
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
            // The "Revisar" shortcut is new in 2.2: its AltGr check runs once, on the first start
            // that doesn't have it in the file yet (new installs and updates alike).
            let new_proofread = std::fs::read_to_string(settings::settings_path(&config_dir))
                .map_or(true, |t| !t.contains("\"proofread\""));
            pipeline::restore_mute_after_crash(&config_dir);
            let state = AppState::new(config_dir.clone(), models_dir);
            if first_run {
                let n = state.keys.import_legacy();
                if n > 0 {
                    log::info!("imported {n} key(s) from FlowVoice 1.x");
                }
                let mut s = state.settings();
                // A new install starts on this version: no "what's new" (onboarding shows the app).
                s.seen_version = app.package_info().version.to_string();
                // Ctrl+Alt is AltGr: where AltGr+K types a character, the shortcut would stop it.
                if platform::altgr_types('K' as u32) {
                    log::info!("AltGr+K types on this keyboard: Command starts without a shortcut");
                    s.hotkeys.command.clear();
                }
                let _ = state.save_settings(s);
            }
            if new_proofread {
                let mut s = state.settings();
                let h = &s.hotkeys;
                let taken = [&h.dictate, &h.translate, &h.ask, &h.command, &h.paste_last].contains(&&h.proofread);
                if taken || platform::altgr_types('J' as u32) {
                    log::info!("Ctrl+Alt+J is another shortcut or AltGr+J types here: Revisar starts without a shortcut");
                    s.hotkeys.proofread.clear();
                }
                let _ = state.save_settings(s);
            }
            let s = state.settings();
            i18n::set_language(&s.ui_language);
            app.manage(state);
            // The windows are created only now ("create": false in tauri.conf.json): made by Tauri
            // before setup, their pages could call a command before the state above existed (a
            // busy PC at login showed "state not managed for field `state`" in the settings).
            for cfg in app.config().app.windows.iter().filter(|w| !w.create) {
                tauri::WebviewWindowBuilder::from_config(app.handle(), cfg)?.build()?;
            }

            let handle = app.handle().clone();
            windows::apply_window_glass(&handle, &s);
            windows::setup_overlay(&handle);
            windows::start_pointer_watcher(&handle);
            pipeline::start_control(&handle);
            platform::start_focus_tracker();
            tray::setup(&handle)?;
            let failed = hotkeys::register_all(&handle);
            if !failed.is_empty() {
                log::warn!("hotkeys failed: {failed:?}");
            }
            commands::heal_autostart(&handle, s.start_with_os);

            pipeline::ensure_preview_model(&handle);
            heal::start_background(&handle);
            // Local models load on demand (when a dictation starts) and unload when idle.
            handle.state::<AppState>().local.start_idle_unloader();

            // After an update the installer relaunches the app with the arguments it had: started
            // with Windows (`--hidden`) it would come back only in the tray, as if it hadn't
            // reopened. The first launch of a new version always shows the window. (No file yet
            // with settings already there: updated from 2.0 or older.)
            let version = app.package_info().version.to_string();
            let version_file = config_dir.join("last-version");
            let updated = test_dir.is_none()
                && match std::fs::read_to_string(&version_file) {
                    Ok(prev) => prev.trim() != version,
                    Err(_) => !first_run,
                };
            if updated || first_run {
                let _ = std::fs::write(&version_file, &version);
            }
            if updated {
                log::info!("updated to {version}: showing the window");
            }
            let hidden = std::env::args().any(|a| a == "--hidden");
            if !s.onboarded || !hidden || updated {
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
            commands::altgr_blocks_command,
            commands::altgr_blocks_proofread,
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
            commands::command_style,
            commands::paste_text,
            commands::copy_text,
            commands::overlay_hold,
            commands::overlay_resize,
            commands::overlay_hitbox,
            overlay_drag::overlay_drag_arm,
            overlay_drag::get_last_overlay_drag,
            overlay_drag::test_drag,
            windows::get_overlay_anchor,
            windows::test_overlay_position,
            commands::overlay_glass,
            commands::glass_available,
            commands::dismiss_overlay,
            commands::test_probe,
            commands::get_history,
            commands::delete_history_item,
            commands::clear_history,
            commands::retry_failed,
            commands::get_chat,
            commands::chat_send,
            commands::chat_toggle_recording,
            commands::chat_reset,
            commands::open_settings,
            commands::app_info,
            commands::whats_new_seen,
            commands::open_feedback,
            commands::feedback_log,
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
