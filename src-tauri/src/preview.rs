//! Cloud live preview: re-transcribes the current phrase about every second while the user
//! speaks, spreading requests over "lanes" so no provider's rate limit is exceeded.
//!
//! Lanes (measured 2026-10-30/31, giria_quiet.wav):
//! - Groq whisper: ~0.35-0.7 s per request, near-final accuracy. Free tier per MODEL for the
//!   whole organization (extra keys from the same account don't help): 20 requests/min,
//!   2,000/day and 7,200 audio-seconds/hour (each request under 10 s is billed as 10 s).
//!   whisper-large-v3 (primary) and whisper-large-v3-turbo (extension) have separate limits.
//! - Gemini flash-lite as an extra "fill" lane: off by default (`GEMINI_FILL`), its wording
//!   differs from whisper's and it's ~3x slower, which made the text jump.
//!
//! 2.1 rules (each measured with the fake-mic E2E, see tools/dev/pvstats.py):
//! - Every request has an EMPTY prompt (earlier text as context was echoed back).
//! - Exactly one request in flight per lane; every answer carries its window end and an answer
//!   for audio already covered by a newer one is dropped, so the text never goes back in time.
//! - Spacing per lane comes from the model's remaining budget (Groq's `x-ratelimit-*` headers
//!   for requests/day and audio-seconds/hour, a local estimate for requests/min), never closer
//!   than `TARGET_GAP` overall; the last request and the final pass's audio are always left for
//!   the final transcription of the primary model.
//! - A phrase is committed at a real pause once it passes 12 s, and at 15 s at the latest
//!   (shorter windows: fewer billed seconds, faster answers).
//! - LocalAgreement-2: a word becomes stable (solid in the overlay) only when two consecutive
//!   answers agree on it; the rest is shown as not yet stable and may still change.
//!
//! First guess: whisper invents text on very short windows ("Boa noite.", "No ano de 2017,").
//! A dictation's first text needs >= 2 s of voiced audio, and below 3 s it is only shown once a
//! second result agrees with it (shares a word). Later phrases need 1 s / 2 s.

use parking_lot::Mutex;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::audio;
use crate::cloud::{self, RateInfo};
use crate::settings::{ProviderSettings, Settings};
use crate::state::AppState;
use crate::text;

const SR: usize = audio::SAMPLE_RATE as usize;
/// Voiced audio before the dictation's first request.
const FIRST_MIN_VOICED: f32 = 2.0;
/// Below this, the dictation's first text must be confirmed by the next result.
const FIRST_TRUSTED_VOICED: f32 = 3.0;
/// Same for the following phrases (text is already on screen; a pause just committed).
const NEXT_MIN_VOICED: f32 = 1.0;
const NEXT_TRUSTED_VOICED: f32 = 2.0;
/// Minimum spacing between any two requests (all lanes).
const TARGET_GAP: Duration = Duration::from_millis(950);
/// Gemini as an extra "fill" lane next to Groq. Off: its different wording ("barulho" ×
/// "bagulho") and ~1.5 s latency made the text jump more than it helped (2.1 measurements).
const GEMINI_FILL: bool = false;
/// Phrase commit: at a real pause (`PAUSE_SECS` well below the phrase's level) once the window
/// is longer than this…
const COMMIT_AFTER: f32 = 12.0;
/// …and regardless of pauses past this.
const FORCE_COMMIT: f32 = 15.0;
const PAUSE_SECS: f32 = 0.7;

// ---------------------------------------------------------------- rate budget (per model)

/// Groq whisper, free tier: 20 requests/min (not reported in the headers, estimated here).
const RPM: f32 = 20.0;
const RPM_REFILL_SECS: f32 = 3.0;
/// Requests shorter than this are billed as this many audio seconds.
const MIN_BILLED_SECS: f32 = 10.0;
/// Fastest spacing per lane when the budget is plentiful (2.0 used a fixed 3.1 s).
const LANE_MIN_GAP: f32 = 1.9;
/// Budget above what refills is spread over this many seconds of use.
const SPREAD_SECS: f32 = 900.0;

fn billed(secs: f32) -> f32 {
    secs.max(MIN_BILLED_SECS)
}

/// A token bucket as Groq reports it: what's left, the capacity and the refill rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bucket {
    remaining: f32,
    cap: f32,
    /// Units per second.
    rate: f32,
}

impl Bucket {
    fn refill(&mut self, dt: f32) {
        self.remaining = (self.remaining + self.rate * dt).min(self.cap);
    }

    /// `Ok(spacing)` that spreads the spare budget (above `reserve`) over `SPREAD_SECS`, or
    /// `Err(wait)` until `cost` fits without touching `reserve`.
    fn spacing(&self, cost: f32, reserve: f32) -> Result<f32, f32> {
        let avail = self.remaining - reserve;
        if avail < cost {
            return Err(((cost - avail) / self.rate.max(1e-6)).min(3600.0));
        }
        Ok(cost / (self.rate + (avail - cost) / SPREAD_SECS))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Gate {
    /// Minimum spacing since the lane's previous request.
    Ready(Duration),
    /// No budget: wait this long.
    Pause(Duration),
}

/// What's left of one model's rate limits (shared by the preview and the final pass).
#[derive(Clone, Debug)]
pub(crate) struct Budget {
    /// Requests per minute: a local estimate (Groq doesn't report it). Starts half full.
    rpm: Bucket,
    /// Requests per day (`x-ratelimit-*-requests`).
    requests: Option<Bucket>,
    /// Audio seconds per hour (`x-ratelimit-*-audio-seconds`).
    audio: Option<Bucket>,
    at: Instant,
    paused_until: Option<Instant>,
}

impl Budget {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            rpm: Bucket { remaining: RPM / 2.0, cap: RPM, rate: 1.0 / RPM_REFILL_SECS },
            requests: None,
            audio: None,
            at: now,
            paused_until: None,
        }
    }

