//! Cloud live preview: re-transcribes the whole current phrase about every second while the
//! user speaks, spreading requests over "lanes" so no provider's rate limit is exceeded.
//!
//! Lanes (measured 2026-10-30, giria_quiet.wav):
//! - Groq whisper: ~0.35-0.5 s per request, near-final accuracy, but the free tier allows ~20
//!   requests/min per MODEL for the whole organization (extra keys from the same account don't
//!   help). whisper-large-v3 and whisper-large-v3-turbo have separate limits, so with both the
//!   preview can refresh every ~1.5 s instead of ~3 s. Groq ticks are authoritative.
//! - Gemini flash-lite (input_audio): ~1.3-2 s per request and slightly different wording
//!   ("barulho" vs "bagulho"), so it never replaces what Groq wrote: a Gemini result only
//!   APPENDS the words spoken after the last Groq text (aligned on the last shown words), shown
//!   as not-yet-stable. Each key is its own project (own limits), so several keys add capacity.
//!   When Gemini is the first provider (no Groq key) it's the authoritative lane instead.
//!
//! First guess: whisper invents text on very short windows ("Boa noite.", "No ano de 2017,").
//! A phrase's first text needs >= 2 s of voiced audio, and below 3 s it is only shown once a
//! second result agrees with it (shares a word).

use parking_lot::Mutex;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::audio;
use crate::cloud;
use crate::settings::{ProviderSettings, Settings};
use crate::state::AppState;
use crate::text;

const SR: usize = audio::SAMPLE_RATE as usize;
/// Voiced audio before a phrase's first request.
const FIRST_MIN_VOICED: f32 = 2.0;
/// Below this, a phrase's first text must be confirmed by the next result.
const FIRST_TRUSTED_VOICED: f32 = 3.0;
/// Target refresh period (any lane).
const TARGET_GAP: Duration = Duration::from_millis(950);
const MAX_IN_FLIGHT: usize = 3;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Role {
    /// Full re-transcription; replaces the phrase text.
    Primary,
    /// Same cadence as Primary, but once the phrase has text it only appends the words after it
    /// (whisper-large-v3-turbo next to v3: keeps v3's wording instead of flipping every tick).
    /// Replaces the text only when it can't be aligned.
    Extend,
    /// Opportunistic (slow, different style): only appends words after the primary text, and is
    /// only sent when its answer can land before the next primary one.
    Fill,
}

struct Lane {
    name: String,
    provider: String,
    cfg: ProviderSettings,
    keys: Vec<String>,
    role: Role,
    /// Minimum spacing between requests of this lane (rate limit).
    gap: Duration,
    /// Limits are per key (separate projects) rather than per account.
    per_key: bool,
    next_ok: Instant,
    key_next: usize,
    key_until: Vec<Instant>,
    /// Alternate model added only for capacity: dropped if it ever fails as gone.
    optional: bool,
    disabled: bool,
    lat_ms: f32,
    sent: u32,
    ok: u32,
    limited: u32,
    used: u32,
}

impl Lane {
    fn new(provider: &str, cfg: ProviderSettings, keys: Vec<String>, role: Role, optional: bool) -> Self {
        let n = keys.len().max(1) as u32;
        let (gap, per_key, lat) = match provider {
            // ~20 RPM per model per organization.
            "groq" => (Duration::from_millis(3100), false, 450.0),
            // Free tier ~15 RPM per project: 4 s per key.
            "gemini" => ((Duration::from_millis(4000) / n).max(Duration::from_millis(600)), true, 1700.0),
            _ => ((Duration::from_millis(3100) / n).max(Duration::from_millis(900)), false, 1200.0),
        };
        let now = Instant::now();
        Self {
            name: format!("{provider}/{}", cfg.stt_model),
            provider: provider.into(),
            key_until: vec![now; keys.len()],
            cfg,
            keys,
            role,
            gap,
            per_key,
            next_ok: now,
            key_next: 0,
            optional,
            disabled: false,
            lat_ms: lat,
            sent: 0,
            ok: 0,
            limited: 0,
            used: 0,
        }
    }

    fn ready(&self, now: Instant) -> bool {
        !self.disabled && now >= self.next_ok && self.key_until.iter().any(|&t| t <= now)
    }

