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
    pub id: &'static str,
    pub label: &'static str,
    pub file: &'static str,
    pub size_mb: u32,
    pub note: &'static str,
    pub recommended: bool,
}

/// Sizes from huggingface.co/ggerganov/whisper.cpp (checked 2026-10-29).
pub const MODELS: [ModelSpec; 5] = [
    ModelSpec { id: "tiny-q5_1", label: "Tiny", file: "ggml-tiny-q5_1.bin", size_mb: 31, note: "Mais leve — PCs bem fracos", recommended: false },
    ModelSpec { id: "base-q5_1", label: "Base", file: "ggml-base-q5_1.bin", size_mb: 57, note: "Rápido, ideal para a prévia ao vivo", recommended: true },
    ModelSpec { id: "small-q5_1", label: "Small", file: "ggml-small-q5_1.bin", size_mb: 181, note: "Equilíbrio entre precisão e velocidade", recommended: false },
    ModelSpec { id: "turbo-q5_0", label: "Large v3 Turbo", file: "ggml-large-v3-turbo-q5_0.bin", size_mb: 547, note: "Máxima precisão (PC mais forte)", recommended: false },
    ModelSpec { id: "medium-q5_0", label: "Medium", file: "ggml-medium-q5_0.bin", size_mb: 514, note: "Preciso, mais lento que o Turbo", recommended: false },
];

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
}

#[derive(Serialize, Clone)]
pub struct ModelProgress {
    pub id: String,
    pub downloaded: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

pub struct LocalStt {
    dir: PathBuf,
    loaded: Mutex<Option<(String, Arc<WhisperContext>)>>,
    downloading: Mutex<Vec<String>>,
}

impl LocalStt {
    pub fn new(dir: PathBuf) -> Self {
        whisper_rs::install_logging_hooks();
        Self { dir, loaded: Mutex::new(None), downloading: Mutex::new(vec![]) }
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
            })
            .collect()
    }

    /// Smallest installed model, used when the configured one is missing.
    pub fn any_installed(&self) -> Option<&'static str> {
        ["base-q5_1", "small-q5_1", "turbo-q5_0", "medium-q5_0", "tiny-q5_1"]
            .into_iter()
            .find(|id| self.installed(id))
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
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
        params.set_n_threads(threads as i32);
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
        if fast {
            params.set_temperature_inc(0.0);
            // Encode only the audio we have instead of the fixed 30 s window (big win for the
            // short clips of the live preview; slightly less accurate, so final passes skip it).
            let secs = samples.len() as f32 / 16_000.0;
            if secs < 28.0 {
                params.set_audio_ctx(((secs.max(1.5) / 30.0 * 1500.0) as i32 + 128).min(1500));
            }
        }
        if !prompt.trim().is_empty() {
            params.set_initial_prompt(prompt);
        }
        // whisper.cpp needs at least ~1s of audio; pad short clips with silence.
        let mut audio = samples.to_vec();
        if audio.len() < 16_000 + 1_600 {
            audio.resize(16_000 + 1_600, 0.0);
        }
        state.full(params, &audio).map_err(|e| anyhow!("whisper: {e:?}"))?;
        let mut out = String::new();
        for seg in state.as_iter() {
            if seg.no_speech_probability() > 0.85 {
                continue;
            }
            if let Ok(t) = seg.to_str_lossy() {
                out.push_str(&t);
            }
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
        let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", spec.file);
        let resp = reqwest::Client::new()
            .get(&url)
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