    fn advance(&mut self, now: Instant) {
        let dt = now.saturating_duration_since(self.at).as_secs_f32();
        self.at = self.at.max(now);
        self.rpm.refill(dt);
        for b in [&mut self.requests, &mut self.audio].into_iter().flatten() {
            b.refill(dt);
        }
    }

    /// A request was sent with `secs` of audio.
    pub(crate) fn book(&mut self, now: Instant, secs: f32) {
        self.advance(now);
        self.rpm.remaining -= 1.0;
        if let Some(b) = &mut self.requests {
            b.remaining -= 1.0;
        }
        if let Some(b) = &mut self.audio {
            b.remaining -= billed(secs);
        }
    }

    /// The provider's own numbers replace the estimates.
    pub(crate) fn observe(&mut self, now: Instant, r: &RateInfo) {
        self.advance(now);
        let bucket = |rem: f32, cap: f32, reset: Option<f32>, prev: Option<Bucket>, default_rate: f32| {
            let rate = match reset {
                Some(t) if t > 0.0 && cap > rem => (cap - rem) / t,
                _ => prev.map(|b| b.rate).unwrap_or(default_rate),
            };
            Bucket { remaining: rem, cap, rate }
        };
        if let (Some(rem), Some(cap)) = (r.remaining_requests, r.limit_requests) {
            self.requests = Some(bucket(rem as f32, cap as f32, r.reset_requests, self.requests, cap as f32 / 86_400.0));
        }
        if let (Some(rem), Some(cap)) = (r.remaining_audio, r.limit_audio) {
            self.audio = Some(bucket(rem, cap, r.reset_audio, self.audio, cap / 3_600.0));
        }
    }

    /// 429: wait what the provider asked; the minute budget is spent.
    pub(crate) fn limited(&mut self, now: Instant, wait: Duration) {
        self.advance(now);
        self.paused_until = Some(now + wait);
        self.rpm.remaining = self.rpm.remaining.min(0.0);
    }

    /// May a request with `secs` of audio go now? `final_secs` = keep one request and that much
    /// audio for the final transcription (primary model only).
    pub(crate) fn gate(&self, now: Instant, secs: f32, final_secs: Option<f32>) -> Gate {
        if let Some(t) = self.paused_until {
            if now < t {
                return Gate::Pause(t - now);
            }
        }
        let mut b = self.clone();
        b.advance(now);
        let (keep_req, keep_audio) = match final_secs {
            Some(f) => (1.0, billed(f)),
            None => (0.0, 0.0),
        };
        // Requests/min: from ~3 s per request (refill rate) when nearly empty down to
        // LANE_MIN_GAP when there's room.
        let avail = b.rpm.remaining - keep_req;
        if avail < 1.0 {
            return Gate::Pause(Duration::from_secs_f32((1.0 - avail) * RPM_REFILL_SECS));
        }
        let room = ((avail - 1.0) / (RPM / 2.0)).clamp(0.0, 1.0);
        let mut gap = RPM_REFILL_SECS - (RPM_REFILL_SECS - LANE_MIN_GAP) * room;
        for (bucket, cost, keep) in [(b.requests, 1.0, keep_req), (b.audio, billed(secs), keep_audio)] {
            if let Some(bucket) = bucket {
                match bucket.spacing(cost, keep) {
                    Ok(g) => gap = gap.max(g),
                    Err(w) => return Gate::Pause(Duration::from_secs_f32(w)),
                }
            }
        }
        Gate::Ready(Duration::from_secs_f32(gap.max(LANE_MIN_GAP)))
    }
}

/// Per "provider/model", for the life of the app (consecutive dictations share the budget).
static BUDGETS: LazyLock<Mutex<HashMap<String, Budget>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn with_budget<R>(key: &str, f: impl FnOnce(&mut Budget) -> R) -> R {
    let mut all = BUDGETS.lock();
    f(all.entry(key.to_string()).or_insert_with(|| Budget::new(Instant::now())))
}

/// The final pass (or anything outside the preview) used a model: keeps the preview's view of
/// that model's budget right. `rate`: what the provider reported (success); `limited`: it
/// answered 429 and asked to wait this many seconds.
pub fn note_request(provider: &str, model: &str, secs: f32, rate: Option<&RateInfo>, limited: Option<f32>) {
    if provider != "groq" {
        return;
    }
    let now = Instant::now();
    with_budget(&format!("{provider}/{model}"), |b| {
        b.book(now, secs);
        if let Some(r) = rate {
            b.observe(now, r);
        }
        if let Some(w) = limited {
            b.limited(now, Duration::from_secs_f32(w.clamp(1.0, 60.0)));
        }
    });
}

