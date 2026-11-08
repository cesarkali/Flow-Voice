//! Self-healing provider models.
//!
//! Free providers retire model ids without notice. When a call fails because the model is gone
//! (`ProviderError::model_gone`), we fetch the provider's live `/models` list, rank the candidates
//! with provider-aware rules, verify the best ones with a tiny real request, persist the winner in
//! the settings, tell the UI (`model-healed`) and let the caller retry once. A background check
//! (30 s after launch, then daily) heals configured models that vanished from the list before the
//! user ever hits the error. Only known providers are healed: a model typed for "custom"/"ollama"
//! is never replaced.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::cloud::{self, display_name, ChatMsg};
use crate::settings::ProviderSettings;
use crate::state::AppState;

/// Providers whose model ids we know how to rank (and are allowed to replace).
pub const HEALABLE: [&str; 5] = ["groq", "gemini", "openai", "openrouter", "mistral"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Speech to text (`stt_model`).
    Stt,
    /// Fast rewrites (`model`).
    Chat,
    /// Assistant answers: a stronger chat model (never persisted).
    Strong,
}

pub fn healable(provider: &str) -> bool {
    HEALABLE.contains(&provider)
}

pub fn supports_stt(provider: &str) -> bool {
    matches!(provider, "groq" | "gemini" | "openai" | "mistral")
}

// ---------------------------------------------------------------- name parsing

/// Version / size / snapshot info read from a model id.
#[derive(Debug, Default, PartialEq)]
pub struct ModelName {
    /// First version-like number group ("gemini-2.5-flash" → [2, 5]; "whisper-large-v3" → [3]).
    pub version: Vec<u32>,
    /// Largest parameter count in billions ("llama-3.3-70b" → 70).
    pub size_b: u32,
    /// Pinned snapshot/date suffix ("-001", "-2507", "-2024-08-06").
    pub snapshot: bool,
}

pub fn parse_name(id: &str) -> ModelName {
    let s = id.to_lowercase();
    let s = s.rsplit('/').next().unwrap_or(&s).trim_end_matches(":free").to_string();
    let b = s.as_bytes();
    let mut out = ModelName::default();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && (b[i].is_ascii_digit() || (b[i] == b'.' && i + 1 < b.len() && b[i + 1].is_ascii_digit())) {
            i += 1;
        }
        let run = &s[start..i];
        let first_len = run.split('.').next().map_or(0, str::len);
        let next = b.get(i).copied();
        let after = b.get(i + 1).copied();
        let unit_end = after.map_or(true, |c| !c.is_ascii_alphabetic());
        if matches!(next, Some(b'b') | Some(b'm')) && unit_end {
            // Parameter size: "70b", "8b", "270m", "1.5b".
            let v: f32 = run.parse().unwrap_or(0.0);
            let billions = if next == Some(b'b') { v } else { v / 1000.0 };
            out.size_b = out.size_b.max(billions.round() as u32);
            continue;
        }
        if first_len >= 3 {
            out.snapshot = true; // "001", "2507", "2024", "0905"
            continue;
        }
        // "4o", "3n", "e4b" stay versions; only the first version group counts.
        let prev_is_sep = start == 0 || matches!(b[start - 1], b'-' | b'_' | b'v' | b'.') || b[start - 1].is_ascii_alphabetic();
        if out.version.is_empty() && prev_is_sep {
            out.version = run.split('.').filter_map(|p| p.parse().ok()).collect();
        } else if !out.version.is_empty() && first_len <= 2 && start > 0 && b[start - 1] == b'-' {
            // A second short pure number after the version, like "gemini-2.0-flash-001"'s cousin
            // "mistral-small-3-1" — treat trailing short numbers as a pinned snapshot.
            let pure = next.map_or(true, |c| c == b'-');
            if pure && i == b.len() {
                out.snapshot = true;
            }
        }
    }
    out
}

/// Family = the id without numbers (version/size/date) — "gemini-2.5-flash-lite" and
/// "gemini-3.1-flash-lite" share a family; "whisper-large-v3" and "-v3-turbo" don't.
pub fn family(id: &str) -> String {
    let s = id.to_lowercase();
    let s = s.trim_end_matches(":free").trim_end_matches("-latest");
    let mut out = String::new();
    let mut last_hash = false;
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && last_hash) {
            if !last_hash {
                out.push('#');
                last_hash = true;
            }
        } else {
            last_hash = false;
            out.push(c);
        }
    }
    // Drop pinned snapshot tails so "x-#" and "x" match.
    let mut f = out.as_str();
    while let Some(stripped) = f.strip_suffix("-#") {
        f = stripped;
    }
    f.to_string()
}

