//! Tauri commands called by the UI (see src/lib/api.ts).

use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
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
    // Only `whats_new_seen` moves it: a window holding older settings must not undo that.
    let settings = Settings { seen_version: before.seen_version.clone(), ..settings };
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
    if before.ui_language != after.ui_language {
        crate::i18n::set_language(&after.ui_language);
    }
    if before.ui_language != after.ui_language
        || before.styles.len() != after.styles.len()
        || before.active_style != after.active_style
        || before.styles.iter().zip(&after.styles).any(|(a, b)| a.name != b.name || a.id != b.id)
    {
        tray::refresh(app);
    }
    if before.start_with_os != after.start_with_os {
        set_autostart(app, after.start_with_os);
    }
    if before.window_glass != after.window_glass || before.overlay_glass != after.overlay_glass || before.theme != after.theme {
        windows::apply_window_glass(app, after);
    }
    crate::pipeline::ensure_preview_model(app);
    broadcast_settings(app, after, from_window);
    let hk_changed = before.hotkeys != after.hotkeys;
    if hk_changed {
        let failed = hotkeys::register_all(app);
        if !failed.is_empty() {
            return Err(crate::i18n::trf("Não foi possível ativar: {}", &[&failed.join(", ")]));
        }
    }
    Ok(())
}

/// Only the installed app may touch the OS startup entry: dev builds, binaries run from a cargo
/// `target` folder and isolated test instances (FLOWVOICE_DATA_DIR) must never add, repoint or
/// delete it (a test instance with autostart off once wiped the user's real entry).
fn owns_autostart() -> bool {
    if cfg!(debug_assertions) || std::env::var_os("FLOWVOICE_DATA_DIR").is_some() {
        return false;
    }
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
    !exe.is_empty() && !exe.contains("\\target\\") && !exe.contains("/target/")
}

