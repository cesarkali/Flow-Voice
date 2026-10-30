//! Offline transcription with whisper.cpp (ggml models downloaded on demand from Hugging Face).

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct ModelSpec {
    /// Stable id stored in settings (stt.local_model / stt.preview_model) — never rename.
    pub id: &'static str,
    pub label: &'static str,
    /// File name inside the models folder.
    pub file: &'static str,
    /// Direct download URL (Hugging Face `resolve/main`).
    pub url: &'static str,
    pub size_mb: u32,
    /// Approximate peak RAM while transcribing (model + whisper.cpp buffers), measured on Windows.
    pub ram_mb: u32,
    /// Languages it can transcribe; `"*"` = all Whisper languages.
    pub langs: &'static [&'static str],
    /// Languages it is the best pick for; `"*"` = every language it supports.
    pub recommended_for: &'static [&'static str],
    /// 1 (slowest) ..= 5 (fastest) on a mid-range CPU.
    pub speed: u8,
    /// 1 (most mistakes) ..= 5 (most accurate).
    pub quality: u8,
    /// Architecture · quantization, shown in small print.
    pub tech: &'static str,
    pub note: &'static str,
    /// General default (onboarding).
    pub recommended: bool,
}

const ALL: &[&str] = &["*"];

macro_rules! ggerganov {
    ($file:literal) => {
        concat!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/", $file)
    };
}

/// High-resource languages where Whisper small is already a solid everyday pick (Whisper paper,
/// FLEURS per-language results); elsewhere only turbo is recommended.
const SMALL_OK: &[&str] = &["en", "es", "pt", "it", "de", "fr", "ca", "nl", "ja", "ru", "pl", "ko", "id", "sv", "tr", "gl", "uk", "zh", "ms", "no", "vi", "fi", "cs", "ro"];

/// Catalog. Sizes/URLs checked with HEAD requests on 2026-10-30; RAM and tiers measured on an
/// i5-10400F (see live_tests::live_local_bench).
pub const MODELS: [ModelSpec; 8] = [
    ModelSpec {
        id: "tiny-q5_1", label: "Tiny", file: "ggml-tiny-q5_1.bin", url: ggerganov!("ggml-tiny-q5_1.bin"),
        size_mb: 31, ram_mb: 150, langs: ALL, recommended_for: &[], speed: 5, quality: 1,
        tech: "whisper tiny · q5_1", note: "Mais leve — PCs bem fracos", recommended: false,
    },
    ModelSpec {
        id: "base-q5_1", label: "Base", file: "ggml-base-q5_1.bin", url: ggerganov!("ggml-base-q5_1.bin"),
        size_mb: 57, ram_mb: 300, langs: ALL, recommended_for: &[], speed: 5, quality: 2,
        tech: "whisper base · q5_1", note: "Rápido, ideal para a prévia ao vivo", recommended: true,
    },
    ModelSpec {
        id: "base-en-q5_1", label: "Base (English)", file: "ggml-base.en-q5_1.bin", url: ggerganov!("ggml-base.en-q5_1.bin"),
        size_mb: 57, ram_mb: 300, langs: &["en"], recommended_for: &["en"], speed: 5, quality: 3,
        tech: "whisper base.en · q5_1", note: "Só inglês — mais preciso que o Base no mesmo tamanho", recommended: false,
    },
    ModelSpec {
        id: "small-q5_1", label: "Small", file: "ggml-small-q5_1.bin", url: ggerganov!("ggml-small-q5_1.bin"),
        size_mb: 181, ram_mb: 590, langs: ALL, recommended_for: SMALL_OK, speed: 4, quality: 3,
        tech: "whisper small · q5_1", note: "Equilíbrio entre precisão e velocidade", recommended: false,
    },
    ModelSpec {
        id: "small-en-q5_1", label: "Small (English)", file: "ggml-small.en-q5_1.bin", url: ggerganov!("ggml-small.en-q5_1.bin"),
        size_mb: 181, ram_mb: 590, langs: &["en"], recommended_for: &["en"], speed: 4, quality: 4,
        tech: "whisper small.en · q5_1", note: "Só inglês — ótimo equilíbrio", recommended: false,
    },
    ModelSpec {
        id: "distil-ptbr-q5_0", label: "Distil Large v3 PT-BR", file: "ggml-distil-large-v3-ptbr-q5_0.bin",
        url: "https://huggingface.co/JoaoZaokk/distil-whisper-large-v3-ptbr-ggml/resolve/main/ggml-distil-whisper-large-v3-ptbr-q5_0.bin",
        size_mb: 513, ram_mb: 950, langs: &["pt"], recommended_for: &["pt"], speed: 2, quality: 5,
        tech: "distil-whisper large-v3 pt-BR (freds0) · q5_0", note: "Ajustado para português do Brasil (freds0, MIT)", recommended: false,
    },
    ModelSpec {
        id: "turbo-q5_0", label: "Large v3 Turbo", file: "ggml-large-v3-turbo-q5_0.bin", url: ggerganov!("ggml-large-v3-turbo-q5_0.bin"),
        size_mb: 547, ram_mb: 1020, langs: ALL, recommended_for: ALL, speed: 2, quality: 5,
        tech: "whisper large-v3-turbo · q5_0", note: "Máxima precisão (PC mais forte)", recommended: false,
    },
    ModelSpec {
        id: "medium-q5_0", label: "Medium", file: "ggml-medium-q5_0.bin", url: ggerganov!("ggml-medium-q5_0.bin"),
        size_mb: 514, ram_mb: 1200, langs: ALL, recommended_for: &[], speed: 1, quality: 4,
        tech: "whisper medium · q5_0", note: "Preciso, mais lento que o Turbo", recommended: false,
    },
];

