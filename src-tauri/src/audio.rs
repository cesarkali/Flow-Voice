//! Microphone capture: any device format → 16 kHz mono f32 kept in memory.

use anyhow::{anyhow, Result};
use crate::i18n::{tr, trf};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

pub const SAMPLE_RATE: u32 = 16_000;
/// Longest recording: a toggle recording left running stops by itself here (and providers refuse
/// uploads over ~25 MB, ~13 min of 16 kHz 16-bit mono). Nothing past it is kept.
pub const MAX_RECORDING_SECS: u64 = 12 * 60;
const MAX_SAMPLES: usize = SAMPLE_RATE as usize * MAX_RECORDING_SECS as usize;

/// Appends captured audio, never past MAX_SAMPLES.
fn keep(samples: &Mutex<Vec<f32>>, block: &[f32]) {
    let mut all = samples.lock();
    let room = MAX_SAMPLES.saturating_sub(all.len());
    all.extend_from_slice(&block[..block.len().min(room)]);
}

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
    /// The chosen microphone was not found and the system default is recording instead.
    pub used_fallback: bool,
    /// The device failed mid-recording (unplugged, driver reset): the audio may be cut short.
    pub failed: Arc<AtomicBool>,
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
        let failed = Arc::new(AtomicBool::new(false));
        let f = failed.clone();
        // Test hook: FLOWVOICE_FAKE_MIC=<16 kHz mono wav> streams a file in real time instead of the mic.
        if let Ok(path) = std::env::var("FLOWVOICE_FAKE_MIC") {
            let thread = std::thread::spawn(move || fake_mic(&path, s, l, stop_rx));
            return Ok(Self { samples, used_fallback: false, failed, level, stop_tx, thread: Some(thread) });
        }
        // cpal streams are !Send on some platforms, so the stream lives on its own thread.
        let (fallback_tx, fallback_rx) = mpsc::channel::<bool>();
        let thread = std::thread::spawn(move || {
            let stream = match build_stream(device_name.as_deref(), s, l, f, fallback_tx) {
                Ok(st) => st,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            if let Err(e) = stream.play() {
                let _ = ready_tx.send(Err(mic_error("não foi possível iniciar o microfone: {}", &e)));
                return;
            }
            let _ = ready_tx.send(Ok(()));
            let _ = stop_rx.recv();
            drop(stream);
        });
        match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => {
                let used_fallback = fallback_rx.try_recv().unwrap_or(false);
                Ok(Self { samples, used_fallback, failed, level, stop_tx, thread: Some(thread) })
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!(tr("o microfone não respondeu"))),
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

fn fake_mic(path: &str, samples: Arc<Mutex<Vec<f32>>>, level: Arc<AtomicU32>, stop_rx: mpsc::Receiver<()>) {
    let data: Vec<f32> = match hound::WavReader::open(path) {
        Ok(mut r) => r.samples::<i16>().filter_map(Result::ok).map(|v| v as f32 / 32768.0).collect(),
        Err(e) => {
            log::error!("fake mic: {e}");
            vec![]
        }
    };
    let chunk = (SAMPLE_RATE / 50) as usize; // 20 ms
    let mut pos = 0;
    loop {
        if stop_rx.recv_timeout(std::time::Duration::from_millis(20)).is_ok() {
            return;
        }
        let end = (pos + chunk).min(data.len());
        let block: Vec<f32> = if pos < end { data[pos..end].to_vec() } else { vec![0.0; chunk] };
        pos = end;
        let lvl = (rms(&block) * 28.0).sqrt().min(1.0);
        level.store(lvl.to_bits(), Ordering::Relaxed);
        keep(&samples, &block);
    }
}

/// Windows renames a re-plugged USB mic "2- USB Audio": compare names without that prefix.
fn device_key(name: &str) -> String {
    let t = name.trim();
    let rest = t.trim_start_matches(|c: char| c.is_ascii_digit());
    let rest = if rest.len() < t.len() { rest.strip_prefix("- ").unwrap_or(t) } else { t };
    rest.to_lowercase()
}

/// The named microphone (exact, then ignoring Windows' "N- " prefix), else the default one.
/// The bool is true when the default had to be used instead of the chosen device.
fn pick_device(name: Option<&str>) -> Result<(cpal::Device, bool)> {
    let host = cpal::default_host();
    if let Some(name) = name {
        let devices: Vec<cpal::Device> = host.input_devices().map(|it| it.collect()).unwrap_or_default();
        let exact = devices.iter().position(|d| d.name().map(|n| n == name).unwrap_or(false));
        let fuzzy = || devices.iter().position(|d| d.name().map(|n| device_key(&n) == device_key(name)).unwrap_or(false));
        if let Some(i) = exact.or_else(fuzzy) {
            return Ok((devices.into_iter().nth(i).unwrap(), false));
        }
        log::warn!("input device {name} not found, using default");
        return host.default_input_device().map(|d| (d, true)).ok_or_else(|| anyhow!(tr("Nenhum microfone encontrado.")));
    }
    host.default_input_device().map(|d| (d, false)).ok_or_else(|| anyhow!(tr("Nenhum microfone encontrado.")))
}

/// What a microphone failure means for the user (None: nothing more specific than the error).
fn mic_problem(error: &str) -> Option<&'static str> {
    let e = error.to_lowercase();
    if e.contains("0x80070005") || e.contains("access is denied") || e.contains("acesso negado") {
        Some("O Windows bloqueou o microfone. Permita que apps da área de trabalho o usem em Configurações > Privacidade e segurança > Microfone.")
    } else if e.contains("0x8889000a") || e.contains("device_in_use") {
        Some("Outro app está usando o microfone com exclusividade.")
    } else {
        None
    }
}

