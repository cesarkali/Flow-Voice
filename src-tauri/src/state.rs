//! Shared application state and small persisted collections (history).

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

use crate::audio::Recording;
use crate::keys::KeyStore;
use crate::local_stt::LocalStt;
use crate::settings::{self, Settings};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordMode {
    Dictate,
    Translate,
    Ask,
    /// Apply a spoken instruction to the text selected in the target app.
    Command,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DictationResult {
    pub id: String,
    pub ts: String,
    pub mode: RecordMode,
    pub raw: String,
    pub text: String,
    pub style_id: String,
    pub target: Option<String>,
    pub provider: Option<String>,
    pub stt_provider: String,
    pub pasted: bool,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
    /// Normalized id of the app the text went to ("outlook"); missing in older history files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// Friendly name of that app ("Outlook").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Command mode: the selected text the instruction (`raw`) was applied to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The transcription failed (`error` says why): no text; the recording is kept in memory to
    /// try again (see `FailedStore`). Never written to history.json.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// For the UI only (filled by `get_history`): whether a failed item's recording is still kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryState>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RetryState {
    /// Kept: can be tried again.
    Ready,
    /// Being tried again right now.
    Running,
}

/// A recording whose transcription failed, kept in memory only (never on disk) to try again from
/// the error card or the history, as it was dictated: same app, style and translation.
#[derive(Clone, Debug)]
pub struct FailedRecording {
    /// Same id as its history item.
    pub id: String,
    /// The voiced audio (silences already trimmed) as 16-bit samples: half the room of floats.
    pub pcm: Vec<i16>,
    pub mode: RecordMode,
    pub target_window: isize,
    pub app: Option<crate::apps::AppContext>,
    pub style_id: String,
    pub target: Option<String>,
    pub duration_ms: u64,
    pub retrying: bool,
}

pub fn to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16).collect()
}

pub fn from_pcm16(pcm: &[i16]) -> Vec<f32> {
    pcm.iter().map(|&s| s as f32 / 32767.0).collect()
}

/// The newest few failed recordings (a long dictation is a few MB).
#[derive(Default)]
pub struct FailedStore {
    items: std::collections::VecDeque<FailedRecording>,
}

impl FailedStore {
    pub const MAX: usize = 3;

    /// Keeps `rec`; returns the ids of the oldest ones dropped past `MAX`.
    pub fn keep(&mut self, rec: FailedRecording) -> Vec<String> {
        self.items.retain(|f| f.id != rec.id);
        self.items.push_back(rec);
        let mut dropped = vec![];
        while self.items.len() > Self::MAX {
            if let Some(f) = self.items.pop_front() {
                dropped.push(f.id);
            }
        }
        dropped
    }