    /// Picks a key and books the request.
    fn take(&mut self, now: Instant) -> Option<usize> {
        let n = self.keys.len();
        let i = (0..n).map(|k| (self.key_next + k) % n).find(|&i| self.key_until[i] <= now)?;
        self.key_next = i + 1;
        self.next_ok = now + self.gap;
        if self.per_key {
            // A key can't be reused before its own spacing (gap * keys).
            self.key_until[i] = now + self.gap * n as u32;
        }
        self.sent += 1;
        Some(i)
    }
}

/// Builds the lanes for the user's STT providers and keys. Empty = no cloud preview.
fn build_lanes(app: &AppHandle, s: &Settings) -> Vec<Lane> {
    let st = app.state::<AppState>();
    let with_keys = |p: &str| -> Option<(ProviderSettings, Vec<String>)> {
        let cfg = s.providers.get(p)?.clone();
        let keys = st.keys.get(p);
        (!keys.is_empty() && !cfg.stt_model.trim().is_empty()).then_some((cfg, keys))
    };
    let Some(primary) = s.stt.cloud_order.iter().find(|p| with_keys(p).is_some()).cloned() else {
        return vec![];
    };
    let (cfg, keys) = with_keys(&primary).unwrap();
    let mut lanes = vec![Lane::new(&primary, cfg.clone(), keys.clone(), Role::Primary, false)];
    if primary == "groq" {
        // Same family, separate rate limit: doubles the Groq refresh rate.
        let alt = match cfg.stt_model.as_str() {
            "whisper-large-v3" => Some("whisper-large-v3-turbo"),
            "whisper-large-v3-turbo" => Some("whisper-large-v3"),
            _ => None,
        };
        if let Some(alt) = alt {
            let mut c = cfg.clone();
            c.stt_model = alt.into();
            let mut lane = Lane::new("groq", c, keys, Role::Extend, true);
            lane.next_ok = Instant::now() + lane.gap / 2; // interleave with the main lane
            lanes.push(lane);
        }
        if let Some((gcfg, gkeys)) = with_keys("gemini") {
            lanes.push(Lane::new("gemini", gcfg, gkeys, Role::Fill, true));
        }
    }
    lanes
}

struct Shared {
    committed_at: usize,
    segments: Vec<String>,
    tail: String,
    /// Words after `tail` from a Fill lane (shown as not stable).
    fill: String,
    applied_end: usize,
    fill_end: usize,
    in_flight: usize,
    /// Unconfirmed first guess of the current phrase.
    pending: Option<String>,
    publishes: Vec<f32>,
    first_text_at: Option<f32>,
    held: u32,
    dropped: u32,
}

fn publish(app: &AppHandle, sh: &mut Shared, t0: Instant) {
    let mut full = sh.segments.iter().filter(|t| !t.is_empty()).cloned().collect::<Vec<_>>().join(" ");
    for part in [&sh.tail, &sh.fill] {
        if part.is_empty() {
            continue;
        }
        if !full.is_empty() {
            full.push(' ');
        }
        full.push_str(part);
    }
    let n = full.chars().count();
    let stable = n - sh.fill.chars().count().min(n);
    let at = t0.elapsed().as_secs_f32();
    if n > 0 && sh.first_text_at.is_none() {
        sh.first_text_at = Some(at);
    }
    sh.publishes.push(at);
    log::info!("live preview: {n} chars ({stable} stable)");
    let _ = app.emit_to("overlay", "overlay-partial", json!({ "text": full, "stable": stable }));
}

fn norm_words(t: &str) -> Vec<String> {
    t.split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .collect()
}

/// Two guesses agree when they share a real word (3+ letters).
fn agrees(a: &str, b: &str) -> bool {
    let wa = norm_words(a);
    norm_words(b).iter().any(|w| w.chars().count() >= 3 && wa.contains(w))
}

/// Words of `newer` spoken after `shown`, aligned on `shown`'s last words. The last one or two
/// words of a window are the least reliable (cut mid-word), so the anchor may skip them: the
/// result says how many trailing words of `shown` to drop before appending.
/// `None` when they can't be aligned (then the newer text is not used as an extension).
pub(crate) fn extension(shown: &str, newer: &str) -> Option<(usize, String)> {
    let s = norm_words(shown);
    let orig: Vec<&str> = newer.split_whitespace().collect();
    let n = norm_words(newer);
    if s.is_empty() || n.is_empty() {
        return None;
    }
    for drop in 0..=2usize {
        if s.len() < drop + 2 {
            break;
        }
        let s = &s[..s.len() - drop];
        for k in [3usize, 2, 1] {
            if s.len() < k || (drop > 0 && k < 2) {
                continue;
            }
            let anchor = &s[s.len() - k..];
            let letters: usize = anchor.iter().map(|w| w.chars().count()).sum();
            if letters < if k == 1 { 5 } else { 6 } {
                continue;
            }
            if let Some(p) = (0..=n.len().saturating_sub(k)).rev().find(|&i| n.get(i..i + k) == Some(anchor)) {
                let rest = orig[p + k..].join(" ");
                // Must add more than it removes.
                return (n.len() - (p + k) > drop).then_some((drop, rest));
            }
        }
    }
    None
}