fn mic_error(pt: &str, e: &dyn std::fmt::Display) -> anyhow::Error {
    let text = e.to_string();
    log::warn!("microphone: {text}");
    if mic_blocked_by_windows() {
        return anyhow!(tr("O Windows bloqueou o microfone. Permita que apps da área de trabalho o usem em Configurações > Privacidade e segurança > Microfone."));
    }
    match mic_problem(&text) {
        Some(problem) => anyhow!(tr(problem)),
        None => anyhow!(trf(pt, &[&text])),
    }
}

/// Settings > Privacy & security > Microphone is off for this account or for desktop apps.
#[cfg(windows)]
fn mic_blocked_by_windows() -> bool {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY, RRF_RT_REG_SZ};
    const STORE: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";
    let denied = |root: HKEY, path: String| {
        let mut buf = [0u16; 16];
        let mut len = (buf.len() * 2) as u32;
        let ok = unsafe {
            RegGetValueW(root, &HSTRING::from(path), &HSTRING::from("Value"), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut len))
        };
        ok.is_ok() && String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]) == "Deny"
    };
    denied(HKEY_LOCAL_MACHINE, STORE.to_string())
        || denied(HKEY_CURRENT_USER, STORE.to_string())
        || denied(HKEY_CURRENT_USER, format!(r"{STORE}\NonPackaged"))
}

#[cfg(not(windows))]
fn mic_blocked_by_windows() -> bool {
    false
}

fn build_stream(
    name: Option<&str>,
    samples: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    failed: Arc<AtomicBool>,
    fallback_tx: mpsc::Sender<bool>,
) -> Result<cpal::Stream> {
    let (device, fallback) = pick_device(name)?;
    let _ = fallback_tx.send(fallback);
    let config = device.default_input_config().map_err(|e| mic_error("microfone indisponível: {}", &e))?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate().0;
    let format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();
    let err_fn = move |e| {
        log::error!("audio stream error: {e}");
        failed.store(true, Ordering::SeqCst);
    };

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
            // Quiet mics peak around rms 0.03 while speaking: map that near the top.
            let lvl = (rms * 28.0).sqrt().min(1.0);
            level.store(lvl.to_bits(), Ordering::Relaxed);
        }
        keep(&samples, &out);
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
        other => return Err(anyhow!(trf("formato de áudio não suportado: {}", &[&format!("{other:?}")]))),
    }
    .map_err(|e| mic_error("não foi possível abrir o microfone: {}", &e))?;
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