#[allow(dead_code)]
impl ModelSpec {
    pub fn supports(&self, lang: &str) -> bool {
        self.langs.contains(&"*") || self.langs.contains(&lang)
    }
    pub fn recommended_for_lang(&self, lang: &str) -> bool {
        self.supports(lang) && (self.recommended_for.contains(&"*") || self.recommended_for.contains(&lang))
    }
}

/// whisper.cpp is built with AVX2/FMA; older CPUs would crash on the first local call.
pub fn cpu_supported() -> bool {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma")
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        true
    }
}

pub fn spec(id: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.id == id)
}

#[derive(Serialize, Clone)]
pub struct LocalModelInfo {
    pub id: String,
    pub label: String,
    pub file: String,
    pub size_mb: u32,
    pub installed: bool,
    pub note: String,
    pub recommended: bool,
    pub ram_mb: u32,
    pub langs: Vec<String>,
    pub recommended_for: Vec<String>,
    pub speed: u8,
    pub quality: u8,
    pub tech: String,
}

#[derive(Serialize, Clone)]
pub struct ModelProgress {
    pub id: String,
    pub downloaded: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

/// A loaded model is dropped after this long without use, so nothing sits in RAM between
/// dictations (reloading a small model takes well under a second).
const IDLE_UNLOAD: std::time::Duration = std::time::Duration::from_secs(15);

pub struct LocalStt {
    dir: PathBuf,
    loaded: Mutex<Option<(String, Arc<WhisperContext>)>>,
    last_used: Mutex<std::time::Instant>,
    downloading: Mutex<Vec<String>>,
}

impl LocalStt {
    pub fn new(dir: PathBuf) -> Self {
        whisper_rs::install_logging_hooks();
        Self { dir, loaded: Mutex::new(None), last_used: Mutex::new(std::time::Instant::now()), downloading: Mutex::new(vec![]) }
    }

    /// Background janitor: frees the model once it has been idle for IDLE_UNLOAD.
    pub fn start_idle_unloader(self: &Arc<Self>) {
        let me = Arc::downgrade(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let Some(me) = me.upgrade() else { return };
            let idle = me.last_used.lock().elapsed() > IDLE_UNLOAD;
            if idle {
                let mut loaded = me.loaded.lock();
                // Arc count 1 = no transcription is using it right now.
                if loaded.as_ref().is_some_and(|(_, ctx)| Arc::strong_count(ctx) == 1) {
                    let (id, _) = loaded.take().unwrap();
                    log::info!("local model {id} unloaded (idle)");
                }
            }
        });
    }

    pub fn path(&self, id: &str) -> Option<PathBuf> {
        spec(id).map(|s| self.dir.join(s.file))
    }

    /// Installed AND runnable on this CPU.
    pub fn installed(&self, id: &str) -> bool {
        cpu_supported() && self.path(id).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn list(&self) -> Vec<LocalModelInfo> {
        MODELS
            .iter()
            .map(|m| LocalModelInfo {
                id: m.id.into(),
                label: m.label.into(),
                file: m.file.into(),
                size_mb: m.size_mb,
                installed: self.path(m.id).map(|p| p.exists()).unwrap_or(false),
                note: m.note.into(),
                recommended: m.recommended,
                ram_mb: m.ram_mb,
                langs: m.langs.iter().map(|l| l.to_string()).collect(),
                recommended_for: m.recommended_for.iter().map(|l| l.to_string()).collect(),
                speed: m.speed,
                quality: m.quality,
                tech: m.tech.into(),
            })
            .collect()
    }

    /// Fastest decent multilingual model installed, used when the configured one is missing.
    /// Language-specific models are skipped: an English-only model would garble Portuguese.
    pub fn any_installed(&self) -> Option<&'static str> {
        ["base-q5_1", "small-q5_1", "turbo-q5_0", "medium-q5_0", "tiny-q5_1"]
            .into_iter()
            .find(|id| self.installed(id))
    }

