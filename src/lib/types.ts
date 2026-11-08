// Shared contract between the Svelte UI and the Rust core (src-tauri/src/*).
// Field names are snake_case because they mirror the serde structs 1:1.

export type UiLanguage = "pt" | "en" | "es" | "fr" | "de" | "ja" | "zh" | "ru" | "hi" | "bn";
/** The setting: a language, or "auto" (the one Windows is in). */
export type UiLanguagePref = UiLanguage | "auto";
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
  /** Always "" for built-in styles (their prompts are internal and not editable). */
  prompt: string;
  builtin: boolean;
}

export interface Hotkeys {
  dictate: string;
  translate: string;
  ask: string;
  /** Edit the selected text by voice ("Comando"). */
  command: string;
  /** Fix spelling and punctuation of the selected text, no recording ("Revisar"). */
  proofread: string;
  /** Paste the last dictation again into the focused app ("" = off). */
  paste_last: string;
}

/** "Estilos por aplicativo": dictations into `app` (normalized id, see lib/apps.ts) use `style_id`. */
export interface AppStyle {
  app: string;
  style_id: string;
}

export interface Settings {
  ui_language: UiLanguagePref;
  theme: Theme;
  /** Hex accent color, e.g. "#D7FF3F" (default phosphor lime). */
  accent_color: string;
  /** Optional name the app uses to greet the user ("" = none). Never added to dictations. */
  user_name: string;
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
  /** Intensity of the built-in "prompt" style. */
  prompt_strength: "light" | "medium" | "strong";
  styles: Style[];
  /** Per-app rules: a match overrides active_style for that dictation only. */
  app_styles: AppStyle[];
  app_styles_enabled: boolean;
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
  overlay_position: "bottom" | "top" | "custom";
  /** Card center as a fraction (0..1) of the monitor's work area, used when overlay_position = "custom". */
  overlay_custom: OverlayPoint | null;
  /** Drag the overlay during a recording. */
  overlay_drag: boolean;
  /** Result card width; compact also shrinks text a little. */
  overlay_width: "compact" | "normal" | "wide";
  /** true = overlay appears on the monitor under the mouse; false = always on the primary monitor. */
  overlay_follow_mouse: boolean;
  overlay_glass: "solid" | "glass" | "translucent";
  overlay_glass_light: boolean;
  window_glass: boolean;
  /** Custom vocabulary: names/terms to bias transcription and the LLM. */
  dictionary: string[];
  onboarded: boolean;
  /** Version whose "what's new" was seen and closed ("" = before 2.2). */
  seen_version: string;
  /** Internal migration counter (round-trip it unchanged). */
  schema?: number;
}

export interface KeyStatus {
  /** Number of keys saved (comma separated list supports several). */
  count: number;
  /** Masked previews, e.g. "gsk_…9fA2". */
  masked: string[];
}

export type KeyStatusMap = Record<KeyProviderId, KeyStatus>;

/** A retired provider model that FlowVoice replaced by itself (heal.rs). */
export interface ModelNotice {
  provider: string;
  kind: "stt" | "chat";
  old: string;
  new: string;
  message: string;
  ts: string;
}

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
  /** General default (onboarding). */
  recommended: boolean;
  /** Approximate peak RAM while transcribing. */
  ram_mb: number;
  /** Languages it can transcribe; "*" = every Whisper language. */
  langs: string[];
  /** Languages it is the best pick for; "*" = every language it supports. */
  recommended_for: string[];
  /** 1 (slowest) … 5 (fastest). */
  speed: number;
  /** 1 (most mistakes) … 5 (most accurate). */
  quality: number;
  /** "whisper small · q5_1" */
  tech: string;
}

export interface ModelProgress {
  id: string;
  downloaded: number;
  total: number;
  done: boolean;
  error: string | null;
  /** Stopped by the user (cancel_local_model_download); `error` is then "cancelado". */
  cancelled?: boolean;
}

/** command = apply a spoken instruction to the text selected in the target app. */
export type RecordMode = "dictate" | "translate" | "ask" | "command";

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
  /** Normalized id of the app the text went to ("outlook"); absent in older history. */
  app?: string | null;
  /** Friendly name of that app ("Outlook"). */
  app_name?: string | null;
  /** Command mode: the selected text the instruction (`raw`) was applied to. */
  source?: string | null;
  /** The transcription failed (`error` says why): no text, the recording kept in memory to try again. */
  failed?: boolean;
  error?: string | null;
  /** Failed or cancelled items: "ready" = the recording is still kept, "running" = being transcribed, absent = gone. */
  retry?: "ready" | "running" | null;
  /** Stopped with Esc while recording: `text` is what the live preview showed (may be empty). */
  cancelled?: boolean;
}

export type OverlayEvent =
  | {
      phase: "listening";
      mode: RecordMode;
      style_id: string;
      target: string | null;
      live: boolean;
      /** Friendly name of the focused app, when known. */
      app?: string | null;
      /** Command mode: "pending" = being copied (then `overlay-selection`), "at_stop" = copied on release. */
      selection?: "pending" | "at_stop" | null;
    }
  | { phase: "transcribing" }
  | { phase: "refining"; provider: string; mode?: RecordMode | "proofread" }
  | { phase: "done"; result: DictationResult }
  | { phase: "answer"; query: string } // ask mode finished, chat window opened
  /** hint = the user can fix it (nothing selected…): no "open settings" action. */
  /** retry = id of the kept recording ("Tentar de novo"); key = every provider refused the key. */
  /** cancelled = a recording stopped with Esc: raw = its live preview, retry = "Transcrever". */
  | { phase: "error"; message: string; detail: string | null; raw: string | null; hint?: boolean; retry?: string | null; key?: boolean; cancelled?: boolean }
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

/** A spot on a screen as fractions 0..1 of its work area. */
export interface OverlayPoint {
  x: number;
  y: number;
}

export type SettingsPage = "home" | "styles" | "providers" | "speech" | "shortcuts" | "general" | "about";

/** Event names emitted by the Rust core. */
export const EVENTS = {
  overlayState: "overlay-state", // OverlayEvent
  overlayLevel: "overlay-level", // number 0..1 (~30/s while listening)
  overlayPointer: "overlay-pointer", // { inside: boolean } cursor over the overlay hitbox (use instead of DOM hover)
  overlayAnchor: "overlay-anchor", // { dx, dy, grow: "up" | "down" } where the card sits inside the (fixed) window
  overlayMove: "overlay-move", // { phase: "out" | "in" } changing monitors
  overlayDrag: "overlay-drag", // { active: boolean } the card is being dragged (Rust follows the real cursor)
  overlayPartial: "overlay-partial", // { text: string; stable: number } live transcript (stable = confirmed chars)
  overlaySelection: "overlay-selection", // { text: string } Command mode: preview of the copied selection
  overlayGlassOff: "overlay-glass-off", // void: Windows' blur could not be shown, draw the card solid
  modelProgress: "model-progress", // ModelProgress
  updateProgress: "update-progress", // { downloaded: number; total: number | null }
  navigate: "navigate", // { page: SettingsPage }
  settingsChanged: "settings-changed", // Settings
  historyChanged: "history-changed", // void
  chatUpdated: "chat-updated", // ChatMessage[]
  chatTranscript: "chat-transcript", // { text: string }
  chatRecording: "chat-recording", // { recording: boolean }
  modelHealed: "model-healed", // ModelNotice: a retired provider model was replaced automatically
} as const;
