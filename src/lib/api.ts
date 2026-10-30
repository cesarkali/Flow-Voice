// Typed wrappers over the Rust commands. When the UI runs in a plain browser
// (vite dev without Tauri) every call falls back to an in-memory mock so the
// screens can be designed and checked without the native side.
import type {
  AppInfo,
  ChatMessage,
  DictationResult,
  KeyProviderId,
  KeyStatusMap,
  LlmProviderId,
  LocalModel,
  RecordMode,
  Settings,
  SettingsPage,
  TestResult,
  UpdateInfo,
} from "./types";
import { mock } from "./mock";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) return mock.invoke<T>(cmd, args);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

/** Subscribe to a Rust event. Returns an unsubscribe function. */
export async function on<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  if (!isTauri) return mock.on(event, handler as (p: unknown) => void);
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (e) => handler(e.payload));
}

export const api = {
  // settings
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  resetStyles: () => call<Settings>("reset_styles"),
  validateHotkey: (accelerator: string) => call<string | null>("validate_hotkey", { accelerator }),
  listInputDevices: () => call<string[]>("list_input_devices"),

  // keys & providers
  getKeyStatus: () => call<KeyStatusMap>("get_key_status"),
  setKeys: (provider: KeyProviderId, keys: string) => call<KeyStatusMap>("set_keys", { provider, keys }),
  revealKeys: (provider: KeyProviderId) => call<string>("reveal_keys", { provider }),
  testProvider: (provider: string, kind: "llm" | "stt" | "search") =>
    call<TestResult>("test_provider", { provider, kind }),
  listProviderModels: (provider: LlmProviderId) => call<string[]>("list_provider_models", { provider }),

  // local whisper models
  listLocalModels: () => call<LocalModel[]>("list_local_models"),
  downloadLocalModel: (id: string) => call<void>("download_local_model", { id }),
  deleteLocalModel: (id: string) => call<void>("delete_local_model", { id }),

  // recording / overlay
  startRecording: (mode: RecordMode) => call<void>("start_recording", { mode }),
  stopRecording: () => call<void>("stop_recording"),
  cancelRecording: () => call<void>("cancel_recording"),
  /** Re-run the last transcript with another style and/or translation target. */
  reprocess: (styleId: string, target: string | null) =>
    call<DictationResult>("reprocess", { styleId, target }),
  /** Paste into the app that was focused when recording started. replace = undo the previous paste first. */
  pasteText: (text: string, replace: boolean) => call<void>("paste_text", { text, replace }),
  copyText: (text: string) => call<void>("copy_text", { text }),
  /** Keep the overlay open while the pointer is over it. */
  overlayHold: (hold: boolean) => call<void>("overlay_hold", { hold }),
  /** Tell Rust how big the overlay content is so the native window can fit it. */
  overlayResize: (width: number, height: number) => call<void>("overlay_resize", { width, height }),
  dismissOverlay: () => call<void>("dismiss_overlay"),

  // history
  getHistory: () => call<DictationResult[]>("get_history"),
  deleteHistoryItem: (id: string) => call<void>("delete_history_item", { id }),
  clearHistory: () => call<void>("clear_history"),

  // chat (voice search assistant)
  getChat: () => call<ChatMessage[]>("get_chat"),
  chatSend: (text: string) => call<ChatMessage[]>("chat_send", { text }),
  chatToggleRecording: () => call<boolean>("chat_toggle_recording"),
  chatReset: () => call<void>("chat_reset"),

  // app
  openSettings: (page?: SettingsPage) => call<void>("open_settings", { page: page ?? null }),
  appInfo: () => call<AppInfo>("app_info"),
  checkUpdate: () => call<UpdateInfo>("check_update"),
  installUpdate: () => call<void>("install_update"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  quit: () => call<void>("quit_app"),
};