    /// Best installed model for `lang` (pipeline.rs can use it instead of `any_installed`): recommended ones first (most accurate first), then any
    /// other model that supports the language, then the multilingual fallback.
    #[allow(dead_code)]
    pub fn any_installed_for(&self, lang: &str) -> Option<&'static str> {
        let mut ok: Vec<&'static ModelSpec> = MODELS.iter().filter(|m| m.supports(lang) && self.installed(m.id)).collect();
        ok.sort_by_key(|m| (!m.recommended_for_lang(lang), std::cmp::Reverse(m.quality), std::cmp::Reverse(m.speed)));
        ok.first().map(|m| m.id).or_else(|| self.any_installed())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        {
            let mut loaded = self.loaded.lock();
            if loaded.as_ref().is_some_and(|(lid, _)| lid == id) {
                *loaded = None;
            }
        }
        let p = self.path(id).ok_or_else(|| anyhow!("modelo desconhecido"))?;
        if p.exists() {
            std::fs::remove_file(p)?;
        }
        Ok(())
    }

    fn context(&self, id: &str) -> Result<Arc<WhisperContext>> {
        *self.last_used.lock() = std::time::Instant::now();
        let mut loaded = self.loaded.lock();
        if let Some((lid, ctx)) = &*loaded {
            if lid == id {
                return Ok(ctx.clone());
            }
        }
        if !cpu_supported() {
            return Err(anyhow!("este processador não tem AVX2, necessário para o modo local — use a transcrição na nuvem"));
        }
        let path = self.path(id).ok_or_else(|| anyhow!("modelo local desconhecido: {id}"))?;
        if !path.exists() {
            return Err(anyhow!("o modelo local \"{id}\" não foi baixado"));
        }
        let ctx = WhisperContext::new_with_params(&path, WhisperContextParameters::default())
            .map_err(|e| anyhow!("falha ao carregar o modelo {id}: {e:?}"))?;
        let ctx = Arc::new(ctx);
        log::info!("local model {id} loaded");
        *loaded = Some((id.to_string(), ctx.clone()));
        Ok(ctx)
    }

    /// Loads the model in the background so the first dictation doesn't pay for it.
    pub fn warm_up(&self, id: &str) {
        if self.installed(id) {
            let _ = self.context(id);
        }
    }

    /// Transcribes 16 kHz mono f32 samples.
    pub fn transcribe(&self, id: &str, samples: &[f32], lang: &str, prompt: &str, fast: bool) -> Result<String> {
        let ctx = self.context(id)?;
        let mut state = ctx.create_state().map_err(|e| anyhow!("whisper: {e:?}"))?;
        let strategy = if fast {
            SamplingStrategy::Greedy { best_of: 1 }
        } else {
            SamplingStrategy::BeamSearch { beam_size: 3, patience: -1.0 }
        };
        let mut params = FullParams::new(strategy);
        // More than ~6 threads buys nothing (whisper.cpp scales with physical cores) and makes the
        // PC sluggish while it runs.
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 6);
        params.set_n_threads(tune("FV_THREADS", threads as f32) as i32);
        let lang_opt = if lang == "auto" { None } else { Some(lang) };
        params.set_language(lang_opt);
        params.set_detect_language(false);
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_no_timestamps(true);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_no_context(true);
        if !fast {
            params.set_temperature_inc(tune("FV_TINC", 0.2));
        }
        if fast {
            params.set_temperature_inc(0.0);
        }
        if fast || tune("FV_ACTX", 1.0) > 0.0 {
            // Encode only the audio we have instead of the fixed 30 s window. Measured on pt-BR
            // dictation (live_local_bench): 2-4x faster on short clips for every model, and small
            // stops dropping most of a quiet recording (WER 72 % -> 11 %); turbo/distil unchanged.
            let secs = samples.len() as f32 / 16_000.0;
            if secs < 28.0 {
                params.set_audio_ctx(((secs.max(1.5) / 30.0 * 1500.0) as i32 + 128).min(1500));
            }
        }
        if !prompt.trim().is_empty() && tune("FV_PROMPT", 1.0) > 0.0 {
            params.set_initial_prompt(prompt);
        }
        // whisper.cpp needs at least ~1s of audio; pad short clips with silence.
        let mut audio = samples.to_vec();
        if audio.len() < 16_000 + 1_600 {
            audio.resize(16_000 + 1_600, 0.0);
        }
        let res = state.full(params, &audio);
        *self.last_used.lock() = std::time::Instant::now();
        res.map_err(|e| anyhow!("whisper: {e:?}"))?;
        let mut out = String::new();
        for seg in state.as_iter() {
            let Ok(t) = seg.to_str_lossy() else { continue };
            let no_speech = seg.no_speech_probability();
            // Mean log-probability of the text tokens (special tokens like [_BEG_] excluded).
            let (mut sum, mut n) = (0.0f32, 0);
            for i in 0..seg.n_tokens() {
                if let Some(tok) = seg.get_token(i) {
                    if tok.to_str_lossy().map(|s| s.starts_with("[_") || s.starts_with("<|")).unwrap_or(true) {
                        continue;
                    }
                    sum += tok.token_data().plog;
                    n += 1;
                }
            }
            let avg_logprob = if n > 0 { sum / n as f32 } else { -10.0 };
            if cfg!(test) && std::env::var("FV_DEBUG").is_ok() {
                println!("    seg no_speech={no_speech:.2} avg_logprob={avg_logprob:.2} {t:?}");
            }
            if is_hallucination(&t, no_speech, avg_logprob) {
                log::info!("local stt: dropped likely hallucination (no_speech {no_speech:.2}, logprob {avg_logprob:.2})");
                continue;
            }
            out.push_str(&t);
        }
        Ok(out.trim().to_string())
    }

    pub async fn download(self: Arc<Self>, app: AppHandle, id: String) -> Result<()> {
        let spec = spec(&id).ok_or_else(|| anyhow!("modelo desconhecido"))?;
        {
            let mut d = self.downloading.lock();
            if d.contains(&id) {
                return Ok(());
            }
            d.push(id.clone());
        }
        let result = self.download_inner(&app, spec).await;
        self.downloading.lock().retain(|x| x != &id);
        let progress = |error: Option<String>| ModelProgress { id: id.clone(), downloaded: 0, total: 0, done: true, error };
        match &result {
            Ok(_) => {
                let _ = app.emit("model-progress", progress(None));
            }
            Err(e) => {
                let _ = app.emit("model-progress", progress(Some(e.to_string())));
            }
        }
        result
    }

    async fn download_inner(&self, app: &AppHandle, spec: &ModelSpec) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let resp = reqwest::Client::new()
            .get(spec.url)
            .send()
            .await
            .context("sem conexão com o Hugging Face")?
            .error_for_status()
            .context("download recusado")?;
        let total = resp.content_length().unwrap_or(spec.size_mb as u64 * 1_048_576);
        let tmp = self.dir.join(format!("{}.part", spec.file));
        let mut file = std::fs::File::create(&tmp)?;
        let mut stream = resp.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_emit = std::time::Instant::now();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("conexão interrompida")?;
            std::io::Write::write_all(&mut file, &chunk)?;
            downloaded += chunk.len() as u64;
            if last_emit.elapsed().as_millis() > 150 {
                last_emit = std::time::Instant::now();
                let _ = app.emit(
                    "model-progress",
                    ModelProgress { id: spec.id.into(), downloaded, total, done: false, error: None },
                );
            }
        }
        drop(file);
        if downloaded < 1_000_000 {
            let _ = std::fs::remove_file(&tmp);
            return Err(anyhow!("arquivo incompleto"));
        }
        std::fs::rename(&tmp, self.dir.join(spec.file))?;
        Ok(())
    }
}

