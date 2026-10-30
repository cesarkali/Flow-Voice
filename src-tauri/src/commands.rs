//! Tauri commands called by the UI (see src/lib/api.ts).

use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

use crate::cloud::{self, ChatMsg};
use crate::keys::{KeyStatus, KEY_PROVIDERS};
use crate::local_stt::LocalModelInfo;
use crate::settings::{builtin_styles, Settings};
use crate::state::{AppState, ChatMessage, DictationResult, RecordMode};
use crate::{audio, hotkeys, pipeline, tray, windows};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ---------------------------------------------------------------- settings

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings().for_ui()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    window: tauri::WebviewWindow,
    state: State<AppState>,
    settings: Settings,
) -> CmdResult<Settings> {
    let before = state.settings();
    let saved = state.save_settings(settings).map_err(err)?;
    // Settings are saved even if a shortcut couldn't be registered; the UI hears about that separately.
    if let Err(e) = apply_settings(&app, &before, &saved, Some(window.label())) {
        log::warn!("{e}");
        let _ = app.emit("hotkey-error", e);
    }
    Ok(saved.for_ui())
}

#[tauri::command]
pub fn reset_styles(app: AppHandle, state: State<AppState>) -> CmdResult<Settings> {
    let mut s = state.settings();
    s.styles = builtin_styles();
    s.active_style = "natural".into();
    let saved = state.save_settings(s).map_err(err)?;
    tray::refresh(&app);
    broadcast_settings(&app, &saved, None);
    Ok(saved.for_ui())
}

/// Tells every window (except the one that just saved) about new settings.
pub fn broadcast_settings(app: &AppHandle, s: &Settings, except: Option<&str>) {
    let payload = s.for_ui();
    let except = except.map(str::to_string);
    let _ = app.emit_filter("settings-changed", payload, move |target| match (&except, target) {
        (Some(label), tauri::EventTarget::WebviewWindow { label: l }) => l != label,
        (Some(label), tauri::EventTarget::Window { label: l }) => l != label,
        (Some(label), tauri::EventTarget::Webview { label: l }) => l != label,
        _ => true,
    });
}

/// Side effects of a settings change (hotkeys, autostart, tray, model warm-up).
pub fn apply_settings(app: &AppHandle, before: &Settings, after: &Settings, from_window: Option<&str>) -> CmdResult<()> {
    if before.styles.len() != after.styles.len()
        || before.active_style != after.active_style
        || before.styles.iter().zip(&after.styles).any(|(a, b)| a.name != b.name || a.id != b.id)
    {
        tray::refresh(app);
    }
    if before.start_with_os != after.start_with_os {
        set_autostart(app, after.start_with_os);
    }
    crate::pipeline::ensure_preview_model(app);
    let local_wanted = after.stt.engine == "local" || after.stt.live_preview;
    if local_wanted
        && (before.stt.engine != after.stt.engine
            || before.stt.local_model != after.stt.local_model
            || before.stt.preview_model != after.stt.preview_model)
    {
        let st = app.state::<AppState>();
        let local = st.local.clone();
        let id = if after.stt.engine == "local" { after.stt.local_model.clone() } else { after.stt.preview_model.clone() };
        std::thread::spawn(move || local.warm_up(&id));
    }
    broadcast_settings(app, after, from_window);
    let hk_changed = before.hotkeys.dictate != after.hotkeys.dictate
        || before.hotkeys.translate != after.hotkeys.translate
        || before.hotkeys.ask != after.hotkeys.ask;
    if hk_changed {
        let failed = hotkeys::register_all(app);
        if !failed.is_empty() {
            return Err(format!("Não foi possível ativar: {}", failed.join(", ")));
        }
    }
    Ok(())
}

pub fn set_autostart(app: &AppHandle, on: bool) {
    if cfg!(debug_assertions) {
        return; // never register the dev binary
    }
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let res = if on { al.enable() } else { al.disable() };
    if let Err(e) = res {
        log::warn!("autostart: {e}");
    }
}

#[tauri::command]
pub fn validate_hotkey(accelerator: String) -> Option<String> {
    hotkeys::validate(&accelerator)
}

#[tauri::command]
pub fn suspend_hotkeys(app: AppHandle, suspend: bool) {
    if suspend {
        hotkeys::unregister_all(&app);
    } else {
        hotkeys::register_all(&app);
    }
}

#[tauri::command]
pub fn list_input_devices() -> Vec<String> {
    audio::list_input_devices()
}

// ---------------------------------------------------------------- keys & providers

#[tauri::command]
pub fn get_key_status(state: State<AppState>) -> BTreeMap<String, KeyStatus> {
    state.keys.status()
}