fn has_any(s: &str, words: &[&str]) -> bool {
    words.iter().any(|w| s.contains(w))
}

/// Words that mark a model that is unstable or not a plain text/stt model.
const NEVER: [&str; 30] = [
    "preview", "-exp", "experimental", "embed", "tts", "image", "imagen", "veo", "guard", "moderation",
    "realtime", "native-audio", "-live", "lyria", "dall-e", "babbage", "davinci", "search", "aqa",
    "robotics", "computer-use", "ocr", "learnlm", "nano-banana", "safeguard", "orpheus", "playai",
    "deep-research", "diarize", "thinking",
];

/// Rank tier for a candidate (lower is better); `None` = never use for this kind.
fn tier(provider: &str, kind: Kind, id: &str) -> Option<u8> {
    let m = id.to_lowercase();
    if has_any(&m, &NEVER) {
        return None;
    }
    match (provider, kind) {
        ("groq", Kind::Stt) => {
            if !m.contains("whisper") {
                return None;
            }
            Some(if m.contains("whisper-large") && !m.contains("turbo") {
                0
            } else if m.contains("whisper-large") {
                1
            } else {
                2
            })
        }
        ("groq", Kind::Chat | Kind::Strong) => {
            if has_any(&m, &["whisper", "distil", "compound", "allam", "prompt-guard"]) {
                return None;
            }
            let order: &[&str] = if kind == Kind::Chat {
                &["gpt-oss-20b", "gpt-oss", "llama-4-scout", "llama-3.3-70b", "llama-4", "kimi", "qwen", "llama"]
            } else {
                &["gpt-oss-120b", "gpt-oss", "kimi", "llama-4-maverick", "llama-3.3-70b", "qwen", "llama-4", "llama"]
            };
            Some(order.iter().position(|w| m.contains(w)).map_or(9, |p| p as u8))
        }
        ("gemini", Kind::Stt) => {
            // Only Gemini flash models take audio input on the free tier (no gemma, no pro).
            if !m.starts_with("gemini-") || !m.contains("flash") || m.contains("gemma") {
                return None;
            }
            let latest = m.contains("latest");
            Some(match (m.contains("flash-lite"), latest) {
                (true, false) => 0,
                (false, false) => 1,
                (true, true) => 2,
                (false, true) => 3,
            })
        }
        ("gemini", Kind::Chat | Kind::Strong) => {
            if m.contains("gemma") {
                return Some(8);
            }
            if !m.starts_with("gemini-") {
                return None;
            }
            let latest = m.contains("latest");
            let base = match (kind, m.contains("flash-lite"), m.contains("flash"), m.contains("pro")) {
                (Kind::Chat, true, _, _) => 0,
                (Kind::Chat, false, true, _) => 1,
                (Kind::Strong, false, true, _) => 0,
                (Kind::Strong, true, _, _) => 1,
                (_, _, _, true) => 3,
                _ => return None,
            };
            Some(base + if latest { 4 } else { 0 })
        }
        ("openai", Kind::Stt) => {
            if m.contains("transcribe") {
                Some(if m.contains("mini") { 1 } else { 0 })
            } else if m.starts_with("whisper") {
                Some(2)
            } else {
                None
            }
        }
        ("openai", Kind::Chat | Kind::Strong) => {
            if has_any(&m, &["audio", "transcribe", "whisper", "codex", "-pro", "chat-latest", "instruct", "oss", "sora", "gpt-image"]) {
                return None;
            }
            let gpt = m.starts_with("gpt-");
            let small = has_any(&m, &["mini", "nano"]);
            let nano = m.contains("nano");
            Some(match (kind, gpt, small, nano) {
                (Kind::Chat, true, true, false) => 0,
                (Kind::Chat, true, false, _) => 1,
                (Kind::Chat, true, true, true) => 2,
                (Kind::Strong, true, false, _) => 0,
                (Kind::Strong, true, true, false) => 1,
                (Kind::Strong, true, true, true) => 2,
                (_, false, _, _) if m.starts_with('o') && m.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) => 5,
                _ => return None,
            })
        }
        ("openrouter", Kind::Chat | Kind::Strong) => {
            if !(m.ends_with(":free")) {
                return None;
            }
            if has_any(&m, &["-vl", "vision", "r1", "coder", "math", "uncensored", "roleplay", "nemotron-nano"]) {
                return Some(7);
            }
            let order = ["google/gemma", "meta-llama/llama", "mistralai/", "qwen/", "deepseek/", "openai/gpt-oss"];
            Some(order.iter().position(|w| m.starts_with(w)).map_or(6, |p| p as u8))
        }
        ("mistral", Kind::Stt) => {
            if !m.contains("voxtral") {
                return None;
            }
            Some(match (m.contains("mini"), m.ends_with("-latest")) {
                (true, true) => 0,
                (true, false) => 1,
                _ => return None, // voxtral-small doesn't serve /audio/transcriptions
            })
        }
        ("mistral", Kind::Chat | Kind::Strong) => {
            if has_any(&m, &["voxtral", "embed", "moderation", "ocr", "codestral", "devstral", "pixtral", "magistral"]) {
                return None;
            }
            let latest = m.ends_with("-latest");
            let order: &[&str] = if kind == Kind::Chat {
                &["mistral-small", "mistral-medium", "ministral", "mistral-large", "open-mistral"]
            } else {
                &["mistral-medium", "mistral-large", "mistral-small", "ministral", "open-mistral"]
            };
            let fam = order.iter().position(|w| m.starts_with(w)).map_or(6, |p| p as u8);
            Some(if latest { fam } else { fam + 10 })
        }
        _ => None,
    }
}