/// Brings quiet microphones to a consistent level before transcription (speech recognizers do
/// noticeably worse on very quiet audio). Peak-based with a ceiling so noise isn't blown up.
pub fn normalize(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return vec![];
    }
    let mut mags: Vec<f32> = samples.iter().step_by(4).map(|x| x.abs()).collect();
    mags.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p = mags[((mags.len() as f32) * 0.995) as usize % mags.len()].max(1e-4);
    let gain = (0.6 / p).clamp(1.0, 12.0);
    samples.iter().map(|x| (x * gain).clamp(-0.98, 0.98)).collect()
}

/// 2nd-order Butterworth high-pass at 120 Hz, for measuring levels only (the audio sent to the
/// recognizer is untouched): a mic's DC offset and mains hum (50/60 Hz) are not speech, but they
/// raised the noise floor above the voice itself.
fn high_pass(samples: &[f32]) -> Vec<f32> {
    let w0 = 2.0 * std::f32::consts::PI * 120.0 / SAMPLE_RATE as f32;
    let (cos, alpha) = (w0.cos(), w0.sin() / (2.0 * std::f32::consts::FRAC_1_SQRT_2));
    let a0 = 1.0 + alpha;
    let (b0, b1, b2) = ((1.0 + cos) / 2.0 / a0, -(1.0 + cos) / a0, (1.0 + cos) / 2.0 / a0);
    let (a1, a2) = (-2.0 * cos / a0, (1.0 - alpha) / a0);
    // As if the signal had always been at its first value: a DC offset doesn't start with a
    // step, whose ringing would look like a loud first window.
    let first = samples.first().copied().unwrap_or(0.0);
    let (mut x1, mut x2, mut y1, mut y2) = (first, first, 0.0f32, 0.0f32);
    samples
        .iter()
        .map(|&x| {
            let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            y
        })
        .collect()
}

/// The level above which a 20 ms window counts as voice, from the recording's noise floor and
/// its loudest window. `floor * 2.5` keeps quiet microphones working; but on a noisy one (fan,
/// hum, gain all the way up) that sat above the voice itself (issue #2: floor 0.108, peak 0.154,
/// threshold 0.269, every dictation "no speech"). When the recording does rise above its floor,
/// the threshold then stays a third of the way up from the floor to the peak. A flat recording
/// (steady noise only: its windows vary by a few percent) stays silence.
fn voice_threshold(floor: f32, peak: f32) -> f32 {
    let mut thresh = floor * 2.5;
    if peak > floor * 1.3 {
        thresh = thresh.min(floor + (peak - floor) * 0.35);
    }
    thresh.max(0.0025)
}

/// Trims leading/trailing near-silence (keeps 200 ms of padding). The threshold adapts to the
/// mic's noise floor so quiet microphones aren't mistaken for silence.
pub fn trim_silence(samples: &[f32]) -> &[f32] {
    trim(samples, true)
}

/// `trim_silence` without the log line: the live preview measures its window many times a second.
pub fn trim_silence_quiet(samples: &[f32]) -> &[f32] {
    trim(samples, false)
}