/// Applies an aligned extension: trims the unreliable trailing words of the tail, sets the fill.
fn apply_extension(sh: &mut Shared, drop: usize, ext: String, end: usize) {
    if drop > 0 {
        let words: Vec<&str> = sh.tail.split_whitespace().collect();
        sh.tail = words[..words.len().saturating_sub(drop)].join(" ");
    }
    sh.fill = ext;
    sh.fill_end = end;
}

pub fn cloud_preview(app: AppHandle, samples: Arc<Mutex<Vec<f32>>>, stop: Arc<AtomicBool>) {
    let st = app.state::<AppState>();
    let s = st.settings();
    let lanes = build_lanes(&app, &s);
    if lanes.is_empty() {
        return;
    }
    let base_prompt = crate::pipeline::stt_prompt(&s);
    let debug_text = std::env::var_os("FLOWVOICE_LOG_PREVIEW").is_some();
    if debug_text {
        let desc: Vec<String> = lanes
            .iter()
            .map(|l| format!("{} {:?} {} key(s) gap {} ms", l.name, l.role, l.keys.len(), l.gap.as_millis()))
            .collect();
        log::info!("cloud preview lanes: {}", desc.join("; "));
    }
    let lanes = Arc::new(Mutex::new(lanes));
    let shared = Arc::new(Mutex::new(Shared {
        committed_at: 0,
        segments: vec![],
        tail: String::new(),
        fill: String::new(),
        applied_end: 0,
        fill_end: 0,
        in_flight: 0,
        pending: None,
        publishes: vec![],
        first_text_at: None,
        held: 0,
        dropped: 0,
    }));
    let t0 = Instant::now();
    let mut last_send = Instant::now() - TARGET_GAP;
    let mut last_end = 0usize;

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
        let now = Instant::now();
        if now - last_send < TARGET_GAP || shared.lock().in_flight >= MAX_IN_FLIGHT {
            continue;
        }
        let (start, window, phrase_shown) = {
            let sh = shared.lock();
            let all = samples.lock();
            if all.len() <= sh.committed_at {
                continue;
            }
            (sh.committed_at, all[sh.committed_at..].to_vec(), !sh.tail.is_empty())
        };
        let end = start + window.len();
        // Nothing new worth a request (silence after the last one).
        if end < last_end + SR * 3 / 10 {
            continue;
        }
        let voiced = audio::trim_silence(&window).len() as f32 / SR as f32;
        if voiced < if phrase_shown { 1.0 } else { FIRST_MIN_VOICED } {
            continue;
        }

        // Pick a lane: the authoritative lane that's ready (earliest first); otherwise a fill lane,
        // but only if its answer can land before the next authoritative result.
        let pick = {
            let mut ls = lanes.lock();
            let primary = (0..ls.len())
                .filter(|&i| ls[i].role != Role::Fill && ls[i].ready(now))
                .min_by_key(|&i| ls[i].next_ok);
            let pick = primary.or_else(|| {
                let next_primary = ls
                    .iter()
                    .filter(|l| l.role != Role::Fill && !l.disabled)
                    .map(|l| l.next_ok.max(now) + Duration::from_millis(l.lat_ms as u64))
                    .min()?;
                (0..ls.len()).find(|&i| {
                    let l = &ls[i];
                    l.role == Role::Fill
                        && l.ready(now)
                        && phrase_shown
                        && now + Duration::from_millis(l.lat_ms as u64 + 150) < next_primary
                })
            });
            pick.and_then(|i| ls[i].take(now).map(|k| (i, k, ls[i].role, ls[i].provider.clone(), ls[i].cfg.clone(), ls[i].keys[k].clone(), ls[i].name.clone())))
        };
        let Some((lane_idx, key_idx, role, provider, cfg, key, lane_name)) = pick else { continue };
        last_send = now;
        last_end = end;
        shared.lock().in_flight += 1;

        let context: String = {
            let sh = shared.lock();
            let joined = sh.segments.join(" ");
            joined.chars().rev().take(240).collect::<Vec<_>>().into_iter().rev().collect()
        };
        let prompt = if context.is_empty() { base_prompt.clone() } else { format!("{base_prompt} {context}") };
        // Commit at a real pause at the end of a long phrase (taken from the audio itself).
        let level = audio::rms(&window).max(1e-4);
        let tail = &window[window.len().saturating_sub(SR * 7 / 10)..];
        let commit = role == Role::Primary && ((window.len() > SR * 12 && audio::rms(tail) < level * 0.2) || window.len() > SR * 20);

        let (app2, shared2, lanes2, stop2) = (app.clone(), shared.clone(), lanes.clone(), stop.clone());
        let lang = s.speech_language.clone();
        let secs = window.len() as f32 / SR as f32;
        tauri::async_runtime::spawn(async move {
            let wav = audio::to_wav(&audio::normalize(&window));
            let t1 = Instant::now();
            let http = app2.state::<AppState>().http.clone();
            let res = cloud::transcribe(&http, &provider, &cfg, &key, &wav, &lang, &prompt).await;
            let ms = t1.elapsed().as_millis() as f32;
            let at = t0.elapsed().as_secs_f32();
            let text = match res {
                Ok(t) => {
                    let mut ls = lanes2.lock();
                    let l = &mut ls[lane_idx];
                    l.ok += 1;
                    l.lat_ms = l.lat_ms * 0.6 + ms * 0.4;
                    crate::pipeline::tidy_preview(&text::strip_annotations(&t))
                }
                Err(e) => {
                    shared2.lock().in_flight -= 1;
                    lane_error(&app2, &lanes2, lane_idx, key_idx, &e, debug_text);
                    return;
                }
            };
            let mut sh = shared2.lock();
            sh.in_flight -= 1;
            if stop2.load(Ordering::Relaxed) || start != sh.committed_at {
                return;
            }
            let bad = text::is_hallucination_voiced(&text, voiced) || text.chars().count() as f32 > secs * 30.0 + 40.0;
            let log_line = |what: &str| {
                if debug_text {
                    log::info!("preview {what} [{lane_name}] t+{at:.1}s ({ms:.0} ms, {secs:.1}s audio, {voiced:.1}s voiced): {text}");
                }
            };
            // An Extend result that aligns with the shown text only appends; otherwise it acts
            // like a Primary result.
            if role == Role::Extend && !sh.tail.is_empty() && !bad && end > sh.applied_end && end > sh.fill_end {
                if let Some((drop, ext)) = extension(&sh.tail, &text) {
                    apply_extension(&mut sh, drop, ext, end);
                    log_line("extend");
                    lanes2.lock()[lane_idx].used += 1;
                    publish(&app2, &mut sh, t0);
                    return;
                }
            }
            match role {
                Role::Primary | Role::Extend => {
                    if end <= sh.applied_end {
                        log_line("stale");
                        return;
                    }
                    sh.applied_end = end;
                    if bad {
                        sh.dropped += 1;
                        log_line("dropped");
                        return;
                    }
                    if sh.tail.is_empty() && !commit {
                        // First text of the phrase: trust it only with enough audio or a second opinion.
                        let confirmed = voiced >= FIRST_TRUSTED_VOICED || sh.pending.as_deref().is_some_and(|p| agrees(p, &text));
                        if !confirmed {
                            sh.held += 1;
                            sh.pending = Some(text.clone());
                            log_line("held");
                            return;
                        }
                    }
                    sh.pending = None;
                    log_line("text");
                    if commit {
                        sh.segments.push(text);
                        sh.tail.clear();
                        sh.committed_at = end;
                    } else {
                        sh.tail = text;
                    }
                    // The appended words were aligned on the previous text: drop them.
                    sh.fill.clear();
                    sh.fill_end = end;
                    lanes2.lock()[lane_idx].used += 1;
                    publish(&app2, &mut sh, t0);
                }
                Role::Fill => {
                    if end <= sh.applied_end || end <= sh.fill_end || bad || sh.tail.is_empty() {
                        log_line("fill unused");
                        return;
                    }
                    match extension(&sh.tail, &text) {
                        Some((drop, ext)) => {
                            apply_extension(&mut sh, drop, ext, end);
                            log_line("fill");
                            lanes2.lock()[lane_idx].used += 1;
                            publish(&app2, &mut sh, t0);
                        }
                        None => log_line("fill unaligned"),
                    }
                }
            }
        });
    }

    if debug_text {
        // Let in-flight answers land for the stats.
        std::thread::sleep(Duration::from_millis(600));
        let sh = shared.lock();
        let gaps: Vec<f32> = sh.publishes.windows(2).map(|w| w[1] - w[0]).collect();
        let mean = if gaps.is_empty() { 0.0 } else { gaps.iter().sum::<f32>() / gaps.len() as f32 };
        let max = gaps.iter().cloned().fold(0.0f32, f32::max);
        log::info!(
            "preview summary: {} updates, mean gap {mean:.2}s, max gap {max:.2}s, first text t+{:.1}s, held {}, dropped {}",
            sh.publishes.len(),
            sh.first_text_at.unwrap_or(-1.0),
            sh.held,
            sh.dropped
        );
        for l in lanes.lock().iter() {
            log::info!(
                "preview lane {}: sent {} ok {} used {} rate-limited {} avg {:.0} ms",
                l.name, l.sent, l.ok, l.used, l.limited, l.lat_ms
            );
        }
    }
}