/// Candidates for `kind`, best first. `old` (the gone model) is excluded; models of the same
/// family as `old` come first (a retired "gemini-2.5-flash-lite" becomes the newest flash-lite).
pub fn rank(provider: &str, kind: Kind, ids: &[String], old: &str) -> Vec<String> {
    let old_fam = family(old);
    let old_l = old.to_lowercase();
    let mut scored: Vec<(u8, u8, ModelName, &String)> = ids
        .iter()
        .filter(|id| id.to_lowercase() != old_l)
        .filter_map(|id| {
            let t = tier(provider, kind, id)?;
            let affinity = if !old.is_empty() && family(id) == old_fam { 0 } else { 1 };
            Some((affinity, t, parse_name(id), id))
        })
        .collect();
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(b.2.version.cmp(&a.2.version))
            .then(a.2.snapshot.cmp(&b.2.snapshot))
            .then(b.2.size_b.cmp(&a.2.size_b))
            .then(a.3.len().cmp(&b.3.len()))
            .then(a.3.cmp(b.3))
    });
    scored.into_iter().map(|(.., id)| id.clone()).collect()
}

// ---------------------------------------------------------------- live list cache

struct Cache {
    lists: HashMap<String, (Instant, Vec<String>)>,
    /// Last heal attempt per (provider, kind, failed model): never retry the same thing in a loop.
    attempts: HashMap<(String, Kind, String), Instant>,
    notices: Vec<Notice>,
}

fn cache() -> &'static Mutex<Cache> {
    static C: std::sync::OnceLock<Mutex<Cache>> = std::sync::OnceLock::new();
    C.get_or_init(|| Mutex::new(Cache { lists: HashMap::new(), attempts: HashMap::new(), notices: vec![] }))
}

/// One heal at a time (preview ticks can fail together).
fn heal_lock() -> &'static tokio::sync::Mutex<()> {
    static L: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    L.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[derive(Clone, Debug, Serialize)]
pub struct Notice {
    pub provider: String,
    pub kind: Kind,
    pub old: String,
    pub new: String,
    pub message: String,
    pub ts: String,
}

/// Notices not yet shown by the settings window (it may have been closed when they happened).
pub fn take_notices() -> Vec<Notice> {
    std::mem::take(&mut cache().lock().notices)
}

async fn fetch_list(app: &AppHandle, provider: &str, cfg: &ProviderSettings, max_age: Duration) -> Option<Vec<String>> {
    if let Some((t, l)) = cache().lock().lists.get(provider) {
        if t.elapsed() < max_age && !l.is_empty() {
            return Some(l.clone());
        }
    }
    let st = app.state::<AppState>();
    for key in st.keys.get(provider) {
        match cloud::list_models(&st.http, provider, cfg, &key).await {
            Ok(list) if !list.is_empty() => {
                cache().lock().lists.insert(provider.to_string(), (Instant::now(), list.clone()));
                return Some(list);
            }
            Ok(_) => return None,
            Err(e) => log::warn!("model list {provider}: {e}"),
        }
    }
    None
}