#[tauri::command]
pub fn set_keys(state: State<AppState>, provider: String, keys: String) -> CmdResult<BTreeMap<String, KeyStatus>> {
    if !KEY_PROVIDERS.contains(&provider.as_str()) {
        return Err("provedor desconhecido".into());
    }
    state.keys.set(&provider, &keys).map_err(err)?;
    Ok(state.keys.status())
}

#[tauri::command]
pub fn reveal_keys(state: State<AppState>, provider: String) -> String {
    state.keys.raw(&provider)
}

#[derive(Serialize)]
pub struct TestResult {
    ok: bool,
    latency_ms: u64,
    model: String,
    message: String,
}

#[tauri::command]
pub async fn test_provider(state: State<'_, AppState>, provider: String, kind: String) -> CmdResult<TestResult> {
    let s = state.settings();
    let key = state.keys.get(&provider).into_iter().next().unwrap_or_default();
    let needs_key = !matches!(provider.as_str(), "ollama" | "custom");
    if key.is_empty() && needs_key {
        return Ok(TestResult { ok: false, latency_ms: 0, model: String::new(), message: "nenhuma chave salva".into() });
    }
    let cfg = s.providers.get(&provider).cloned().unwrap_or_default();
    let t0 = Instant::now();
    let (outcome, model) = match kind.as_str() {
        "search" => (cloud::tavily(&state.http, &key, "FlowVoice ditado por voz").await.map(|_| "ok".to_string()), "tavily".to_string()),
        "stt" => {
            // 1.2 s of quiet noise: validates key + model without needing real speech.
            let samples: Vec<f32> = (0..19_200).map(|i| ((i as f32) * 0.37).sin() * 0.002).collect();
            let wav = audio::to_wav(&samples);
            (
                cloud::transcribe(&state.http, &provider, &cfg, &key, &wav, &s.speech_language, "").await,
                cfg.stt_model.clone(),
            )
        }
        _ => {
            let msgs = [
                ChatMsg::new("system", "Responda exatamente: OK"),
                ChatMsg::new("user", "teste"),
            ];
            (cloud::chat(&state.http, &provider, &cfg, &key, &msgs, 0.0, 256).await, cfg.model.clone())
        }
    };
    let latency_ms = t0.elapsed().as_millis() as u64;
    Ok(match outcome {
        Ok(_) => TestResult { ok: true, latency_ms, model, message: "funcionando".into() },
        Err(e) => TestResult { ok: false, latency_ms, model, message: format!("{} ({})", e.message, e.detail) },
    })
}

#[tauri::command]
pub async fn list_provider_models(state: State<'_, AppState>, provider: String) -> CmdResult<Vec<String>> {
    let s = state.settings();
    let cfg = s.providers.get(&provider).cloned().unwrap_or_default();
    let key = state.keys.get(&provider).into_iter().next().unwrap_or_default();
    cloud::list_models(&state.http, &provider, &cfg, &key).await.map_err(err)
}

// ---------------------------------------------------------------- local models

#[tauri::command]
pub fn list_local_models(state: State<AppState>) -> Vec<LocalModelInfo> {
    state.local.list()
}

#[tauri::command]
pub async fn download_local_model(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.local.clone().download(app, id).await.map_err(err)
}

#[tauri::command]
pub fn delete_local_model(state: State<AppState>, id: String) -> CmdResult<()> {
    state.local.delete(&id).map_err(err)
}

// ---------------------------------------------------------------- recording / overlay

#[tauri::command]
pub fn start_recording(app: AppHandle, mode: RecordMode) {
    pipeline::start(&app, mode);
}

#[tauri::command]
pub fn stop_recording(app: AppHandle) {
    pipeline::stop(&app);
}

#[tauri::command]
pub fn cancel_recording(app: AppHandle) {
    pipeline::cancel(&app);
}

#[tauri::command]
pub async fn reprocess(app: AppHandle, style_id: String, target: Option<String>) -> CmdResult<DictationResult> {
    pipeline::reprocess(&app, style_id, target).await.map_err(err)
}

#[tauri::command]
pub async fn paste_text(app: AppHandle, text: String, replace: bool) -> CmdResult<()> {
    let target = pipeline::last_target_window(&app);
    windows::hide_overlay(&app);
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        crate::platform::paste(&text, target, replace)
    })
    .await
    .map_err(err)?
    .map(|_| ())
    .map_err(err)
}

#[tauri::command]
pub fn copy_text(text: String) -> CmdResult<()> {
    crate::platform::set_clipboard(&text).map_err(err)
}