// ---------------------------------------------------------------- lanes

#[derive(Clone, Copy, PartialEq, Debug)]
enum Role {
    /// Full re-transcription; replaces the phrase's unstable words.
    Primary,
    /// Same cadence as Primary, but once the phrase has text it only appends the words after it
    /// (whisper-large-v3-turbo next to v3: keeps v3's wording instead of flipping every tick).
    /// Replaces the unstable words only when it can't be aligned.
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
    /// Fixed spacing for providers without a budget model (not Groq).
    gap: Duration,
    /// Limits are per key (separate projects) rather than per account.
    per_key: bool,
    next_ok: Instant,
    last_sent: Option<Instant>,
    key_next: usize,
    key_until: Vec<Instant>,
    /// Alternate model added only for capacity: dropped if it ever fails as gone.
    optional: bool,
    disabled: bool,
    /// Exactly one request per lane at a time: answers land in order.
    in_flight: bool,
    lat_ms: f32,
    sent: u32,
    ok: u32,
    limited: u32,
    used: u32,
    paused: u32,
    was_paused: bool,
}

impl Lane {
    fn new(provider: &str, cfg: ProviderSettings, keys: Vec<String>, role: Role, optional: bool) -> Self {
        let n = keys.len().max(1) as u32;
        let (gap, per_key, lat) = match provider {
            "groq" => (Duration::from_secs_f32(LANE_MIN_GAP), false, 450.0),
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
            last_sent: None,
            key_next: 0,
            optional,
            disabled: false,
            in_flight: false,
            lat_ms: lat,
            sent: 0,
            ok: 0,
            limited: 0,
            used: 0,
            paused: 0,
            was_paused: false,
        }
    }

    fn budgeted(&self) -> bool {
        self.provider == "groq"
    }

    /// When this lane may send a window of `secs` (None: busy or off).
    fn due(&mut self, now: Instant, secs: f32, final_secs: f32) -> Option<Instant> {
        if self.disabled || self.in_flight {
            return None;
        }
        let key_free = self.key_until.iter().min().copied().unwrap_or(now);
        let mut t = self.next_ok.max(key_free);
        if self.budgeted() {
            let keep = (self.role == Role::Primary).then_some(final_secs);
            match with_budget(&self.name, |b| b.gate(now, secs, keep)) {
                Gate::Ready(g) => {
                    self.was_paused = false;
                    if let Some(l) = self.last_sent {
                        t = t.max(l + g);
                    }
                }
                Gate::Pause(d) => {
                    if !self.was_paused {
                        self.was_paused = true;
                        self.paused += 1;
                        log::info!("preview lane {} paused {:.1}s (rate budget)", self.name, d.as_secs_f32());
                    }
                    t = t.max(now + d);
                }
            }
        } else if let Some(l) = self.last_sent {
            t = t.max(l + self.gap);
        }
        Some(t)
    }

    /// Picks a key and books the request.
    fn take(&mut self, now: Instant, secs: f32) -> Option<usize> {
        let n = self.keys.len();
        let i = (0..n).map(|k| (self.key_next + k) % n).find(|&i| self.key_until[i] <= now)?;
        self.key_next = i + 1;
        self.last_sent = Some(now);
        if self.per_key {
            // A key can't be reused before its own spacing (gap * keys).
            self.key_until[i] = now + self.gap * n as u32;
        }
        if self.budgeted() {
            with_budget(&self.name, |b| b.book(now, secs));
        }
        self.sent += 1;
        self.in_flight = true;
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
    let Some((cfg, keys)) = with_keys(&primary) else {
        return vec![];
    };
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
            // Interleave with the main lane.
            lane.next_ok = Instant::now() + Duration::from_secs_f32(LANE_MIN_GAP / 2.0);
            lanes.push(lane);
        }
        if GEMINI_FILL {
            if let Some((gcfg, gkeys)) = with_keys("gemini") {
                lanes.push(Lane::new("gemini", gcfg, gkeys, Role::Fill, true));
            }
        }
    }
    lanes
}

// ---------------------------------------------------------------- text of the phrase

fn norm_word(w: &str) -> String {
    w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

fn norm_words(t: &str) -> Vec<String> {
    t.split_whitespace().map(norm_word).collect()
}

/// Number of leading words two word lists share (ignoring case and punctuation).
fn common_prefix<A: AsRef<str>, B: AsRef<str>>(a: &[A], b: &[B]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| norm_word(x.as_ref()) == norm_word(y.as_ref())).count()
}

/// Two guesses agree when they share a real word (3+ letters).
fn agrees(a: &str, b: &str) -> bool {
    let wa = norm_words(a);
    norm_words(b).iter().any(|w| w.chars().count() >= 3 && wa.contains(w))
}