/// Tiny real request with `model`: Ok(true) works, Ok(false) model refused, Err = can't tell now
/// (rate limit, network) so nothing should be changed.
async fn verify(app: &AppHandle, provider: &str, cfg: &ProviderSettings, kind: Kind, model: &str) -> Result<bool, String> {
    let st = app.state::<AppState>();
    let mut cfg = cfg.clone();
    let mut last = String::from("sem chave");
    for key in st.keys.get(provider).into_iter().take(3) {
        let res = match kind {
            Kind::Stt => {
                cfg.stt_model = model.to_string();
                // 1.2 s of quiet tone: validates model + audio input without real speech.
                let samples: Vec<f32> = (0..19_200).map(|i| ((i as f32) * 0.37).sin() * 0.002).collect();
                cloud::transcribe(&st.http, provider, &cfg, &key, &crate::audio::to_wav(&samples), "pt", "").await
            }
            Kind::Chat | Kind::Strong => {
                cfg.model = model.to_string();
                let msgs = [ChatMsg::new("system", "Responda exatamente: OK"), ChatMsg::new("user", "teste")];
                cloud::chat(&st.http, provider, &cfg, &key, &msgs, 0.0, 256).await
            }
        };
        match res {
            Ok(_) => return Ok(true),
            Err(e) if e.model_gone => return Ok(false),
            Err(e) if e.key_problem => last = e.detail,
            // Empty answer, 5xx, odd 400: this model doesn't serve us — try the next candidate.
            Err(e) => {
                log::info!("heal verify {provider} {model}: {}", e.detail);
                return Ok(false);
            }
        }
    }
    Err(last)
}

fn current(cfg: &ProviderSettings, kind: Kind) -> &str {
    match kind {
        Kind::Stt => &cfg.stt_model,
        _ => &cfg.model,
    }
}

/// Replaces `failed` (a model that just failed as gone, or vanished from the list) with the best
/// working model. Returns the model to use now: the new one, or the one another task already
/// healed to. `None` = nothing better found (caller keeps its error).
pub async fn heal(app: &AppHandle, provider: &str, kind: Kind, failed: &str) -> Option<String> {
    if !healable(provider) || (kind == Kind::Stt && !supports_stt(provider)) || kind == Kind::Strong {
        return None;
    }
    let _guard = heal_lock().lock().await;
    let st = app.state::<AppState>();
    let s = st.settings();
    let cfg = s.providers.get(provider)?.clone();
    let now_model = current(&cfg, kind).to_string();
    if now_model != failed && !now_model.trim().is_empty() {
        return Some(now_model); // healed by a concurrent call
    }
    let attempt_key = (provider.to_string(), kind, failed.to_string());
    {
        let mut c = cache().lock();
        if let Some(t) = c.attempts.get(&attempt_key) {
            if t.elapsed() < Duration::from_secs(600) {
                return None;
            }
        }
        c.attempts.insert(attempt_key, Instant::now());
    }
    let list = fetch_list(app, provider, &cfg, Duration::from_secs(300)).await?;
    let candidates = rank(provider, kind, &list, failed);
    log::info!("heal {provider} {kind:?}: '{failed}' gone, candidates {:?}", candidates.iter().take(5).collect::<Vec<_>>());
    for cand in candidates.into_iter().take(4) {
        match verify(app, provider, &cfg, kind, &cand).await {
            Ok(true) => {
                persist(app, provider, kind, failed, &cand);
                return Some(cand);
            }
            Ok(false) => continue,
            Err(why) => {
                log::warn!("heal {provider}: can't verify now ({why})");
                // Allow another attempt soon: this wasn't the model's fault.
                cache().lock().attempts.remove(&(provider.to_string(), kind, failed.to_string()));
                return None;
            }
        }
    }
    None
}

fn persist(app: &AppHandle, provider: &str, kind: Kind, old: &str, new: &str) {
    let st = app.state::<AppState>();
    let mut s = st.settings();
    let Some(cfg) = s.providers.get_mut(provider) else { return };
    match kind {
        Kind::Stt => cfg.stt_model = new.to_string(),
        _ => cfg.model = new.to_string(),
    }
    match st.save_settings(s) {
        Ok(saved) => crate::commands::broadcast_settings(app, &saved, None),
        Err(e) => log::warn!("heal: save failed: {e}"),
    }
    let args: [&dyn std::fmt::Display; 3] = [&display_name(provider), &old, &new];
    let message = if kind == Kind::Stt {
        crate::i18n::trf("{}: o modelo {} foi descontinuado — trocado para {} automaticamente (transcrição).", &args)
    } else {
        crate::i18n::trf("{}: o modelo {} foi descontinuado — trocado para {} automaticamente (texto).", &args)
    };
    log::warn!("{message}");
    let notice = Notice {
        provider: provider.into(),
        kind,
        old: old.into(),
        new: new.into(),
        message,
        ts: chrono::Local::now().to_rfc3339(),
    };
    cache().lock().notices.push(notice.clone());
    let _ = app.emit("model-healed", &notice);
}