/// Benchmark knobs (live_tests): read from the environment in test builds only.
fn tune(name: &str, default: f32) -> f32 {
    if cfg!(test) {
        if let Some(v) = std::env::var(name).ok().and_then(|v| v.parse().ok()) {
            return v;
        }
    }
    default
}

/// Phrases Whisper invents on silence/noise (subtitle credits from its YouTube training data).
const HALLUCINATIONS: &[&str] = &[
    "amara.org", "legendas pela comunidade", "legenda adriana zanotto", "obrigado por assistir", "inscreva-se no canal",
    "thanks for watching", "thank you for watching", "subtitles by", "please subscribe", "subtítulos realizados por",
    "gracias por ver", "sous-titres réalisés", "untertitel der amara", "untertitel im auftrag",
];

/// OpenAI's rule (silent when the model thinks there's no speech AND isn't confident in the
/// text), plus known credit lines. A bare high no-speech score is not enough: on quiet mics
/// whisper reports it for real speech too.
fn is_hallucination(text: &str, no_speech: f32, avg_logprob: f32) -> bool {
    if tune("FV_OLD", 0.0) > 0.0 {
        return no_speech > 0.85;
    }
    let lower = text.to_lowercase();
    if HALLUCINATIONS.iter().any(|h| lower.contains(h)) {
        return true;
    }
    // "Legenda por Fulano" / "Legendas: Fulano" — a credit line, never dictation.
    let t = lower.trim_start();
    if (t.starts_with("legenda") || t.starts_with("subtitles") || t.starts_with("subtítulos")) && t.split_whitespace().count() <= 8 {
        return true;
    }
    let ns = tune("FV_NS", 0.6);
    let lp = tune("FV_LP", -1.0);
    no_speech > ns && avg_logprob < lp
}