/// Words of `newer` spoken after `shown`, aligned on `shown`'s last words. The last one or two
/// words of a window are the least reliable (cut mid-word), so the anchor may skip up to
/// `max_drop` of them: the result says how many trailing words of `shown` to drop before
/// appending. `None` when they can't be aligned (then the newer text is not used as an extension).
pub(crate) fn extension(shown: &str, newer: &str, max_drop: usize) -> Option<(usize, String)> {
    let s = norm_words(shown);
    let orig: Vec<&str> = newer.split_whitespace().collect();
    let n = norm_words(newer);
    if s.is_empty() || n.is_empty() {
        return None;
    }
    for drop in 0..=max_drop.min(2) {
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

/// The phrase being spoken: its words, the first `locked` of them stable (LocalAgreement-2).
#[derive(Clone, Debug, Default)]
pub(crate) struct Phrase {
    words: Vec<String>,
    locked: usize,
}

impl Phrase {
    fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    fn text(&self) -> String {
        self.words.join(" ")
    }

    /// LocalAgreement-2: the words a new answer shares with what's shown (the previous answer)
    /// become stable. Returns the agreed length.
    fn agree(&mut self, new: &[String]) -> usize {
        let k = common_prefix(&self.words, new);
        self.locked = self.locked.max(k);
        k
    }

    /// Authoritative answer: keeps the stable words, replaces the rest. When it contradicts a
    /// stable word, only what it says after the stable words (aligned) is used.
    fn apply_full(&mut self, text: &str) -> bool {
        let new: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let before = self.words.clone();
        let k = self.agree(&new);
        if k >= self.locked {
            self.words.truncate(k);
            self.words.extend(new[k..].iter().cloned());
        } else if let Some((_, rest)) = extension(&self.words[..self.locked].join(" "), text, 0) {
            self.words.truncate(self.locked);
            self.words.extend(rest.split_whitespace().map(str::to_string));
        }
        self.words != before
    }

    /// Extension answer (other model): agreement, then only appends after the shown words; may
    /// replace up to two unstable trailing words. Unaligned: replaces the unstable words, but
    /// never shortens the text.
    fn apply_extend(&mut self, text: &str) -> bool {
        let new: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let before = self.words.clone();
        let k = self.agree(&new);
        let max_drop = self.words.len() - self.locked;
        if let Some((drop, rest)) = extension(&self.text(), text, max_drop) {
            self.words.truncate(self.words.len() - drop);
            self.words.extend(rest.split_whitespace().map(str::to_string));
        } else if k >= self.locked && new.len() >= self.words.len() {
            self.words.truncate(k);
            self.words.extend(new[k..].iter().cloned());
        }
        self.words != before
    }

    /// Ends the phrase: everything shown becomes a committed segment.
    fn commit(&mut self) -> String {
        let t = self.text();
        *self = Self::default();
        t
    }
}

struct Shared {
    committed_at: usize,
    segments: Vec<String>,
    phrase: Phrase,
    /// Words after the phrase from a Fill lane (shown as not stable).
    fill: String,
    applied_end: usize,
    fill_end: usize,
    /// A commit answer is pending: no other request until it lands (it must not go stale).
    commit_in_flight: bool,
    /// Unconfirmed first guess of the current phrase.
    pending: Option<String>,
    publishes: Vec<f32>,
    first_text_at: Option<f32>,
    held: u32,
    dropped: u32,
}

impl Shared {
    fn new() -> Self {
        Self {
            committed_at: 0,
            segments: vec![],
            phrase: Phrase::default(),
            fill: String::new(),
            applied_end: 0,
            fill_end: 0,
            commit_in_flight: false,
            pending: None,
            publishes: vec![],
            first_text_at: None,
            held: 0,
            dropped: 0,
        }
    }

    /// The line shown and how many of its leading chars are stable.
    fn line(&self) -> (String, usize) {
        let mut full = String::new();
        let push = |full: &mut String, part: &str| {
            if !part.is_empty() {
                if !full.is_empty() {
                    full.push(' ');
                }
                full.push_str(part);
            }
        };
        for s in &self.segments {
            push(&mut full, s);
        }
        push(&mut full, &self.phrase.words[..self.phrase.locked].join(" "));
        let stable = full.chars().count();
        push(&mut full, &self.phrase.words[self.phrase.locked..].join(" "));
        push(&mut full, &self.fill);
        (full, stable)
    }
}

/// One answer for the window [start, end).
struct Answer {
    role: Role,
    start: usize,
    end: usize,
    text: String,
    voiced: f32,
    secs: f32,
    commit: bool,
    prompt: String,
}

#[derive(Debug, PartialEq)]
enum Outcome {
    /// From a phrase already committed.
    Old,
    /// Covers less audio than what's shown.
    Stale,
    Dropped,
    Held,
    /// Shown (the label is for the log).
    Shown(&'static str),
    Unused(&'static str),
}

/// Applies an answer to the preview state (pure: no I/O).
fn accept(sh: &mut Shared, a: &Answer) -> Outcome {
    if a.commit {
        sh.commit_in_flight = false;
    }
    if a.start != sh.committed_at {
        return Outcome::Old;
    }
    // The prompt (if any) echoed back, a classic hallucination, or a loop.
    let bad = text::echoes_prompt(&a.text, &a.prompt)
        || text::is_hallucination_voiced(&a.text, a.voiced)
        || a.text.chars().count() as f32 > a.secs * 30.0 + 40.0;
    if a.role == Role::Fill {
        if a.end <= sh.applied_end || a.end <= sh.fill_end || bad || sh.phrase.is_empty() {
            return Outcome::Unused("fill unused");
        }
        let max_drop = sh.phrase.words.len() - sh.phrase.locked;
        return match extension(&sh.phrase.text(), &a.text, max_drop) {
            Some((drop, ext)) => {
                let n = sh.phrase.words.len();
                sh.phrase.words.truncate(n - drop);
                sh.fill = ext;
                sh.fill_end = a.end;
                Outcome::Shown("fill")
            }
            None => Outcome::Unused("fill unaligned"),
        };
    }
    if a.end <= sh.applied_end {
        return Outcome::Stale;
    }
    sh.applied_end = a.end;
    if bad {
        sh.dropped += 1;
        return Outcome::Dropped;
    }
    if sh.phrase.is_empty() && !a.commit {
        // First text of the phrase: trust it only with enough audio or a second opinion.
        let trusted = if sh.segments.iter().any(|s| !s.is_empty()) { NEXT_TRUSTED_VOICED } else { FIRST_TRUSTED_VOICED };
        let confirmed = a.voiced >= trusted || sh.pending.as_deref().is_some_and(|p| agrees(p, &a.text));
        if !confirmed {
            sh.held += 1;
            sh.pending = Some(a.text.clone());
            return Outcome::Held;
        }
    }
    sh.pending = None;
    let label = if a.role == Role::Extend && !sh.phrase.is_empty() {
        sh.phrase.apply_extend(&a.text);
        "extend"
    } else {
        sh.phrase.apply_full(&a.text);
        "text"
    };
    if a.commit {
        let seg = sh.phrase.commit();
        sh.segments.push(seg);
        sh.committed_at = a.end;
    }
    // Fill words were aligned on the previous text: drop them.
    sh.fill.clear();
    sh.fill_end = a.end;
    Outcome::Shown(label)
}

fn publish(app: &AppHandle, sh: &mut Shared, t0: Instant, debug: bool, stop: &Arc<AtomicBool>) {
    let (full, stable) = sh.line();
    let n = full.chars().count();
    let at = t0.elapsed().as_secs_f32();
    if n > 0 && sh.first_text_at.is_none() {
        sh.first_text_at = Some(at);
    }
    sh.publishes.push(at);
    log::debug!("live preview: {n} chars ({stable} stable)");
    if debug {
        log::info!("preview shown t+{at:.2}s stable {stable}/{n}: {full}");
    }
    crate::pipeline::note_live_text(app, stop, &full);
    let _ = app.emit_to("overlay", "overlay-partial", json!({ "text": full, "stable": stable }));
}

/// Commit decision for a window: a real pause at its end once it's long, or it's too long.
fn should_commit(window: &[f32]) -> bool {
    let secs = window.len() as f32 / SR as f32;
    if secs > FORCE_COMMIT {
        return true;
    }
    if secs <= COMMIT_AFTER {
        return false;
    }
    let level = audio::rms(window).max(1e-4);
    let tail = &window[window.len().saturating_sub((SR as f32 * PAUSE_SECS) as usize)..];
    audio::rms(tail) < level * 0.2
}

pub fn cloud_preview(app: AppHandle, samples: Arc<Mutex<Vec<f32>>>, stop: Arc<AtomicBool>) {
    let st = app.state::<AppState>();
    let s = st.settings();
    let lanes = build_lanes(&app, &s);
    if lanes.is_empty() {
        return;
    }
    // Always the plain prompt (empty): earlier text as context made whisper echo it back.
    let prompt = crate::pipeline::stt_prompt(&s);
    let debug = std::env::var_os("FLOWVOICE_LOG_PREVIEW").is_some();
    if debug {
        let desc: Vec<String> = lanes.iter().map(|l| format!("{} {:?} {} key(s)", l.name, l.role, l.keys.len())).collect();
        log::info!("cloud preview lanes: {}", desc.join("; "));
    }
    let lanes = Arc::new(Mutex::new(lanes));
    let shared = Arc::new(Mutex::new(Shared::new()));
    let t0 = Instant::now();
    let mut last_send = Instant::now() - TARGET_GAP;
    let mut last_end = 0usize;

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
        let now = Instant::now();
        if now - last_send < TARGET_GAP {
            continue;
        }
        let (start, window, any_shown, total) = {
            let sh = shared.lock();
            if sh.commit_in_flight {
                continue;
            }
            let all = samples.lock();
            if all.len() <= sh.committed_at {
                continue;
            }
            let any = !sh.phrase.is_empty() || sh.segments.iter().any(|s| !s.is_empty());
            (sh.committed_at, all[sh.committed_at..].to_vec(), any, all.len())
        };
        let end = start + window.len();
        // Nothing new worth a request (silence after the last one).
        if end < last_end + SR * 3 / 10 {
            continue;
        }
        let voiced = audio::trim_silence(&window).len() as f32 / SR as f32;
        if voiced < if any_shown { NEXT_MIN_VOICED } else { FIRST_MIN_VOICED } {
            continue;
        }
        let secs = window.len() as f32 / SR as f32;
        let total_secs = total as f32 / SR as f32;
        let wants_commit = should_commit(&window);

        // Pick the lane that's been due the longest; a fill lane only if its answer can land
        // before the next authoritative one.
        let pick = {
            let mut ls = lanes.lock();
            let due: Vec<Option<Instant>> = ls.iter_mut().map(|l| l.due(now, secs, total_secs)).collect();
            let main = (0..ls.len())
                .filter(|&i| ls[i].role != Role::Fill && due[i].is_some_and(|t| t <= now))
                .min_by_key(|&i| due[i]);
            let pick = main.or_else(|| {
                let next_main = (0..ls.len())
                    .filter(|&i| ls[i].role != Role::Fill)
                    .filter_map(|i| due[i].map(|t| t.max(now) + Duration::from_millis(ls[i].lat_ms as u64)))
                    .min()?;
                (0..ls.len()).find(|&i| {
                    ls[i].role == Role::Fill
                        && due[i].is_some_and(|t| t <= now)
                        && any_shown
                        && now + Duration::from_millis(ls[i].lat_ms as u64 + 150) < next_main
                })
            });
            pick.and_then(|i| {
                ls[i].take(now, secs).map(|k| (i, k, ls[i].role, ls[i].provider.clone(), ls[i].cfg.clone(), ls[i].keys[k].clone(), ls[i].name.clone()))
            })
        };
        let Some((lane_idx, key_idx, role, provider, cfg, key, lane_name)) = pick else { continue };
        last_send = now;
        last_end = end;
        // The primary model commits; the extension one only if the primary can't get to it.
        let commit = wants_commit && (role == Role::Primary || (role == Role::Extend && secs > FORCE_COMMIT + 3.0));
        if commit {
            shared.lock().commit_in_flight = true;
        }

        let (app2, shared2, lanes2, stop2, prompt) = (app.clone(), shared.clone(), lanes.clone(), stop.clone(), prompt.clone());
        let lang = s.speech_language.clone();
        tauri::async_runtime::spawn(async move {
            let wav = audio::to_wav(&audio::normalize(&window));
            let t1 = Instant::now();
            let http = app2.state::<AppState>().http.clone();
            let res = cloud::transcribe_rated(&http, &provider, &cfg, &key, &wav, &lang, &prompt).await;
            let ms = t1.elapsed().as_millis() as f32;
            let at = t0.elapsed().as_secs_f32();
            let text = match res {
                Ok((t, rate)) => {
                    let mut ls = lanes2.lock();
                    let l = &mut ls[lane_idx];
                    l.in_flight = false;
                    l.ok += 1;
                    l.lat_ms = l.lat_ms * 0.6 + ms * 0.4;
                    if l.budgeted() {
                        with_budget(&l.name, |b| b.observe(Instant::now(), &rate));
                    }
                    crate::pipeline::tidy_preview(&text::strip_annotations(&t))
                }
                Err(e) => {
                    if commit {
                        shared2.lock().commit_in_flight = false;
                    }
                    lane_error(&app2, &lanes2, lane_idx, key_idx, &e, debug);
                    return;
                }
            };
            let mut sh = shared2.lock();
            if stop2.load(Ordering::Relaxed) {
                if commit {
                    sh.commit_in_flight = false;
                }
                return;
            }
            let answer = Answer { role, start, end, text, voiced, secs, commit, prompt };
            let outcome = accept(&mut sh, &answer);
            if debug {
                let what = match &outcome {
                    Outcome::Old => "old",
                    Outcome::Stale => "stale",
                    Outcome::Dropped => "dropped",
                    Outcome::Held => "held",
                    Outcome::Shown(l) | Outcome::Unused(l) => l,
                };
                let c = if commit { " commit" } else { "" };
                log::info!("preview {what} [{lane_name}] t+{at:.1}s ({ms:.0} ms, {secs:.1}s audio, {voiced:.1}s voiced{c}): {}", answer.text);
            }
            if let Outcome::Shown(_) = outcome {
                lanes2.lock()[lane_idx].used += 1;
                publish(&app2, &mut sh, t0, debug, &stop2);
            }
        });
    }

    if debug {
        // Let in-flight answers land for the stats.
        std::thread::sleep(Duration::from_millis(600));
        let sh = shared.lock();
        let gaps: Vec<f32> = sh.publishes.windows(2).map(|w| w[1] - w[0]).collect();
        let mean = if gaps.is_empty() { 0.0 } else { gaps.iter().sum::<f32>() / gaps.len() as f32 };
        let max = gaps.iter().cloned().fold(0.0f32, f32::max);
        log::info!(
            "preview summary: {} updates, mean gap {mean:.2}s, max gap {max:.2}s, first text t+{:.1}s, held {}, dropped {}, segments {}",
            sh.publishes.len(),
            sh.first_text_at.unwrap_or(-1.0),
            sh.held,
            sh.dropped,
            sh.segments.len()
        );
        for l in lanes.lock().iter() {
            log::info!(
                "preview lane {}: sent {} ok {} used {} rate-limited {} paused {} avg {:.0} ms",
                l.name, l.sent, l.ok, l.used, l.limited, l.paused, l.lat_ms
            );
            if l.budgeted() {
                let b = with_budget(&l.name, |b| {
                    b.advance(Instant::now());
                    b.clone()
                });
                log::info!(
                    "preview budget {}: rpm~{:.1} requests/day {:?} audio-s {:?}",
                    l.name,
                    b.rpm.remaining,
                    b.requests.map(|x| x.remaining as i64),
                    b.audio.map(|x| x.remaining as i64)
                );
            }
        }
    }
}

fn lane_error(app: &AppHandle, lanes: &Arc<Mutex<Vec<Lane>>>, lane_idx: usize, key_idx: usize, e: &cloud::ProviderError, debug: bool) {
    let mut ls = lanes.lock();
    let l = &mut ls[lane_idx];
    l.in_flight = false;
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
        } else if l.budgeted() {
            with_budget(&l.name, |b| b.limited(now, wait)); // account-wide: the model waits
        } else {
            l.next_ok = now + wait;
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

    fn w(t: &str) -> Vec<String> {
        t.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn extension_aligns_on_last_words() {
        let shown = "Mano, tipo assim, tô falando meio baixo aqui, né,";
        let gem = "Não, tipo assim, tô falando meio baixo aqui, né? Pra ver se o barulho";
        let ext = |a: &str, b: &str| extension(a, b, 2).map(|(d, t)| format!("{d}|{t}"));
        assert_eq!(ext(shown, gem).as_deref(), Some("0|Pra ver se o barulho"));
        assert_eq!(ext("Pô, velho, amanhã", "Pô velho amanhã eu vou lá").as_deref(), Some("0|eu vou lá"));
        // Real case: v3 ended "Pô, velho." where turbo heard "Pô, bem, amanhã eu vou...".
        assert_eq!(
            ext("pra ver se o bagulho pega direitinho. Pô, velho.", "o bagulho pega direitinho. Pô, bem, amanhã eu vou lá").as_deref(),
            Some("1|bem, amanhã eu vou lá")
        );
        // …but not when "velho" is already stable.
        assert_eq!(extension("pra ver se o bagulho pega direitinho. Pô, velho.", "o bagulho pega direitinho. Pô, bem, amanhã eu vou lá", 0), None);
        // No common anchor → unusable.
        assert_eq!(extension("Mano, tipo assim", "Bom dia a todos", 2), None);
        // Nothing new.
        assert_eq!(extension("tô falando meio baixo", "tô falando meio baixo.", 2), None);
    }

    #[test]
    fn first_guess_agreement() {
        assert!(!agrees("No ano de 2017,", "Mano, tipo assim."));
        assert!(agrees("Mano, tipo assim.", "Mano, tipo assim, tô falando"));
        assert!(!agrees("Mãe, te gosto.", "Mano, tipo assim."));
    }

    #[test]
    fn agreement_prefix() {
        assert_eq!(common_prefix(&w("Mano, tipo assim, tô"), &w("mano tipo assim tô falando")), 4);
        assert_eq!(common_prefix(&w("Mano, tipo assim"), &w("Não, tipo assim")), 0);
        assert_eq!(common_prefix::<String, String>(&[], &w("a b")), 0);

        let mut p = Phrase::default();
        p.apply_full("Mano, tipo assim, tô");
        assert_eq!((p.text().as_str(), p.locked), ("Mano, tipo assim, tô", 0)); // one answer: nothing stable
        p.apply_full("Mano, tipo assim, tô falando meio");
        assert_eq!(p.locked, 4); // two answers agree on 4 words
        // The next answer changes an unstable word: allowed; the stable ones keep their spelling.
        p.apply_full("mano tipo assim tô falei meio baixo");
        assert_eq!((p.text().as_str(), p.locked), ("Mano, tipo assim, tô falei meio baixo", 4));
        // An answer contradicting a stable word: the stable words stay, the rest is aligned.
        p.apply_full("Não, tipo assim, tô falando meio baixo aqui");
        assert_eq!(p.text(), "Mano, tipo assim, tô falando meio baixo aqui");
        assert_eq!(p.locked, 4);
        // Unalignable contradiction: nothing changes.
        assert!(!p.apply_full("Bom dia a todos"));
        assert_eq!(p.commit(), "Mano, tipo assim, tô falando meio baixo aqui");
        assert!(p.is_empty() && p.locked == 0);
    }

    #[test]
    fn extend_only_appends() {
        let mut p = Phrase::default();
        p.apply_full("pra ver se o bagulho pega direitinho. Pô, velho.");
        // Turbo agrees on 7 words (stable), replaces the unreliable last word and appends.
        assert!(p.apply_extend("pra ver se o bagulho pega direitinho. Pô, bem, amanhã eu vou lá"));
        assert_eq!(p.text(), "pra ver se o bagulho pega direitinho. Pô, bem, amanhã eu vou lá");
        assert_eq!(p.locked, 8);
        // A shorter unaligned answer never shortens the text.
        assert!(!p.apply_extend("pra ver"));
        assert_eq!(p.words.len(), 13);
    }

    fn answer(role: Role, end: usize, text: &str, voiced: f32) -> Answer {
        Answer { role, start: 0, end, text: text.into(), voiced, secs: end as f32 / SR as f32, commit: false, prompt: String::new() }
    }

    #[test]
    fn stale_and_first_guess() {
        let mut sh = Shared::new();
        // Short first guess: held until a second one agrees.
        assert_eq!(accept(&mut sh, &answer(Role::Primary, SR * 2, "Mano, tipo assim.", 2.2)), Outcome::Held);
        assert_eq!(accept(&mut sh, &answer(Role::Extend, SR * 3, "Mano, tipo assim, tô", 2.6)), Outcome::Shown("text"));
        // An answer for less audio than what's shown is dropped.
        assert_eq!(accept(&mut sh, &answer(Role::Primary, SR * 3, "Mano tipo", 2.6)), Outcome::Stale);
        assert_eq!(accept(&mut sh, &answer(Role::Primary, SR * 2, "Mano tipo", 2.0)), Outcome::Stale);
        assert_eq!(accept(&mut sh, &answer(Role::Primary, SR * 4, "Legendas por Fulano", 3.5)), Outcome::Dropped);
        assert_eq!(accept(&mut sh, &answer(Role::Primary, SR * 5, "Mano, tipo assim, tô falando", 4.5)), Outcome::Shown("text"));
        assert_eq!(sh.line(), ("Mano, tipo assim, tô falando".to_string(), "Mano, tipo assim, tô".chars().count()));
        // Commit: the phrase becomes a segment, the next window starts at its end.
        let mut c = answer(Role::Primary, SR * 13, "Mano, tipo assim, tô falando meio baixo.", 12.0);
        c.commit = true;
        sh.commit_in_flight = true;
        assert_eq!(accept(&mut sh, &c), Outcome::Shown("text"));
        assert!(!sh.commit_in_flight);
        assert_eq!(sh.committed_at, SR * 13);
        let (line, stable) = sh.line();
        assert_eq!(stable, line.chars().count());
        // An answer for the committed phrase that lands late is ignored.
        assert_eq!(accept(&mut sh, &answer(Role::Extend, SR * 14, "whatever", 9.0)), Outcome::Old);
    }

    #[test]
    fn budget_gate() {
        let now = Instant::now();
        let mut b = Budget::new(now);
        // Fresh: some room on the minute estimate → faster than the refill rate.
        let Gate::Ready(g) = b.gate(now, 5.0, Some(20.0)) else { panic!() };
        assert!(g.as_secs_f32() >= LANE_MIN_GAP && g.as_secs_f32() < RPM_REFILL_SECS);
        // Groq reports the day and audio buckets: plenty → no extra spacing.
        let r = RateInfo {
            limit_requests: Some(2000),
            remaining_requests: Some(1938),
            reset_requests: Some(2678.4),
            limit_audio: Some(7200.0),
            remaining_audio: Some(7195.0),
            reset_audio: Some(2.5),
            org: None,
        };
        b.observe(now, &r);
        assert!((b.audio.unwrap().rate - 2.0).abs() < 1e-3);
        assert!((b.requests.unwrap().rate - 62.0 / 2678.4).abs() < 1e-4);
        assert_eq!(b.gate(now, 5.0, Some(20.0)), Gate::Ready(g));
        // Minute estimate used up: the primary keeps the last request for the final pass.
        for _ in 0..9 {
            b.book(now, 5.0);
        }
        assert!((b.rpm.remaining - 1.0).abs() < 1e-3);
        assert!(matches!(b.gate(now, 5.0, Some(20.0)), Gate::Pause(_)));
        assert!(matches!(b.gate(now, 5.0, None), Gate::Ready(_)));
        // Refills with time.
        assert!(matches!(b.gate(now + Duration::from_secs(4), 5.0, Some(20.0)), Gate::Ready(_)));
        // Audio budget nearly gone: pause until the final's audio + this request fit.
        let mut low = Budget::new(now);
        low.observe(now, &RateInfo { remaining_audio: Some(25.0), reset_audio: Some(3587.5), ..r.clone() });
        let Gate::Pause(p) = low.gate(now, 12.0, Some(20.0)) else { panic!() };
        assert!((p.as_secs_f32() - 3.5).abs() < 0.05); // (12 + 20 - 25) / 2 per second
        // Day budget low: slower than the minute limit alone.
        let mut day = Budget::new(now);
        day.observe(now, &RateInfo { remaining_requests: Some(60), reset_requests: Some(1940.0 * 43.2), ..r.clone() });
        let Gate::Ready(g) = day.gate(now, 5.0, Some(5.0)) else { panic!() };
        assert!(g.as_secs_f32() > 10.0);
        // 429: wait what the provider said.
        b.limited(now, Duration::from_secs(7));
        assert_eq!(b.gate(now, 5.0, None), Gate::Pause(Duration::from_secs(7)));
    }

    #[test]
    fn commit_policy() {
        let tone = |secs: f32, amp: f32| -> Vec<f32> { (0..(secs * SR as f32) as usize).map(|i| amp * ((i as f32) * 0.05).sin()).collect() };
        let mut speech = tone(12.5, 0.3);
        assert!(!should_commit(&speech)); // long, but no pause at the end
        speech.extend(vec![0.0; SR * 8 / 10]);
        assert!(should_commit(&speech)); // > 12 s and a 0.8 s pause
        let mut short = tone(8.0, 0.3);
        short.extend(vec![0.0; SR]);
        assert!(!should_commit(&short)); // pause, but the phrase is short
        assert!(should_commit(&tone(15.2, 0.3))); // forced
    }
}
