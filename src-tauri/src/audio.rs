//! Microphone capture: any device format → 16 kHz mono f32 kept in memory.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

pub const SAMPLE_RATE: u32 = 16_000;

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// Box-filter resampler to 16 kHz, stateful across callbacks.
struct Resampler {
    step: f64,
    pos: f64,
    sum: f32,
    count: u32,
}

impl Resampler {
    fn new(input_rate: u32) -> Self {
        Self { step: input_rate as f64 / SAMPLE_RATE as f64, pos: 0.0, sum: 0.0, count: 0 }
    }

    fn push(&mut self, x: f32, out: &mut Vec<f32>) {
        self.sum += x;
        self.count += 1;
        self.pos += 1.0;
        while self.pos >= self.step {
            out.push(self.sum / self.count.max(1) as f32);
            self.pos -= self.step;
            if self.pos < self.step {
                self.sum = 0.0;
                self.count = 0;
            }
        }
    }
}

pub struct Recording {
    pub samples: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    stop_tx: mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Recording {
    /// Opens the microphone and starts capturing right away.
    pub fn start(device_name: Option<&str>) -> Result<Self> {
        let samples = Arc::new(Mutex::new(Vec::with_capacity(SAMPLE_RATE as usize * 60)));
        let level = Arc::new(AtomicU32::new(0));
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();
        let device_name = device_name.map(str::to_string);
        let s = samples.clone();
        let l = level.clone();
        // cpal streams are !Send on some platforms, so the stream lives on its own thread.
        let thread = std::thread::spawn(move || {
            let stream = match build_stream(device_name.as_deref(), s, l) {
                Ok(st) => st,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            if let Err(e) = stream.play() {
                let _ = ready_tx.send(Err(anyhow!("não foi possível iniciar o microfone: {e}")));
                return;
            }
            let _ = ready_tx.send(Ok(()));
            let _ = stop_rx.recv();
            drop(stream);
        });
        match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self { samples, level, stop_tx, thread: Some(thread) }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("o microfone não respondeu")),
        }
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn finish(mut self) -> Vec<f32> {
        let _ = self.stop_tx.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        std::mem::take(&mut *self.samples.lock())
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

fn pick_device(name: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if let Some(name) = name {
        if let Ok(mut devices) = host.input_devices() {
            if let Some(d) = devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
                return Ok(d);
            }
        }
        log::warn!("input device {name} not found, using default");
    }
    host.default_input_device().ok_or_else(|| anyhow!("nenhum microfone encontrado"))
}

fn build_stream(name: Option<&str>, samples: Arc<Mutex<Vec<f32>>>, level: Arc<AtomicU32>) -> Result<cpal::Stream> {
    let device = pick_device(name)?;
    let config = device.default_input_config().map_err(|e| anyhow!("microfone indisponível: {e}"))?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate().0;
    let format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();
    let err_fn = |e| log::error!("audio stream error: {e}");

    let mut resampler = Resampler::new(rate);
    let mut process = move |mono: &mut dyn Iterator<Item = f32>| {
        let mut out = Vec::with_capacity(1024);
        let mut sq = 0.0f32;
        let mut n = 0u32;
        for x in mono {
            sq += x * x;
            n += 1;
            resampler.push(x, &mut out);
        }
        if n > 0 {
            let rms = (sq / n as f32).sqrt();
            let lvl = (rms * 9.0).sqrt().min(1.0);
            level.store(lvl.to_bits(), Ordering::Relaxed);
        }
        samples.lock().extend_from_slice(&out);
    };

    macro_rules! stream_for {
        ($t:ty, $conv:expr) => {
            device.build_input_stream(
                &stream_config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    let mut it = data.chunks(channels).map(|frame| {
                        frame.iter().map(|&s| $conv(s)).sum::<f32>() / channels as f32
                    });
                    process(&mut it);
                },
                err_fn,
                None,
            )
        };
    }

    let stream = match format {
        cpal::SampleFormat::F32 => stream_for!(f32, |s: f32| s),
        cpal::SampleFormat::I16 => stream_for!(i16, |s: i16| s as f32 / 32768.0),
        cpal::SampleFormat::U16 => stream_for!(u16, |s: u16| (s as f32 - 32768.0) / 32768.0),
        cpal::SampleFormat::I32 => stream_for!(i32, |s: i32| s as f32 / 2_147_483_648.0),
        other => return Err(anyhow!("formato de áudio não suportado: {other:?}")),
    }
    .map_err(|e| anyhow!("não foi possível abrir o microfone: {e}"))?;
    Ok(stream)
}

/// 16-bit PCM WAV in memory, for cloud upload.
pub fn to_wav(samples: &[f32]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::with_capacity(samples.len() * 2 + 44));
    {
        let mut w = hound::WavWriter::new(&mut cursor, spec).expect("wav header");
        for &s in samples {
            let _ = w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
        }
        let _ = w.finalize();
    }
    cursor.into_inner()
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Trims leading/trailing near-silence (keeps 200 ms of padding). The threshold adapts to the
/// mic's noise floor so quiet microphones aren't mistaken for silence.
pub fn trim_silence(samples: &[f32]) -> &[f32] {
    let win = (SAMPLE_RATE / 50) as usize; // 20 ms
    let chunks: Vec<&[f32]> = samples.chunks(win).collect();
    if chunks.is_empty() {
        return &samples[0..0];
    }
    let mut levels: Vec<f32> = chunks.iter().map(|c| rms(c)).collect();
    let peak = levels.iter().cloned().fold(0.0, f32::max);
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let floor = levels[levels.len() / 10];
    let thresh = (floor * 2.5).max(0.0025);
    log::info!("audio levels: floor {floor:.4}, peak {peak:.4}, threshold {thresh:.4}");
    let loud = |chunk: &[f32]| rms(chunk) > thresh;
    let first = chunks.iter().position(|c| loud(c));
    let last = chunks.iter().rposition(|c| loud(c));
    match (first, last) {
        (Some(f), Some(l)) => {
            let pad = 10;
            let start = f.saturating_sub(pad) * win;
            let end = ((l + 1 + pad) * win).min(samples.len());
            &samples[start..end]
        }
        _ => &samples[0..0],
    }
}
