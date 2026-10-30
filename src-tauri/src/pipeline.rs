//! Record → transcribe → refine → paste. Also the voice-search assistant.

use anyhow::{anyhow, Result};
use serde_json::json;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::{self, Recording};
use crate::cloud::{self, display_name, ChatMsg, ProviderError};
use crate::platform;
use crate::settings::{self, Settings, StyleKind};
use crate::state::{new_id, ActiveSession, AppState, ChatMessage, ChatSource, DictationResult, RecordMode};
use crate::text;
use crate::windows;

fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

pub fn emit_overlay(app: &AppHandle, payload: serde_json::Value) {
    state(app).overlay_gen.fetch_add(1, Ordering::SeqCst);
    let _ = app.emit_to("overlay", "overlay-state", payload);
}

pub fn overlay_error(app: &AppHandle, message: &str, detail: Option<String>, raw: Option<String>) {
    windows::show_overlay(app);
    crate::hotkeys::register_escape(app, true);
    emit_overlay(app, json!({ "phase": "error", "message": message, "detail": detail, "raw": raw }));
}

// ---------------------------------------------------------------- recording

pub fn is_recording(app: &AppHandle) -> bool {
    state(app).session.lock().is_some()
}

/// Hotkey/tray entry point: starts, or stops when already recording.
pub fn toggle(app: &AppHandle, mode: RecordMode) {
    if is_recording(app) {
        stop(app);
    } else {
        start(app, mode);
    }
}

pub fn start(app: &AppHandle, mode: RecordMode) {
    let st = state(app);
    if st.busy.load(Ordering::SeqCst) || st.session.lock().is_some() {
        return;
    }
    let s = st.settings();
    let target_window = platform::foreground_window();
    log::info!("recording start ({mode:?})");
    let rec = match Recording::start(s.input_device.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            log::error!("mic: {e}");
            overlay_error(app, &e.to_string(), None, None);
            return;
        }
    };
    if s.mute_on_record {
        *st.muted_prev.lock() = platform::mute_speakers(true);
    }
    let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let samples = rec.samples.clone();
    let target = if mode == RecordMode::Translate { Some(s.translation_target.clone()) } else { None };
    let live = s.stt.live_preview && st.local.installed(&preview_model(&s, &st));
    *st.session.lock() = Some(ActiveSession { rec, mode, target_window, started: Instant::now(), stop_flag: stop_flag.clone() });

    windows::show_overlay(app);
    emit_overlay(
        app,
        json!({ "phase": "listening", "mode": mode, "style_id": s.active_style, "target": target, "live": live }),
    );
    crate::hotkeys::register_escape(app, true);

    // Mic level for the waveform.
    let app2 = app.clone();
    let flag = stop_flag.clone();
    std::thread::spawn(move || {
        while !flag.load(Ordering::Relaxed) {
            let level = state(&app2).session.lock().as_ref().map(|s| s.rec.level()).unwrap_or(0.0);
            let _ = app2.emit_to("overlay", "overlay-level", level);
            std::thread::sleep(Duration::from_millis(33));
        }
    });

    if live {
        let app3 = app.clone();
        std::thread::spawn(move || live_preview(app3, samples, stop_flag));
    }
}

fn preview_model(s: &Settings, st: &AppState) -> String {
    if st.local.installed(&s.stt.preview_model) {
        s.stt.preview_model.clone()
    } else {
        st.local.any_installed().unwrap_or("base-q5_1").to_string()
    }
}