/// The setting changed: the Run entry (also marked approved again in Task Manager) and the logon
/// task follow it (see startup.rs).
pub fn set_autostart(app: &AppHandle, on: bool) {
    if !owns_autostart() {
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let res = if on { al.enable() } else { al.disable() };
    match res {
        Ok(()) => log::info!("autostart {}", if on { "on" } else { "off" }),
        Err(e) => log::warn!("autostart: {e}"),
    }
    set_logon_task(on);
}

/// On every launch, so both heal themselves (re-created and pointed at the current exe) if
/// something removed them or the app moved. Startup turned off in Task Manager stays off: enable()
/// would mark it approved again, and the logon task would start the app anyway.
pub fn heal_autostart(app: &AppHandle, on: bool) {
    if on && owns_autostart() && crate::startup::disabled_in_task_manager(&app.package_info().name) {
        log::info!("autostart off in Task Manager: left off");
        set_logon_task(false);
        return;
    }
    set_autostart(app, on);
}

fn set_logon_task(on: bool) {
    let Ok(exe) = std::env::current_exe() else { return };
    std::thread::spawn(move || match crate::startup::set_logon_task(&exe, on) {
        Ok(()) => log::info!("logon task {}", if on { "on" } else { "off" }),
        Err(e) => log::warn!("logon task: {e}"),
    });
}

/// Ctrl+Alt+K would stop AltGr+K from typing on this keyboard (the Shortcuts page explains why
/// Command has no shortcut).
#[tauri::command]
pub fn altgr_blocks_command() -> bool {
    crate::platform::altgr_types('K' as u32)
}

/// Same for "Revisar" (Ctrl+Alt+J).
#[tauri::command]
pub fn altgr_blocks_proofread() -> bool {
    crate::platform::altgr_types('J' as u32)
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

/// `key`: the one just pasted ("Salvar e testar", a key added in Providers); without it, the first
/// saved key.
#[tauri::command]
pub async fn test_provider(state: State<'_, AppState>, provider: String, kind: String, key: Option<String>) -> CmdResult<TestResult> {
    let s = state.settings();
    let key = key
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| state.keys.get(&provider).into_iter().next().unwrap_or_default());
    let needs_key = !matches!(provider.as_str(), "ollama" | "custom");
    if key.is_empty() && needs_key {
        return Ok(TestResult { ok: false, latency_ms: 0, model: String::new(), message: crate::i18n::tr("nenhuma chave salva") });
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

/// Stops a running model download (no-op if none); emits the final `model-progress` with `cancelled: true`.
#[tauri::command]
pub fn cancel_local_model_download(app: AppHandle, state: State<AppState>, id: String) {
    state.local.cancel_download(&app, &id);
}

#[tauri::command]
pub fn delete_local_model(state: State<AppState>, id: String) -> CmdResult<()> {
    state.local.delete(&id).map_err(err)
}

// ---------------------------------------------------------------- recording / overlay

#[tauri::command]
pub fn start_recording(app: AppHandle, mode: RecordMode) {
    pipeline::control(&app, pipeline::Ctl::Start(mode));
}

#[tauri::command]
pub fn stop_recording(app: AppHandle) {
    pipeline::control(&app, pipeline::Ctl::Stop);
}

#[tauri::command]
pub fn cancel_recording(app: AppHandle) {
    pipeline::control(&app, pipeline::Ctl::Cancel);
}

/// Command card: rewrite the selection in a style instead of speaking an instruction.
#[tauri::command]
pub fn command_style(app: AppHandle, style_id: String, target: Option<String>) {
    pipeline::control(&app, pipeline::Ctl::CommandStyle(style_id, target));
}

#[tauri::command]
pub async fn reprocess_history(
    app: AppHandle,
    id: String,
    style_id: String,
    target: Option<String>,
) -> CmdResult<crate::state::DictationResult> {
    pipeline::reprocess_history(&app, id, style_id, target).await.map_err(err)
}

#[tauri::command]
pub async fn reprocess(app: AppHandle, style_id: String, target: Option<String>) -> CmdResult<DictationResult> {
    pipeline::reprocess(&app, style_id, target).await.map_err(err)
}

/// The card's "Colar" / "Substituir". Returns whether it was pasted; if not, the text is only
/// copied and a card says so (the result card is already gone).
#[tauri::command]
pub async fn paste_text(app: AppHandle, text: String, replace: bool) -> CmdResult<bool> {
    let target = pipeline::last_target_window(&app);
    windows::hide_overlay(&app);
    let pasted = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        crate::platform::paste_replaceable(&text, target, replace)
    })
    .await
    .map_err(err)?
    .map_err(err)?;
    if !pasted {
        pipeline::overlay_hint(
            &app,
            &if replace {
                crate::i18n::tr("Não deu para substituir: o texto mudou depois da colagem ou a janela não aceita. A nova versão está copiada, cole com Ctrl+V.")
            } else {
                crate::i18n::tr("Não deu para colar nessa janela — o texto está copiado, cole com Ctrl+V.")
            },
        );
    }
    Ok(pasted)
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

/// "Vidro": Windows' blur under the card's box (None: fades out). See glass.rs.
#[tauri::command]
pub fn overlay_glass(app: AppHandle, rect: Option<crate::glass::GlassRect>) {
    crate::glass::update(&app, rect);
}

#[tauri::command]
pub fn glass_available() -> bool {
    crate::glass::available()
}

#[tauri::command]
/// The card's close button / auto-close. It acts on what the card shows NOW: a recording is
/// never cancelled from here (a result card's countdown can end just as a new recording starts;
/// the listening card's own X calls cancel_recording), and processing is only cancelled when
/// the processing card is the one being closed.
pub fn dismiss_overlay(app: AppHandle) {
    if pipeline::is_recording(&app) {
        log::info!("dismiss ignored: recording");
        return;
    }
    match pipeline::overlay_phase().as_str() {
        "transcribing" | "refining" if pipeline::is_busy(&app) => pipeline::cancel_processing(&app),
        _ => {
            pipeline::release_overlay();
            windows::hide_overlay(&app);
        }
    }
}

// ---------------------------------------------------------------- history

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Vec<DictationResult> {
    state.history_for_ui()
}

#[tauri::command]
pub fn delete_history_item(app: AppHandle, state: State<AppState>, id: String) {
    state.delete_history(&id);
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
pub fn clear_history(app: AppHandle, state: State<AppState>) {
    // The kept recordings of failed transcriptions go too.
    state.failed.lock().clear();
    let mut h = state.history.lock();
    h.clear();
    state.persist_history(&h);
    let _ = app.emit("history-changed", ());
}

/// "Tentar de novo" on a failed transcription (error card or history). `only_copy`: from the
/// history, where the user is in the settings: the result is only copied, not pasted.
#[tauri::command]
pub fn retry_failed(app: AppHandle, id: String, only_copy: bool) {
    pipeline::retry_failed(&app, id, only_copy);
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

/// The "what's new" of this version was shown and closed: not again until the next update.
#[tauri::command]
pub fn whats_new_seen(app: AppHandle, state: State<AppState>) -> CmdResult<()> {
    let mut s = state.settings();
    s.seen_version = app.package_info().version.to_string();
    let saved = state.save_settings(s).map_err(err)?;
    broadcast_settings(&app, &saved, None);
    Ok(())
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
        os: crate::platform::os_version(),
    }
}

/// What a bug report would carry from flowvoice.log (shown before sending; its oldest lines may
/// be left out so the form's URL isn't cut short, see feedback::issue_url).
#[tauri::command]
pub fn feedback_log(state: State<AppState>) -> String {
    crate::feedback::log_excerpt(&state.config_dir)
}

/// Opens a GitHub issue form ("bug" or "idea") prefilled with the app and Windows versions —
/// and, for an error card, the error message. Never the dictated text or any key.
#[tauri::command]
pub fn open_feedback(
    app: AppHandle,
    state: State<AppState>,
    kind: String,
    context: Option<String>,
    include_log: Option<bool>,
) -> CmdResult<()> {
    let version = app.package_info().version.to_string();
    let mut params: Vec<(&str, String)> = vec![("version", version)];
    let mut what = None;
    let mut log = String::new();
    let template = if kind == "idea" {
        "feature_request.yml"
    } else {
        params.push(("os", crate::platform::os_version()));
        what = context.map(|c| c.trim().chars().take(600).collect::<String>()).filter(|c| !c.is_empty());
        if include_log.unwrap_or(true) {
            log = crate::feedback::log_excerpt(&state.config_dir);
        }
        "bug_report.yml"
    };
    params.insert(0, ("template", template.to_string()));
    let url = crate::feedback::issue_url("https://github.com/cesarkali/Flow-Voice/issues/new", &params, what.as_deref(), &log)
        .map_err(err)?;
    app.opener().open_url(url.as_str(), None::<&str>).map_err(err)
}

#[derive(Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> CmdResult<UpdateInfo> {
    let update = match app.updater().map_err(err)?.check().await {
        Ok(u) => u,
        // No manifest published for the latest release (e.g. it is still a draft): nothing new.
        Err(e) if ["successful status code", "valid release JSON"].iter().any(|m| e.to_string().contains(m)) => None,
        Err(e) => return Err(crate::i18n::trf("não foi possível verificar: {}", &[&e])),
    };
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
        return Err(crate::i18n::tr("endereço inválido"));
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

/// Automatic model replacements not yet shown in the settings window (see heal.rs).
#[tauri::command]
pub fn take_model_notices() -> Vec<crate::heal::Notice> {
    crate::heal::take_notices()
}

/// Test probe, only in isolated test instances (FLOWVOICE_DATA_DIR): reads the real overlay
/// state and simulates the pointer, so stress tests never touch the user's mouse or keyboard.
#[tauri::command]
pub fn test_probe(app: AppHandle, action: String, inside: Option<bool>) -> CmdResult<serde_json::Value> {
    if std::env::var_os("FLOWVOICE_DATA_DIR").is_none() {
        return Err("indisponível".into());
    }
    if action == "pointer" {
        windows::test_set_pointer(inside);
    }
    let mut v = windows::test_snapshot(&app);
    v["phase"] = json!(pipeline::overlay_phase());
    v["recording"] = json!(pipeline::is_recording(&app));
    v["busy"] = json!(pipeline::is_busy(&app));
    v["history"] = json!(tauri::Manager::state::<AppState>(&app).history.lock().len());
    Ok(v)
}
