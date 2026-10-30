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

pub struct ActiveSession {
    pub rec: Recording,
    pub mode: RecordMode,
    pub target_window: isize,
    pub started: std::time::Instant,
    pub stop_flag: Arc<AtomicBool>,
}

pub struct AppState {
    pub config_dir: PathBuf,
    pub settings: RwLock<Settings>,
    pub keys: KeyStore,
    pub local: Arc<LocalStt>,
    pub http: reqwest::Client,
    pub session: Mutex<Option<ActiveSession>>,
    pub busy: AtomicBool,
    /// Last finished dictation + the window it was pasted into (for reprocess/replace).
    pub last: Mutex<Option<(DictationResult, isize)>>,
    pub history: Mutex<Vec<DictationResult>>,
    pub chat: Mutex<Vec<ChatMessage>>,
    pub chat_rec: Mutex<Option<Recording>>,
    pub muted_prev: Mutex<Option<bool>>,
    pub overlay_hold: AtomicBool,
    /// Bumped on every overlay change; delayed hides only fire if it didn't move.
    pub overlay_gen: AtomicU64,
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
            busy: AtomicBool::new(false),
            last: Mutex::new(None),
            history: Mutex::new(history),
            chat: Mutex::new(vec![]),
            chat_rec: Mutex::new(None),
            muted_prev: Mutex::new(None),
            overlay_hold: AtomicBool::new(false),
            overlay_gen: AtomicU64::new(0),
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
        if let Ok(bytes) = serde_json::to_vec(h) {
            let _ = std::fs::write(self.config_dir.join("history.json"), bytes);
        }
    }
}

pub fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{n:x}")
}
