//! API keys live in the OS credential vault (Windows Credential Manager / Secret Service).
//! If the vault is unavailable (e.g. a Linux box without a keyring daemon) they fall back to
//! keys.json in the app config dir.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

const SERVICE: &str = "FlowVoice";
pub const KEY_PROVIDERS: [&str; 7] = ["groq", "gemini", "openai", "openrouter", "mistral", "custom", "tavily"];

#[derive(Serialize, Clone)]
pub struct KeyStatus {
    pub count: usize,
    pub masked: Vec<String>,
}

pub struct KeyStore {
    dir: PathBuf,
    cache: Mutex<BTreeMap<String, String>>,
}

fn split(raw: &str) -> Vec<String> {
    raw.split([',', '\n', ';'])
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect()
}

fn mask(k: &str) -> String {
    let chars: Vec<char> = k.chars().collect();
    if chars.len() <= 10 {
        return "•".repeat(chars.len().max(4));
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

impl KeyStore {
    pub fn new(dir: PathBuf) -> Self {
        let store = Self { dir, cache: Mutex::new(BTreeMap::new()) };
        let file = store.read_file();
        let mut cache = store.cache.lock();
        for p in KEY_PROVIDERS {
            // keys.json only holds a provider when the vault refused it (e.g. too long for the
            // Windows 2560-byte limit), so it is the newer copy and wins over the vault.
            let from_vault = || keyring::Entry::new(SERVICE, p).ok().and_then(|e| e.get_password().ok());
            if let Some(v) = file.get(p).cloned().or_else(from_vault) {
                if !v.trim().is_empty() {
                    cache.insert(p.to_string(), v);
                }
            }
        }
        drop(cache);
        store
    }

    fn file_path(&self) -> PathBuf {
        self.dir.join("keys.json")
    }

    fn read_file(&self) -> BTreeMap<String, String> {
        std::fs::read_to_string(self.file_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn write_file_entry(&self, provider: &str, value: Option<&str>) {
        let mut map = self.read_file();
        match value {
            Some(v) => {
                map.insert(provider.to_string(), v.to_string());
            }
            None => {
                map.remove(provider);
            }
        }
        let _ = std::fs::create_dir_all(&self.dir);
        if map.is_empty() {
            let _ = std::fs::remove_file(self.file_path());
        } else if let Ok(bytes) = serde_json::to_vec_pretty(&map) {
            let _ = std::fs::write(self.file_path(), bytes);
        }
    }

    /// Saves a comma-separated list of keys for a provider (empty string removes them).
    pub fn set(&self, provider: &str, raw: &str) -> anyhow::Result<()> {
        let keys = split(raw);
        let joined = keys.join(",");
        let entry = keyring::Entry::new(SERVICE, provider);
        if joined.is_empty() {
            if let Ok(e) = entry {
                let _ = e.delete_credential();
            }
            self.write_file_entry(provider, None);
            self.cache.lock().remove(provider);
            return Ok(());
        }
        let in_vault = entry.map(|e| e.set_password(&joined).is_ok()).unwrap_or(false);
        if in_vault {
            self.write_file_entry(provider, None);
        } else {
            log::warn!("credential vault unavailable, storing {provider} key in keys.json");
            // Drop the stale vault copy so an old key can never come back after a restart.
            if let Ok(e) = keyring::Entry::new(SERVICE, provider) {
                let _ = e.delete_credential();
            }
            self.write_file_entry(provider, Some(&joined));
        }
        self.cache.lock().insert(provider.to_string(), joined);
        Ok(())
    }

    pub fn get(&self, provider: &str) -> Vec<String> {
        self.cache.lock().get(provider).map(|v| split(v)).unwrap_or_default()
    }

    pub fn raw(&self, provider: &str) -> String {
        self.get(provider).join(", ")
    }

    pub fn has(&self, provider: &str) -> bool {
        !self.get(provider).is_empty()
    }

    pub fn status(&self) -> BTreeMap<String, KeyStatus> {
        KEY_PROVIDERS
            .iter()
            .map(|p| {
                let keys = self.get(p);
                (p.to_string(), KeyStatus { count: keys.len(), masked: keys.iter().map(|k| mask(k)).collect() })
            })
            .collect()
    }

    /// Imports keys from the 1.x Python app (config.json) the first time v2 runs.
    pub fn import_legacy(&self) -> usize {
        let Some(base) = legacy_config_dir() else { return 0 };
        let Ok(txt) = std::fs::read_to_string(base.join("config.json")) else { return 0 };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&txt) else { return 0 };
        let mut imported = 0;
        let mut take = |provider: &str, value: Option<&str>| {
            if let Some(v) = value {
                if !v.trim().is_empty() && !self.has(provider) && self.set(provider, v).is_ok() {
                    imported += 1;
                }
            }
        };
        let keys = &json["api_keys"];
        for p in ["groq", "gemini", "openai"] {
            take(p, keys[p].as_str());
        }
        take("tavily", json["tavily_api_key"].as_str());
        imported
    }
}

fn legacy_config_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("FlowVoice"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|c| c.join("FlowVoice"))
    }
}