/// Re-transcribes the tail of the recording every ~0.7 s with a small local model.
/// Audio is committed at pauses so each pass stays short.
fn live_preview(app: AppHandle, samples: Arc<parking_lot::Mutex<Vec<f32>>>, stop: Arc<std::sync::atomic::AtomicBool>) {
    let st = state(&app);
    let s = st.settings();
    let model = preview_model(&s, &st);
    let prompt = s.dictionary.join(", ");
    let mut committed = String::new();
    let mut committed_at = 0usize;
    const SR: usize = audio::SAMPLE_RATE as usize;
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(700));
        let window: Vec<f32> = {
            let all = samples.lock();
            if all.len() <= committed_at {
                continue;
            }
            all[committed_at..].to_vec()
        };
        let level = audio::rms(&window);
        if window.len() < SR / 2 || level < 0.0015 {
            continue;
        }
        // Commit at a trailing pause (≥ 600 ms) once the window has some length, or when it gets long.
        let tail = &window[window.len().saturating_sub(SR * 6 / 10)..];
        let pause = window.len() > SR * 3 && audio::rms(tail) < level * 0.25;
        let long = window.len() > SR * 20;
        let text = match st.local.transcribe(&model, &window, &s.speech_language, &prompt, true) {
            Ok(t) => text::strip_annotations(&t),
            Err(e) => {
                log::warn!("preview failed: {e}");
                return;
            }
        };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let shown = if text::is_hallucination(&text) { String::new() } else { text };
        let full = format!("{committed} {shown}").trim().to_string();
        let _ = app.emit_to("overlay", "overlay-partial", json!({ "text": full }));
        if pause || long {
            committed = full;
            committed_at += window.len();
        }
    }
}

pub fn cancel(app: &AppHandle) {
    let st = state(app);
    let Some(session) = st.session.lock().take() else { return };
    session.stop_flag.store(true, Ordering::SeqCst);
    drop(session.rec);
    after_recording(app);
    windows::hide_overlay(app);
}

fn after_recording(app: &AppHandle) {
    let prev = state(app).muted_prev.lock().take();
    if let Some(prev) = prev {
        platform::mute_speakers(prev);
    }
}

pub fn stop(app: &AppHandle) {
    let st = state(app);
    let Some(session) = st.session.lock().take() else { return };
    session.stop_flag.store(true, Ordering::SeqCst);
    let duration_ms = session.started.elapsed().as_millis() as u64;
    let samples = session.rec.finish();
    log::info!("recording stop: {duration_ms} ms, {} samples", samples.len());
    after_recording(app);
    st.busy.store(true, Ordering::SeqCst);
    emit_overlay(app, json!({ "phase": "transcribing" }));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = process_recording(&app, samples, session.mode, session.target_window, duration_ms).await;
        state(&app).busy.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            log::warn!("dictation failed: {e}");
            let (message, detail, raw) = match e.downcast::<PipelineError>() {
                Ok(pe) => (pe.message, pe.detail, pe.raw),
                Err(e) => (e.to_string(), None, None),
            };
            overlay_error(&app, &message, detail, raw);
        }
    });
}