    pub fn drop_id(&mut self, id: &str) {
        self.items.retain(|f| f.id != id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn state(&self, id: &str) -> Option<RetryState> {
        self.items.iter().find(|f| f.id == id).map(|f| if f.retrying { RetryState::Running } else { RetryState::Ready })
    }

    /// Marks `id` as being tried again and returns a copy; None when it is gone or already running.
    pub fn start_retry(&mut self, id: &str) -> Option<FailedRecording> {
        let f = self.items.iter_mut().find(|f| f.id == id && !f.retrying)?;
        f.retrying = true;
        Some(f.clone())
    }

    pub fn end_retry(&mut self, id: &str) {
        if let Some(f) = self.items.iter_mut().find(|f| f.id == id) {
            f.retrying = false;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatSource {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub sources: Vec<ChatSource>,
}

/// Text selected in the target app for a Command recording.
#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    /// Still being copied (toggle mode copies right away, in the background).
    Pending,
    /// Hold mode: copied when the keys are released.
    AtStop,
    Got(String),
    Empty,
    Failed(String),
}

pub struct ActiveSession {
    pub rec: Recording,
    pub mode: RecordMode,
    pub target_window: isize,
    pub started: std::time::Instant,
    pub stop_flag: Arc<AtomicBool>,
    /// App that had focus at start (per-app style, LLM context, history).
    pub app: Option<crate::apps::AppContext>,
    /// Style for this dictation: a per-app rule or the active style.
    pub style_id: String,
    pub selection: Arc<Mutex<Selection>>,
}

pub struct AppState {
    pub config_dir: PathBuf,
    pub settings: RwLock<Settings>,
    pub keys: KeyStore,
    pub local: Arc<LocalStt>,
    pub http: reqwest::Client,
    pub session: Mutex<Option<ActiveSession>>,
    /// Last finished dictation + the window it was pasted into (for reprocess/replace).
    pub last: Mutex<Option<(DictationResult, isize)>>,
    pub history: Mutex<Vec<DictationResult>>,
    /// Recordings whose transcription failed, to try again (memory only).
    pub failed: Mutex<FailedStore>,
    pub chat: Mutex<Vec<ChatMessage>>,
    pub chat_rec: Mutex<Option<Recording>>,
    pub muted_prev: Mutex<Option<bool>>,
    pub overlay_hold: AtomicBool,
    /// Bumped on every overlay change; delayed hides only fire if it didn't move.
    pub overlay_gen: AtomicU64,
    /// Work-area origin of the monitor the overlay is pinned to while visible.
    pub overlay_monitor: Mutex<Option<(i32, i32)>>,
}

const HISTORY_MAX: usize = 300;

impl AppState {
    pub fn new(config_dir: PathBuf, models_dir: PathBuf) -> Self {
        let settings = settings::load(&config_dir);
        let history = std::fs::read_to_string(config_dir.join("history.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self {
            keys: KeyStore::new(config_dir.clone()),
            local: Arc::new(LocalStt::new(models_dir)),
            http: crate::cloud::http(),
            settings: RwLock::new(settings),
            session: Mutex::new(None),
            last: Mutex::new(None),
            history: Mutex::new(history),
            failed: Mutex::new(FailedStore::default()),
            chat: Mutex::new(vec![]),
            chat_rec: Mutex::new(None),
            muted_prev: Mutex::new(None),
            overlay_hold: AtomicBool::new(false),
            overlay_gen: AtomicU64::new(0),
            overlay_monitor: Mutex::new(None),
            config_dir,
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn save_settings(&self, s: Settings) -> anyhow::Result<Settings> {
        let mut s = s;
        s.normalize();
        settings::save(&self.config_dir, &s)?;
        *self.settings.write() = s.clone();
        Ok(s)
    }

    pub fn push_history(&self, item: &DictationResult) {
        if !self.settings.read().history_enabled {
            return;
        }
        let mut h = self.history.lock();
        if let Some(existing) = h.iter_mut().find(|x| x.id == item.id) {
            *existing = item.clone();
        } else {
            h.insert(0, item.clone());
            h.truncate(HISTORY_MAX);
        }
        self.persist_history(&h);
    }

    pub fn persist_history(&self, h: &[DictationResult]) {
        if let Ok(bytes) = history_json(h) {
            let _ = std::fs::write(self.config_dir.join("history.json"), bytes);
        }
    }

    /// Removes a history item and its kept recording, if any.
    pub fn delete_history(&self, id: &str) {
        self.failed.lock().drop_id(id);
        let mut h = self.history.lock();
        let before = h.len();
        h.retain(|x| x.id != id);
        if h.len() != before {
            self.persist_history(&h);
        }
    }

    /// Keeps a failed recording (dropping the oldest past `FailedStore::MAX`, and their history
    /// items: they can't be tried again) and, with the history on, lists `item` there.
    pub fn keep_failed(&self, rec: FailedRecording, item: &DictationResult) {
        let dropped = self.failed.lock().keep(rec);
        for id in dropped {
            self.delete_history(&id);
        }
        self.push_history(item);
    }

    /// History for the UI: failed items say whether their recording is still kept.
    pub fn history_for_ui(&self) -> Vec<DictationResult> {
        let failed = self.failed.lock();
        let mut h = self.history.lock().clone();
        for item in h.iter_mut().filter(|x| x.failed) {
            item.retry = failed.state(&item.id);
        }
        h
    }
}

/// history.json: failed items stay out (their recording is memory-only, so after a restart
/// they could never be tried again).
fn history_json(h: &[DictationResult]) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec(&h.iter().filter(|x| !x.failed).collect::<Vec<_>>())
}

pub fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{n:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str) -> FailedRecording {
        FailedRecording {
            id: id.into(),
            pcm: vec![0, 1000, -1000],
            mode: RecordMode::Dictate,
            target_window: 1,
            app: None,
            style_id: "natural".into(),
            target: None,
            duration_ms: 3000,
            retrying: false,
        }
    }

    #[test]
    fn failed_store_keeps_the_newest_three() {
        let mut f = FailedStore::default();
        for id in ["a", "b", "c"] {
            assert!(f.keep(rec(id)).is_empty());
        }
        // The oldest goes (and the caller drops its history item).
        assert_eq!(f.keep(rec("d")), vec!["a".to_string()]);
        assert_eq!(f.state("a"), None);
        assert_eq!(f.state("d"), Some(RetryState::Ready));
        f.drop_id("b");
        assert_eq!(f.state("b"), None);
        f.clear();
        assert_eq!(f.state("c"), None);
    }

    #[test]
    fn a_retry_runs_once_at_a_time() {
        let mut f = FailedStore::default();
        f.keep(rec("a"));
        assert!(f.start_retry("a").is_some());
        assert_eq!(f.state("a"), Some(RetryState::Running));
        assert!(f.start_retry("a").is_none(), "a second click while it runs");
        f.end_retry("a");
        assert_eq!(f.state("a"), Some(RetryState::Ready));
        assert!(f.start_retry("gone").is_none());
    }

    #[test]
    fn pcm16_round_trip() {
        let samples = [0.0f32, 0.5, -0.5, 1.0, -1.0, 2.0];
        let back = from_pcm16(&to_pcm16(&samples));
        for (a, b) in samples.iter().zip(&back) {
            assert!((a.clamp(-1.0, 1.0) - b).abs() < 1e-4, "{a} -> {b}");
        }
    }

    #[test]
    fn failed_items_never_reach_history_json() {
        let ok: DictationResult = serde_json::from_str(
            r#"{"id":"1","ts":"","mode":"dictate","raw":"oi","text":"Oi.","style_id":"natural","target":null,"provider":null,"stt_provider":"groq","pasted":true,"duration_ms":900,"warnings":[]}"#,
        )
        .unwrap();
        assert!(!ok.failed, "older files have no `failed`");
        let failed = DictationResult { id: "2".into(), failed: true, error: Some("Falha na transcrição".into()), ..ok.clone() };
        let json = String::from_utf8(history_json(&[failed, ok]).unwrap()).unwrap();
        assert!(json.contains(r#""id":"1""#));
        assert!(!json.contains(r#""id":"2""#));
        assert!(!json.contains("failed") && !json.contains("retry") && !json.contains("error"));
    }
}