/// Stronger chat model for the assistant, chosen from the live list when we have it cached.
/// Only "fast" defaults are upgraded (gpt-oss-20b, flash-lite); anything else stays as configured.
pub fn strong_model(provider: &str, configured: &str) -> String {
    let fast = match provider {
        "groq" => configured.contains("gpt-oss-20b"),
        "gemini" => configured.contains("flash-lite"),
        _ => false,
    };
    if !fast {
        return configured.to_string();
    }
    let fallback = cloud::strong_model(provider, configured);
    let list = cache().lock().lists.get(provider).map(|(_, l)| l.clone());
    match list {
        Some(list) => rank(provider, Kind::Strong, &list, "").into_iter().next().unwrap_or(fallback),
        None => fallback,
    }
}

/// Forgets a cached list (e.g. the strong model we picked from it was refused).
pub fn invalidate(provider: &str) {
    cache().lock().lists.remove(provider);
}

/// Daily prevention: refresh the lists of providers with keys and heal configured models that
/// disappeared (after confirming they really fail — some aliases aren't listed).
pub async fn check_all(app: &AppHandle) {
    let st = app.state::<AppState>();
    let s = st.settings();
    for provider in HEALABLE {
        if !st.keys.has(provider) {
            continue;
        }
        let Some(cfg) = s.providers.get(provider).cloned() else { continue };
        let Some(list) = fetch_list(app, provider, &cfg, Duration::from_secs(60)).await else { continue };
        let listed = |m: &str| list.iter().any(|id| id.eq_ignore_ascii_case(m));
        for kind in [Kind::Chat, Kind::Stt] {
            if kind == Kind::Stt && !supports_stt(provider) {
                continue;
            }
            let model = current(&cfg, kind).trim().to_string();
            if model.is_empty() || listed(&model) {
                continue;
            }
            match verify(app, provider, &cfg, kind, &model).await {
                Ok(true) => log::info!("model check {provider}: '{model}' not listed but works"),
                Ok(false) => {
                    heal(app, provider, kind, &model).await;
                }
                Err(_) => {}
            }
        }
    }
}