#[derive(Debug)]
pub struct PipelineError {
    pub message: String,
    pub detail: Option<String>,
    pub raw: Option<String>,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for PipelineError {}

fn perr(message: impl Into<String>, detail: Option<String>, raw: Option<String>) -> anyhow::Error {
    anyhow::Error::new(PipelineError { message: message.into(), detail, raw })
}

async fn process_recording(
    app: &AppHandle,
    samples: Vec<f32>,
    mode: RecordMode,
    target_window: isize,
    duration_ms: u64,
) -> Result<()> {
    let voiced = audio::trim_silence(&samples).to_vec();
    if voiced.len() < (audio::SAMPLE_RATE as usize) / 4 {
        return Err(perr("Nenhuma fala detectada — verifique o microfone.", None, None));
    }
    let (raw, stt_provider) = transcribe_any(app, &voiced).await?;
    let raw = text::strip_annotations(&raw);
    let s = state(app).settings();
    // "Ah." alone is a hesitation, not speech: never send an empty text to the LLM.
    if text::is_hallucination(&raw) || !text::has_words(&text::remove_fillers(&raw, &s.speech_language)) {
        log::info!("no speech in transcript: {raw:?}");
        return Err(perr("Nenhuma fala detectada.", None, None));
    }

    if mode == RecordMode::Ask {
        emit_overlay(app, json!({ "phase": "answer", "query": raw }));
        start_chat(app, raw).await;
        return Ok(());
    }

    let target = if mode == RecordMode::Translate { Some(s.translation_target.clone()) } else { None };
    let style = s.style(&s.active_style);
    let mut result = DictationResult {
        id: new_id(),
        ts: chrono::Local::now().to_rfc3339(),
        mode,
        raw: raw.clone(),
        text: String::new(),
        style_id: style.id.clone(),
        target: target.clone(),
        provider: None,
        stt_provider,
        pasted: false,
        duration_ms,
        warnings: vec![],
    };
    refine_into(app, &mut result, &s).await;

    if s.auto_paste {
        let text = result.text.clone();
        let pasted = tauri::async_runtime::spawn_blocking(move || platform::paste(&text, target_window, false))
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        match pasted {
            Ok(p) => result.pasted = p,
            Err(e) => result.warnings.push(e.to_string()),
        }
    } else if let Err(e) = platform::set_clipboard(&result.text) {
        result.warnings.push(e.to_string());
    }

    state(app).push_history(&result);
    *state(app).last.lock() = Some((result.clone(), target_window));
    let _ = app.emit("history-changed", ());
    emit_overlay(app, json!({ "phase": "done", "result": result }));
    crate::hotkeys::register_escape(app, true);
    Ok(())
}

/// Applies style/translation to `result.raw`, filling text/provider/warnings.
async fn refine_into(app: &AppHandle, result: &mut DictationResult, s: &Settings) {
    let style = s.style(&result.style_id);
    let lang = s.speech_language.as_str();
    let cleaned = if s.remove_fillers { text::remove_fillers(&result.raw, lang) } else { result.raw.clone() };
    let target = result.target.as_deref();

    let llm_prompt = match (&style.kind, target) {
        (StyleKind::Llm, t) => Some(settings::system_prompt(&style.prompt, t, &s.dictionary)),
        (_, Some(t)) => Some(settings::translation_prompt(t)),
        (_, None) => None,
    };
    let source = if style.kind == StyleKind::Verbatim { result.raw.clone() } else { cleaned.clone() };

    let mut text_out = source.clone();
    result.provider = None;
    if !text::has_words(&source) {
        result.text = source;
        return;
    }
    if let Some(system) = llm_prompt {
        let provider_hint = first_llm(app, s).map(display_name).unwrap_or("IA");
        emit_overlay(app, json!({ "phase": "refining", "provider": provider_hint }));
        let user = format!("<transcricao>\n{source}\n</transcricao>");
        let max_tokens = ((source.len() as u32) + 1024).clamp(1024, 8192);
        match llm_chain(app, &[ChatMsg::new("system", system), ChatMsg::new("user", user)], 0.2, max_tokens).await {
            Ok((out, provider)) => {
                let out = text::clean_llm_output(&out);
                if !out.is_empty() {
                    text_out = out;
                    result.provider = Some(provider);
                }
            }
            Err(errors) => {
                let reason = errors.first().map(|e| e.to_string()).unwrap_or_else(|| "nenhuma IA configurada".into());
                result.warnings.push(format!("Sem refinamento por IA ({reason}). Texto entregue limpo, sem IA."));
            }
        }
    }
    if style.kind == StyleKind::Raw {
        text_out = text::raw_style(&text_out);
    }
    result.text = text_out;
}

/// Re-runs the last transcript with another style/target (overlay chips).
pub async fn reprocess(app: &AppHandle, style_id: String, target: Option<String>) -> Result<DictationResult> {
    let (mut result, window) = state(app).last.lock().clone().ok_or_else(|| anyhow!("nada para refazer"))?;
    let s = state(app).settings();
    result.style_id = style_id;
    result.target = target.filter(|t| !t.is_empty());
    result.warnings.clear();
    result.pasted = false;
    refine_into(app, &mut result, &s).await;
    platform::set_clipboard(&result.text).ok();
    state(app).push_history(&result);
    *state(app).last.lock() = Some((result.clone(), window));
    let _ = app.emit("history-changed", ());
    Ok(result)
}

pub fn last_target_window(app: &AppHandle) -> isize {
    state(app).last.lock().as_ref().map(|(_, w)| *w).unwrap_or(0)
}

// ---------------------------------------------------------------- speech to text

async fn transcribe_any(app: &AppHandle, samples: &[f32]) -> Result<(String, String)> {
    let st = state(app);
    let s = st.settings();
    let prompt = s.dictionary.join(", ");
    let lang = s.speech_language.clone();

    let local_model = if st.local.installed(&s.stt.local_model) {
        Some(s.stt.local_model.clone())
    } else {
        st.local.any_installed().map(str::to_string)
    };
    let run_local = |model: String| {
        let local = st.local.clone();
        let samples = samples.to_vec();
        let lang = lang.clone();
        let prompt = prompt.clone();
        async move {
            tauri::async_runtime::spawn_blocking(move || local.transcribe(&model, &samples, &lang, &prompt, false))
                .await
                .map_err(|e| anyhow!(e.to_string()))?
        }
    };

    if s.stt.engine == "local" {
        let model = local_model.ok_or_else(|| {
            perr("Nenhum modelo local baixado. Baixe um em Configurações → Transcrição.", None, None)
        })?;
        let text = run_local(model).await.map_err(|e| perr(e.to_string(), None, None))?;
        return Ok((text, "local".into()));
    }

    let wav = audio::to_wav(samples);
    let mut errors: Vec<ProviderError> = vec![];
    for provider in &s.stt.cloud_order {
        let keys = st.keys.get(provider);
        let Some(cfg) = s.providers.get(provider) else { continue };
        if keys.is_empty() || cfg.stt_model.trim().is_empty() {
            continue;
        }
        for key in keys {
            match cloud::transcribe(&st.http, provider, cfg, &key, &wav, &lang, &prompt).await {
                Ok(t) => return Ok((t, provider.clone())),
                Err(e) => {
                    log::warn!("stt {provider} failed: {} ({})", e.message, e.detail);
                    let next_key = e.key_problem;
                    errors.push(e);
                    if !next_key {
                        break;
                    }
                }
            }
        }
    }
    if let Some(model) = local_model {
        let text = run_local(model).await.map_err(|e| perr(e.to_string(), None, None))?;
        return Ok((text, "local".into()));
    }
    if errors.is_empty() {
        return Err(perr(
            "Configure uma chave (Groq é grátis) ou baixe um modelo local para transcrever.",
            None,
            None,
        ));
    }
    let detail = errors.iter().map(|e| format!("{} — {}", e, e.detail)).collect::<Vec<_>>().join("\n");
    Err(perr(format!("Falha na transcrição: {}", errors[0]), Some(detail), None))
}

// ---------------------------------------------------------------- LLM failover

fn configured(app: &AppHandle, s: &Settings, provider: &str) -> bool {
    let st = state(app);
    let Some(cfg) = s.providers.get(provider) else { return false };
    if cfg.model.trim().is_empty() {
        return false;
    }
    match provider {
        // Keyless local endpoints count as enabled once the user fills the address.
        "ollama" | "custom" => !cfg.base_url.trim().is_empty(),
        _ => st.keys.has(provider),
    }
}

pub fn first_llm<'a>(app: &AppHandle, s: &'a Settings) -> Option<&'a str> {
    s.llm_order.iter().map(String::as_str).find(|p| configured(app, s, p))
}