fn lane_error(app: &AppHandle, lanes: &Arc<Mutex<Vec<Lane>>>, lane_idx: usize, key_idx: usize, e: &cloud::ProviderError, debug: bool) {
    let mut ls = lanes.lock();
    let l = &mut ls[lane_idx];
    let now = Instant::now();
    if e.model_gone {
        if l.optional {
            l.disabled = true;
            log::info!("preview lane {} off: {}", l.name, e.detail);
        } else {
            // Heal in the background; the lane resumes with the new model.
            l.next_ok = now + Duration::from_secs(5);
            let (app, lanes, provider, model) = (app.clone(), lanes.clone(), l.provider.clone(), l.cfg.stt_model.clone());
            tauri::async_runtime::spawn(async move {
                if let Some(m) = crate::heal::heal(&app, &provider, crate::heal::Kind::Stt, &model).await {
                    let mut ls = lanes.lock();
                    let l = &mut ls[lane_idx];
                    l.cfg.stt_model = m;
                    l.name = format!("{provider}/{}", l.cfg.stt_model);
                    l.next_ok = Instant::now();
                }
            });
        }
        return;
    }
    if e.key_problem {
        let dead = e.detail.starts_with("HTTP 401") || e.detail.starts_with("HTTP 403");
        let wait = if dead { Duration::from_secs(3600) } else { Duration::from_secs_f32(e.retry_after.unwrap_or(10.0).clamp(1.0, 60.0)) };
        if dead || l.per_key {
            l.key_until[key_idx] = now + wait;
        } else {
            l.next_ok = now + wait; // account-wide limit: the whole lane waits
        }
        if !dead {
            l.limited += 1;
        }
        if debug {
            log::info!("preview lane {} key #{key_idx} waits {:.1}s: {}", l.name, wait.as_secs_f32(), e.detail);
        }
    } else {
        log::warn!("cloud preview {}: {e}", l.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_aligns_on_last_words() {
        let shown = "Mano, tipo assim, tô falando meio baixo aqui, né,";
        let gem = "Não, tipo assim, tô falando meio baixo aqui, né? Pra ver se o barulho";
        let ext = |a: &str, b: &str| extension(a, b).map(|(d, t)| format!("{d}|{t}"));
        assert_eq!(ext(shown, gem).as_deref(), Some("0|Pra ver se o barulho"));
        assert_eq!(ext("Pô, velho, amanhã", "Pô velho amanhã eu vou lá").as_deref(), Some("0|eu vou lá"));
        // Real case: v3 ended "Pô, velho." where turbo heard "Pô, bem, amanhã eu vou...".
        assert_eq!(
            ext("pra ver se o bagulho pega direitinho. Pô, velho.", "o bagulho pega direitinho. Pô, bem, amanhã eu vou lá").as_deref(),
            Some("1|bem, amanhã eu vou lá")
        );
        // No common anchor → unusable.
        assert_eq!(extension("Mano, tipo assim", "Bom dia a todos"), None);
        // Nothing new.
        assert_eq!(extension("tô falando meio baixo", "tô falando meio baixo."), None);
    }

    #[test]
    fn first_guess_agreement() {
        assert!(!agrees("No ano de 2017,", "Mano, tipo assim."));
        assert!(agrees("Mano, tipo assim.", "Mano, tipo assim, tô falando"));
        assert!(!agrees("Mãe, te gosto.", "Mano, tipo assim."));
    }
}