fn trim(samples: &[f32], log: bool) -> &[f32] {
    let win = (SAMPLE_RATE / 50) as usize; // 20 ms
    let filtered = high_pass(samples);
    let chunks: Vec<&[f32]> = filtered.chunks(win).collect();
    if chunks.is_empty() {
        return &samples[0..0];
    }
    let chunk_levels: Vec<f32> = chunks.iter().map(|c| rms(c)).collect();
    let peak = chunk_levels.iter().cloned().fold(0.0, f32::max);
    let mut levels = chunk_levels.clone();
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let floor = levels[levels.len() / 10];
    let thresh = voice_threshold(floor, peak);
    if log {
        log::info!("audio levels: floor {floor:.4}, peak {peak:.4}, threshold {thresh:.4}");
    }
    let first = chunk_levels.iter().position(|&l| l > thresh);
    let last = chunk_levels.iter().rposition(|&l| l > thresh);
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

#[cfg(test)]
mod tests {
    #[test]
    fn device_key_ignores_windows_prefix() {
        assert_eq!(super::device_key("2- USB Audio Device"), "usb audio device");
        assert_eq!(super::device_key("USB Audio Device"), "usb audio device");
        assert_eq!(super::device_key("3D Mic"), "3d mic");
    }

    #[test]
    fn recording_stops_growing_at_the_limit() {
        let samples = super::Mutex::new(vec![0.0f32; super::MAX_SAMPLES - 10]);
        super::keep(&samples, &[1.0; 25]);
        assert_eq!(samples.lock().len(), super::MAX_SAMPLES);
        super::keep(&samples, &[1.0; 5]);
        assert_eq!(samples.lock().len(), super::MAX_SAMPLES);
    }
}

#[cfg(test)]
mod trim_tests {
    use super::{trim_silence, SAMPLE_RATE};

    const SR: usize = SAMPLE_RATE as usize;

    /// Uniform white noise with this RMS (fixed seed: same signal every run).
    fn noise(len: usize, rms: f32) -> Vec<f32> {
        let amp = rms * 3f32.sqrt();
        let mut seed = 0x2545_f491u32;
        (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0) * amp
            })
            .collect()
    }

    /// A 300 Hz "voice" of this RMS added between `from` and `to` seconds.
    fn with_voice(mut s: Vec<f32>, from: f32, to: f32, rms: f32) -> Vec<f32> {
        let amp = rms * std::f32::consts::SQRT_2;
        for i in (from * SR as f32) as usize..(to * SR as f32) as usize {
            s[i] += amp * (2.0 * std::f32::consts::PI * 300.0 * i as f32 / SR as f32).sin();
        }
        s
    }

    #[test]
    fn silence_is_silence() {
        assert!(trim_silence(&vec![0.0; 3 * SR]).is_empty());
        assert!(trim_silence(&noise(3 * SR, 0.0005)).is_empty());
    }

    #[test]
    fn quiet_mic_keeps_the_voice() {
        let s = with_voice(noise(3 * SR, 0.002), 1.0, 2.0, 0.02);
        let kept = trim_silence(&s).len() as f32 / SR as f32;
        assert!((1.0..=1.5).contains(&kept), "kept {kept} s");
    }

    #[test]
    fn noisy_mic_keeps_the_voice() {
        // Issue #2: a floor of 0.1 and a voice that only brings it to ~0.15.
        let s = with_voice(noise(3 * SR, 0.1), 1.0, 2.0, 0.112);
        let kept = trim_silence(&s).len() as f32 / SR as f32;
        assert!((1.0..=1.5).contains(&kept), "kept {kept} s");
    }

    #[test]
    fn steady_noise_alone_is_silence() {
        assert!(trim_silence(&noise(3 * SR, 0.1)).is_empty());
    }

    #[test]
    fn dc_offset_and_hum_are_not_a_floor() {
        let mut s = with_voice(noise(3 * SR, 0.002), 1.0, 2.0, 0.02);
        for (i, x) in s.iter_mut().enumerate() {
            *x += 0.2 + 0.08 * (2.0 * std::f32::consts::PI * 50.0 * i as f32 / SR as f32).sin();
        }
        let kept = trim_silence(&s).len() as f32 / SR as f32;
        assert!((1.0..=1.5).contains(&kept), "kept {kept} s");
    }
}

#[cfg(test)]
mod mic_tests {
    use super::mic_problem;

    #[test]
    fn windows_errors_become_clear_messages() {
        assert!(mic_problem("Acesso negado. (0x80070005)").unwrap().contains("bloqueou"));
        assert!(mic_problem("Access is denied.").unwrap().contains("bloqueou"));
        assert!(mic_problem("Device in use (0x8889000A)").unwrap().contains("exclusividade"));
        assert!(mic_problem("some other failure").is_none());
    }
}