/// Tries every configured provider (and every key) in the user's order.
pub async fn llm_chain(
    app: &AppHandle,
    messages: &[ChatMsg],
    temperature: f32,
    max_tokens: u32,
) -> std::result::Result<(String, String), Vec<ProviderError>> {
    let st = state(app);
    let s = st.settings();
    let mut errors = vec![];
    for provider in &s.llm_order {
        if !configured(app, &s, provider) {
            continue;
        }
        let cfg = &s.providers[provider];
        let mut keys = st.keys.get(provider);
        if keys.is_empty() {
            keys.push(String::new()); // ollama / keyless custom endpoint
        }
        for key in keys {
            match cloud::chat(&st.http, provider, cfg, &key, messages, temperature, max_tokens).await {
                Ok(text) => return Ok((text, display_name(provider).to_string())),
                Err(e) => {
                    log::warn!("llm {provider} failed: {} ({})", e.message, e.detail);
                    let next_key = e.key_problem;
                    errors.push(e);
                    if !next_key {
                        break;
                    }
                }
            }
        }
    }
    Err(errors)
}

// ---------------------------------------------------------------- voice search assistant

const SEARCH_SYSTEM: &str = "Você é o assistente do FlowVoice: direto, útil e conciso. Responda no idioma da pergunta. Use markdown leve (parágrafos curtos, listas quando ajudar, **negrito** para o essencial). Quando houver resultados de busca, baseie-se neles, não invente dados e cite as fontes pelo número [1], [2]. Sem resultados de busca e se a pergunta exigir dados em tempo real, diga que não tem acesso à internet e sugira configurar a chave Tavily no FlowVoice. Nunca censure palavrões.";