pub fn start_background(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            check_all(&app).await;
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_versions_sizes_snapshots() {
        assert_eq!(parse_name("gemini-2.5-flash-lite").version, vec![2, 5]);
        assert_eq!(parse_name("whisper-large-v3-turbo").version, vec![3]);
        let l = parse_name("llama-3.3-70b-versatile");
        assert_eq!((l.version.clone(), l.size_b), (vec![3, 3], 70));
        let g = parse_name("openai/gpt-oss-120b");
        assert_eq!((g.version.is_empty(), g.size_b), (true, 120));
        assert!(parse_name("gemini-2.0-flash-001").snapshot);
        assert!(parse_name("voxtral-mini-2507").snapshot);
        assert!(parse_name("gpt-4o-2024-08-06").snapshot);
        assert!(!parse_name("gemini-2.5-flash").snapshot);
        assert_eq!(parse_name("google/gemma-3-27b-it:free").size_b, 27);
        assert_eq!(parse_name("gpt-4.1-mini").version, vec![4, 1]);
    }

    #[test]
    fn family_ignores_numbers() {
        assert_eq!(family("gemini-2.5-flash-lite"), family("gemini-3.1-flash-lite"));
        assert_ne!(family("gemini-2.5-flash-lite"), family("gemini-2.5-flash"));
        assert_ne!(family("whisper-large-v3"), family("whisper-large-v3-turbo"));
        assert_eq!(family("gemini-2.0-flash-001"), family("gemini-2.5-flash"));
    }

    /// Real Groq /models (2026-10-30).
    const GROQ: &[&str] = &[
        "allam-2-7b", "canopylabs/orpheus-arabic-saudi", "canopylabs/orpheus-v1-english",
        "meta-llama/llama-prompt-guard-2-22m", "meta-llama/llama-prompt-guard-2-86m", "openai/gpt-oss-120b",
        "openai/gpt-oss-20b", "openai/gpt-oss-safeguard-20b", "qwen/qwen3.8-27b", "whisper-large-v3",
        "whisper-large-v3-turbo", "groq/compound",
    ];

    #[test]
    fn groq_stt_prefers_large_v3_then_turbo() {
        let r = rank("groq", Kind::Stt, &ids(GROQ), "distil-whisper-large-v3-en");
        assert_eq!(r, ids(&["whisper-large-v3", "whisper-large-v3-turbo"]));
        let r = rank("groq", Kind::Stt, &ids(GROQ), "whisper-large-v3");
        assert_eq!(r[0], "whisper-large-v3-turbo");
        // A future v4 wins over the turbo when v3 is retired.
        let mut l = ids(GROQ);
        l.push("whisper-large-v4".into());
        assert_eq!(rank("groq", Kind::Stt, &l, "whisper-large-v3")[0], "whisper-large-v4");
    }

    #[test]
    fn groq_chat_skips_guards_tts_and_whisper() {
        let r = rank("groq", Kind::Chat, &ids(GROQ), "llama3-8b-8192");
        assert_eq!(r[0], "openai/gpt-oss-20b");
        assert!(!r.iter().any(|m| m.contains("guard") || m.contains("tts") || m.contains("whisper") || m.contains("compound")));
        // gpt-oss-20b retired → stays in the family (120b) before jumping to llama.
        let r = rank("groq", Kind::Chat, &ids(GROQ), "openai/gpt-oss-20b");
        assert_eq!(r[0], "openai/gpt-oss-120b");
        assert_eq!(rank("groq", Kind::Strong, &ids(GROQ), "")[0], "openai/gpt-oss-120b");
    }

    /// Real Gemini /models via the OpenAI-compatible endpoint (2026-10-30).
    const GEMINI: &[&str] = &[
        "antigravity-preview-05-2026", "antigravity-preview-latest", "aqa", "deep-research-preview-04-2026",
        "gemini-2.5-computer-use-preview-10-2025", "gemini-2.5-flash", "gemini-2.5-flash-image",
        "gemini-2.5-flash-lite", "gemini-2.5-flash-native-audio-latest", "gemini-2.5-flash-native-audio-preview-12-2025",
        "gemini-2.5-flash-preview-tts", "gemini-2.5-pro", "gemini-2.5-pro-preview-tts", "gemini-3-flash-preview",
        "gemini-3-pro-image", "gemini-3.1-flash-image", "gemini-3.1-flash-lite", "gemini-3.1-flash-lite-image",
        "gemini-3.1-flash-lite-preview", "gemini-3.1-flash-live-preview", "gemini-3.1-flash-tts-preview",
        "gemini-3.1-pro-preview", "gemini-3.5-flash", "gemini-3.5-flash-lite", "gemini-3.5-live-translate-preview",
        "gemini-3.5-transcribe", "gemini-3.5-transcribe-live", "gemini-3.6-flash", "gemini-3.7-flash",
        "gemini-3.8-flash", "gemini-3.8-flash-lite-tts", "gemini-3.8-flash-tts", "gemini-3.8-live",
        "gemini-3.8-live-extended-thinking", "gemini-4-argon", "gemini-embedding-001", "gemini-embedding-2",
        "gemini-flash-latest", "gemini-flash-lite-latest", "gemini-nano-banana-2.1", "gemini-omni-1.1-flash",
        "gemini-omni-flash-preview", "gemini-pro-latest", "gemini-robotics-er-2-preview", "gemma-4-26b-a4b-it",
        "gemma-4-31b-it", "imagen-4.0-generate-001", "lyria-3.5", "lyria-realtime-exp", "nano-banana-pro-preview",
    ];

    #[test]
    fn gemini_stt_newest_stable_flash_lite() {
        let r = rank("gemini", Kind::Stt, &ids(GEMINI), "gemini-2.0-flash-lite");
        assert_eq!(r[0], "gemini-3.5-flash-lite");
        // Real case: 2.5-flash-lite is still listed but answers 404 "no longer available".
        let r = rank("gemini", Kind::Stt, &ids(GEMINI), "gemini-2.5-flash-lite");
        assert_eq!(&r[..2], &ids(&["gemini-3.5-flash-lite", "gemini-3.1-flash-lite"])[..]);
        assert!(r.iter().all(|m| m.starts_with("gemini-") && m.contains("flash")));
        assert!(!r.iter().any(|m| has_any(m, &["preview", "tts", "image", "native-audio", "gemma", "pro", "exp"])));
        // Retired flash (non-lite) for STT: same family first.
        assert_eq!(rank("gemini", Kind::Stt, &ids(GEMINI), "gemini-2.0-flash")[0], "gemini-3.8-flash");
    }

    #[test]
    fn gemini_chat_and_strong() {
        assert_eq!(rank("gemini", Kind::Chat, &ids(GEMINI), "gemini-1.5-flash-8b")[0], "gemini-3.5-flash-lite");
        assert_eq!(rank("gemini", Kind::Strong, &ids(GEMINI), "")[0], "gemini-3.8-flash");
        let chat = rank("gemini", Kind::Chat, &ids(GEMINI), "");
        assert!(!chat.iter().any(|m| has_any(m, &["argon", "transcribe", "live", "tts", "image", "robotics", "nano-banana"])));
        assert_eq!(strong_model("gemini", "gemini-9-pro"), "gemini-9-pro");
    }

    #[test]
    fn openrouter_only_free() {
        let l = ids(&[
            "deepseek/deepseek-r1-0528:free", "google/gemma-3-12b-it:free", "google/gemma-3-27b-it:free",
            "google/gemma-4-31b-it:free", "meta-llama/llama-3.3-70b-instruct:free", "openai/gpt-4o",
            "qwen/qwen2.5-vl-72b-instruct:free", "openrouter/free",
        ]);
        let r = rank("openrouter", Kind::Chat, &l, "google/gemma-4-31b-it:free");
        assert_eq!(r[0], "google/gemma-3-27b-it:free");
        assert!(r.iter().all(|m| m.ends_with(":free")));
        assert!(r[r.len() - 2..].iter().all(|m| m.contains("-vl-") || m.contains("-r1")));
    }

    #[test]
    fn mistral_prefers_latest_aliases() {
        let l = ids(&[
            "codestral-latest", "magistral-medium-latest", "ministral-8b-latest", "mistral-embed",
            "mistral-large-latest", "mistral-medium-2505", "mistral-medium-latest", "mistral-moderation-latest",
            "mistral-ocr-latest", "mistral-small-2506", "mistral-small-latest", "open-mistral-nemo",
            "pixtral-12b-2409", "voxtral-mini-2507", "voxtral-mini-latest", "voxtral-small-latest",
        ]);
        assert_eq!(rank("mistral", Kind::Chat, &l, "mistral-tiny")[0], "mistral-small-latest");
        // Same size class first (a pinned small), then the next alias.
        assert_eq!(&rank("mistral", Kind::Chat, &l, "mistral-small-latest")[..2], &ids(&["mistral-small-2506", "mistral-medium-latest"])[..]);
        assert_eq!(rank("mistral", Kind::Stt, &l, "voxtral-mini-2407"), ids(&["voxtral-mini-latest", "voxtral-mini-2507"]));
    }

    #[test]
    fn openai_stt_and_chat() {
        let l = ids(&[
            "dall-e-3", "gpt-4o", "gpt-4o-2024-08-06", "gpt-4o-audio-preview", "gpt-4o-mini-transcribe",
            "gpt-4o-mini-tts", "gpt-4o-realtime-preview", "gpt-4o-transcribe", "gpt-4o-transcribe-diarize",
            "gpt-5", "gpt-5-mini", "gpt-5-nano", "gpt-5-pro", "gpt-5.1", "gpt-5.1-mini", "gpt-image-1",
            "o3", "o4-mini", "omni-moderation-latest", "text-embedding-3-small", "whisper-1",
        ]);
        let stt = rank("openai", Kind::Stt, &l, "gpt-transcribe");
        assert_eq!(stt, ids(&["gpt-4o-transcribe", "gpt-4o-mini-transcribe", "whisper-1"]));
        assert_eq!(rank("openai", Kind::Chat, &l, "gpt-6-luna")[0], "gpt-5.1-mini");
        assert_eq!(rank("openai", Kind::Strong, &l, "")[0], "gpt-5.1");
    }

    #[test]
    fn never_heals_unknown_providers_or_stt_to_chat_models() {
        assert!(rank("ollama", Kind::Chat, &ids(&["llama3"]), "gemma3:4b").is_empty());
        assert!(rank("custom", Kind::Chat, &ids(&["x"]), "y").is_empty());
        assert!(rank("openrouter", Kind::Stt, &ids(&["google/gemma-3-27b-it:free"]), "").is_empty());
        assert!(rank("groq", Kind::Stt, &ids(&["llama-3.3-70b-versatile"]), "whisper-large-v3").is_empty());
        assert!(!healable("ollama") && !healable("custom"));
    }
}