#[tauri::command]
pub fn overlay_hold(state: State<AppState>, hold: bool) {
    state.overlay_hold.store(hold, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub fn overlay_resize(width: f64, height: f64) {
    // The overlay window has a fixed size now (see windows::OVERLAY_W); kept for older UIs.
    let _ = (width, height);
}

#[derive(serde::Deserialize)]
pub struct HitRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[tauri::command]
pub fn overlay_hitbox(rect: Option<HitRect>) {
    windows::set_overlay_hitbox(rect.map(|r| (r.x, r.y, r.width, r.height)));
}

#[tauri::command]
pub fn dismiss_overlay(app: AppHandle) {
    if pipeline::is_recording(&app) {
        pipeline::cancel(&app);
    } else {
        windows::hide_overlay(&app);
    }
}

// ---------------------------------------------------------------- history

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Vec<DictationResult> {
    state.history.lock().clone()
}

#[tauri::command]
pub fn delete_history_item(app: AppHandle, state: State<AppState>, id: String) {
    let mut h = state.history.lock();
    h.retain(|x| x.id != id);
    state.persist_history(&h);
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
pub fn clear_history(app: AppHandle, state: State<AppState>) {
    let mut h = state.history.lock();
    h.clear();
    state.persist_history(&h);
    let _ = app.emit("history-changed", ());
}

// ---------------------------------------------------------------- chat

#[tauri::command]
pub fn get_chat(state: State<AppState>) -> Vec<ChatMessage> {
    state.chat.lock().clone()
}

#[tauri::command]
pub async fn chat_send(app: AppHandle, text: String) -> Vec<ChatMessage> {
    pipeline::chat_send(&app, text).await
}

#[tauri::command]
pub async fn chat_toggle_recording(app: AppHandle) -> CmdResult<bool> {
    pipeline::chat_toggle_recording(&app).await.map_err(err)
}

#[tauri::command]
pub fn chat_reset(state: State<AppState>) {
    state.chat.lock().clear();
}

// ---------------------------------------------------------------- app

#[tauri::command]
pub fn open_settings(app: AppHandle, page: Option<String>) {
    windows::open_settings(&app, page);
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    author: String,
    github: String,
    website: String,
    instagram: String,
    data_dir: String,
    os: String,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        author: "Júlio Caliberda".into(),
        github: "https://github.com/cesarkali/Flow-Voice".into(),
        website: "https://caliberda.com.br".into(),
        instagram: "https://www.instagram.com/cesar.kali/".into(),
        data_dir: state.config_dir.display().to_string(),
        os: std::env::consts::OS.into(),
    }
}

#[derive(Serialize)]
pub struct UpdateInfo {
    available: bool,
    version: Option<String>,
    notes: Option<String>,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> CmdResult<UpdateInfo> {
    let update = app.updater().map_err(err)?.check().await.map_err(|e| format!("não foi possível verificar: {e}"))?;
    Ok(match update {
        Some(u) => UpdateInfo { available: true, version: Some(u.version.clone()), notes: u.body.clone() },
        None => UpdateInfo { available: false, version: None, notes: None },
    })
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> CmdResult<()> {
    let update = app.updater().map_err(err)?.check().await.map_err(err)?.ok_or("já está na versão mais recente")?;
    let mut downloaded: u64 = 0;
    let app2 = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = app2.emit("update-progress", json!({ "downloaded": downloaded, "total": total }));
            },
            || {},
        )
        .await
        .map_err(|e| format!("falha ao instalar: {e}"))?;
    app.restart();
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("endereço inválido".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(err)
}

/// Author photo (GitHub avatar) as a data URL. Refreshed at most every 12 h; offline it falls
/// back to the last downloaded copy, and to None (initials) if there never was one.
#[tauri::command]
pub async fn author_avatar(state: State<'_, AppState>) -> CmdResult<Option<String>> {
    use base64::Engine;
    let path = state.config_dir.join("author-avatar.img");
    let fresh = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|e| e.as_secs() < 12 * 3600).unwrap_or(false))
        .unwrap_or(false);
    if !fresh {
        let fetched = async {
            let resp = state
                .http
                .get("https://github.com/cesarkali.png?size=160")
                .timeout(std::time::Duration::from_secs(8))
                .send()
                .await
                .ok()?
                .error_for_status()
                .ok()?;
            let bytes = resp.bytes().await.ok()?;
            (bytes.len() > 200).then_some(bytes)
        }
        .await;
        if let Some(bytes) = fetched {
            let _ = std::fs::write(&path, &bytes);
        }
    }
    let Ok(bytes) = std::fs::read(&path) else { return Ok(None) };
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
    Ok(Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))))
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