async fn answer(app: &AppHandle, history: &[ChatMessage]) -> ChatMessage {
    let st = state(app);
    let query = history.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
    let mut sources = vec![];
    let mut context = String::new();
    let tavily_keys = st.keys.get("tavily");
    if let Some(key) = tavily_keys.first() {
        match cloud::tavily(&st.http, key, &query).await {
            Ok(r) => {
                if let Some(a) = &r.answer {
                    context.push_str(&format!("Resumo da busca: {a}\n\n"));
                }
                for (i, (title, url, content)) in r.results.iter().enumerate() {
                    context.push_str(&format!("[{}] {title} ({url})\n{content}\n\n", i + 1));
                    sources.push(ChatSource { title: title.clone(), url: url.clone() });
                }
            }
            Err(e) => context.push_str(&format!("(A busca na web falhou: {e})\n")),
        }
    }
    let mut msgs = vec![ChatMsg::new("system", SEARCH_SYSTEM)];
    for m in history.iter().take(history.len().saturating_sub(1)) {
        msgs.push(ChatMsg::new(&m.role, m.content.clone()));
    }
    let last = if context.is_empty() {
        query.clone()
    } else {
        format!("Pergunta: {query}\n\nResultados da busca (hoje é {}):\n{context}", chrono::Local::now().format("%d/%m/%Y"))
    };
    msgs.push(ChatMsg::new("user", last));
    let content = match llm_chain(app, &msgs, 0.4, 2048).await {
        Ok((text, _)) => text::clean_llm_output(&text),
        Err(errors) if errors.is_empty() => {
            "Nenhuma IA configurada. Adicione uma chave grátis (Groq ou Gemini) em Configurações → Provedores.".into()
        }
        Err(errors) => format!("Não consegui responder agora: {}", errors[0]),
    };
    ChatMessage { role: "assistant".into(), content, sources }
}

async fn start_chat(app: &AppHandle, query: String) {
    let snapshot = push_chat(app, ChatMessage { role: "user".into(), content: query, sources: vec![] }, true);
    windows::open_chat(app);
    windows::hide_overlay(app);
    let _ = app.emit_to("chat", "chat-updated", &snapshot);
    let reply = answer(app, &snapshot).await;
    let snapshot = push_chat(app, reply, false);
    let _ = app.emit_to("chat", "chat-updated", &snapshot);
}

fn push_chat(app: &AppHandle, msg: ChatMessage, reset: bool) -> Vec<ChatMessage> {
    let st = state(app);
    let mut chat = st.chat.lock();
    if reset {
        chat.clear();
    }
    chat.push(msg);
    chat.clone()
}

pub async fn chat_send(app: &AppHandle, text: String) -> Vec<ChatMessage> {
    let snapshot = push_chat(app, ChatMessage { role: "user".into(), content: text, sources: vec![] }, false);
    let _ = app.emit_to("chat", "chat-updated", &snapshot);
    let reply = answer(app, &snapshot).await;
    let snapshot = push_chat(app, reply, false);
    let _ = app.emit_to("chat", "chat-updated", &snapshot);
    snapshot
}

/// Mic button inside the chat window. Returns whether it is now recording.
pub async fn chat_toggle_recording(app: &AppHandle) -> Result<bool> {
    let existing = state(app).chat_rec.lock().take();
    match existing {
        None => {
            let device = state(app).settings().input_device;
            let rec = Recording::start(device.as_deref())?;
            *state(app).chat_rec.lock() = Some(rec);
            let _ = app.emit_to("chat", "chat-recording", json!({ "recording": true }));
            Ok(true)
        }
        Some(rec) => {
            let _ = app.emit_to("chat", "chat-recording", json!({ "recording": false }));
            let samples = rec.finish();
            let voiced = audio::trim_silence(&samples).to_vec();
            if voiced.len() < (audio::SAMPLE_RATE as usize) / 4 {
                return Ok(false);
            }
            let (raw, _) = transcribe_any(app, &voiced).await?;
            let raw = text::strip_annotations(&raw);
            if !text::is_hallucination(&raw) {
                let _ = app.emit_to("chat", "chat-transcript", json!({ "text": raw }));
            }
            Ok(false)
        }
    }
}