#[cfg(test)]
mod probe {
    use crate::cloud;
    use crate::settings::default_providers;

    pub fn load_wav(path: &str) -> Vec<f32> {
        let mut r = hound::WavReader::open(path).unwrap();
        r.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect()
    }

    /// Prints live model lists and short-window STT latency (PROBE_OUT / PROBE_OUT2 files).
    #[tokio::test]
    #[ignore]
    async fn live_probe_models() {
        let keys = crate::keys::KeyStore::new(std::env::temp_dir().join("fv-probe"));
        let http = cloud::http();
        let provs = default_providers();
        let mut out = String::new();
        for p in ["groq", "gemini", "openrouter", "mistral", "openai"] {
            let k = keys.get(p);
            out.push_str(&format!("== {p} ({} keys)\n", k.len()));
            let Some(key) = k.first() else { continue };
            match cloud::list_models(&http, p, &provs[p], key).await {
                Ok(ids) => out.push_str(&format!("{}\n", ids.join("\n"))),
                Err(e) => out.push_str(&format!("ERR {e}\n")),
            }
        }
        std::fs::write(std::env::var("PROBE_OUT").unwrap_or("probe.txt".into()), &out).unwrap();
        let wav = load_wav(r"C:\Dev\ST\tools\dev\giria_quiet.wav");
        let mut out2 = String::new();
        for p in ["groq", "gemini"] {
            let ks = keys.get(p);
            if ks.is_empty() {
                continue;
            }
            for (i, secs) in [1.5f32, 2.5, 4.0, 6.0, 9.0].iter().enumerate() {
                let n = ((*secs * 16000.0) as usize).min(wav.len());
                let w = crate::audio::to_wav(&crate::audio::normalize(&wav[..n]));
                let key = &ks[i % ks.len()];
                let t0 = std::time::Instant::now();
                let r = cloud::transcribe(&http, p, &provs[p], key, &w, "pt", "").await;
                out2.push_str(&format!("{p} {secs}s {}ms: {:?}\n", t0.elapsed().as_millis(), r.map_err(|e| e.detail)));
            }
        }
        std::fs::write(std::env::var("PROBE_OUT2").unwrap_or("probe2.txt".into()), &out2).unwrap();
    }
}

