// Shared contract between the Svelte UI and the Rust core (src-tauri/src/*).
// Field names are snake_case because they mirror the serde structs 1:1.

export type UiLanguage = "pt" | "en" | "es";
export type Theme = "system" | "dark" | "light";

export type SttProviderId = "groq" | "openai" | "gemini" | "mistral";
export type LlmProviderId = "groq" | "gemini" | "openai" | "openrouter" | "mistral" | "ollama" | "custom";
/** Everything that can hold API keys. */
export type KeyProviderId = Exclude<LlmProviderId, "ollama"> | "tavily";

export interface ProviderSettings {
  /** Chat model used for refining/translating/chat. */
  model: string;
  /** Speech-to-text model (only for providers in SttProviderId). */
  stt_model: string;
  /** Only used by ollama and custom. */
  base_url: string;
}

export type StyleKind = "llm" | "raw" | "verbatim";

export interface Style {
  id: string;
  name: string;
  description: string;
  /** llm = rewritten by the model; raw = no punctuation/line breaks, no LLM; verbatim = transcript as-is, no LLM */
  kind: StyleKind;
  prompt: string;
  builtin: boolean;
}

export interface Hotkeys {
  dictate: string;
  translate: string;
  ask: string;
}

export interface Settings {
  ui_language: UiLanguage;
  theme: Theme;
  /** ISO-639-1 code of what the user speaks, or "auto". */
  speech_language: string;
  /** null = system default microphone */
  input_device: string | null;
  stt: {
    engine: "cloud" | "local";
    /** Failover order among cloud STT providers that have keys. */
    cloud_order: SttProviderId[];
    /** Local whisper model id used when engine = local (and as offline fallback). */
    local_model: string;
    /** Show live text while speaking (uses preview_model locally). */
    live_preview: boolean;
    preview_model: string;
  };
  /** Failover order among LLM providers that are configured. */
  llm_order: LlmProviderId[];
  providers: Record<LlmProviderId, ProviderSettings>;
  active_style: string;
  styles: Style[];
  /** ISO-639-1 target for translation, e.g. "en". */
  translation_target: string;
  hotkeys: Hotkeys;
  /** toggle = press to start, press to stop; hold = push-to-talk */
  hotkey_mode: "toggle" | "hold";
  auto_paste: boolean;
  /** Strip "né", "tipo", "hã"... before sending to the LLM (saves tokens) and in no-LLM styles. */
  remove_fillers: boolean;
  mute_on_record: boolean;
  sounds: boolean;
  start_with_os: boolean;
  history_enabled: boolean;
  overlay_position: "bottom" | "top";
  /** Custom vocabulary: names/terms to bias transcription and the LLM. */
  dictionary: string[];
  onboarded: boolean;
}

export interface KeyStatus {
  /** Number of keys saved (comma separated list supports several). */
  count: number;
  /** Masked previews, e.g. "gsk_…9fA2". */
  masked: string[];
}

export type KeyStatusMap = Record<KeyProviderId, KeyStatus>;

export interface TestResult {
  ok: boolean;
  latency_ms: number;
  model: string;
  message: string;
}

export interface LocalModel {
  id: string;
  label: string;
  file: string;
  size_mb: number;
  installed: boolean;
  /** Short hint like "Prévia ao vivo" / "Equilíbrio" / "Máxima precisão". */
  note: string;
  recommended: boolean;
}

export interface ModelProgress {
  id: string;
  downloaded: number;
  total: number;
  done: boolean;
  error: string | null;
}

export type RecordMode = "dictate" | "translate" | "ask";

export interface DictationResult {
  id: string;
  ts: string; // ISO date
  mode: RecordMode;
  raw: string;
  text: string;
  style_id: string;
  target: string | null;
  /** LLM provider that produced `text` (null when no LLM was used). */
  provider: string | null;
  stt_provider: string;
  pasted: boolean;
  duration_ms: number;
  warnings: string[];
}

export type OverlayEvent =
  | { phase: "listening"; mode: RecordMode; style_id: string; target: string | null; live: boolean }
  | { phase: "transcribing" }
  | { phase: "refining"; provider: string }
  | { phase: "done"; result: DictationResult }
  | { phase: "answer"; query: string } // ask mode finished, chat window opened
  | { phase: "error"; message: string; detail: string | null; raw: string | null }
  | { phase: "hidden" };

export interface ChatSource {
  title: string;
  url: string;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  sources: ChatSource[];
}

export interface AppInfo {
  version: string;
  author: string;
  github: string;
  website: string;
  instagram: string;
  data_dir: string;
  os: "windows" | "linux" | "macos";
}

export interface UpdateInfo {
  available: boolean;
  version: string | null;
  notes: string | null;
}

export type SettingsPage = "home" | "styles" | "providers" | "speech" | "shortcuts" | "general" | "about";

/** Event names emitted by the Rust core. */
export const EVENTS = {
  overlayState: "overlay-state", // OverlayEvent
  overlayLevel: "overlay-level", // number 0..1 (~30/s while listening)
  overlayPartial: "overlay-partial", // { text: string } live transcript
  modelProgress: "model-progress", // ModelProgress
  updateProgress: "update-progress", // { downloaded: number; total: number | null }
  navigate: "navigate", // { page: SettingsPage }
  settingsChanged: "settings-changed", // Settings
  historyChanged: "history-changed", // void
  chatUpdated: "chat-updated", // ChatMessage[]
  chatTranscript: "chat-transcript", // { text: string }
  chatRecording: "chat-recording", // { recording: boolean }
} as const;