#[cfg(test)]
mod probe2 {
    use crate::cloud;
    use crate::settings::default_providers;

    /// Gemini STT model alternatives (latency/accuracy on short windows) + Groq rate-limit headers.
    #[tokio::test]
    #[ignore]
    async fn live_probe_stt_variants() {
        let keys = crate::keys::KeyStore::new(std::env::temp_dir().join("fv-probe"));
        let http = cloud::http();
        let wav = super::probe::load_wav(r"C:\Dev\ST\tools\dev\giria_quiet.wav");
        let mut out = String::new();
        let gk = keys.get("gemini");
        let mut ki = 0;
        for model in ["gemini-3.5-transcribe", "gemini-3.5-flash-lite", "gemini-3.1-flash-lite", "gemini-2.5-flash-lite"] {
            let mut cfg = default_providers()["gemini"].clone();
            cfg.stt_model = model.into();
            for secs in [2.5f32, 6.0] {
                let n = ((secs * 16000.0) as usize).min(wav.len());
                let w = crate::audio::to_wav(&crate::audio::normalize(&wav[..n]));
                ki += 1;
                let t0 = std::time::Instant::now();
                let r = cloud::transcribe(&http, "gemini", &cfg, &gk[ki % gk.len()], &w, "pt", "").await;
                out.push_str(&format!("{model} {secs}s {}ms: {:?}\n", t0.elapsed().as_millis(), r.map_err(|e| e.detail)));
            }
        }
        let gq = keys.get("groq");
        let w = crate::audio::to_wav(&crate::audio::normalize(&wav[..32000]));
        let part = reqwest::multipart::Part::bytes(w).file_name("a.wav").mime_str("audio/wav").unwrap();
        let form = reqwest::multipart::Form::new().part("file", part).text("model", "whisper-large-v3").text("language", "pt");
        let resp = http.post("https://api.groq.com/openai/v1/audio/transcriptions").bearer_auth(&gq[0]).multipart(form).send().await.unwrap();
        for (k, v) in resp.headers() {
            if k.as_str().contains("ratelimit") || k.as_str().contains("retry") {
                out.push_str(&format!("groq hdr {k}: {v:?}\n"));
            }
        }
        std::fs::write(std::env::var("PROBE_OUT2").unwrap_or("probe3.txt".into()), &out).unwrap();
    }
}
