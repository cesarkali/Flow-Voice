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
  ModelNotice,
  OverlayPoint,
  RecordMode,
  Settings,
  SettingsPage,
  TestResult,
  UpdateInfo,
} from "./types";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) return (await import("./mock")).mock.invoke<T>(cmd, args);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

/** Subscribe to a Rust event. Returns an unsubscribe function. */
export async function on<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  if (!isTauri) return (await import("./mock")).mock.on(event, handler as (p: unknown) => void);
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (e) => handler(e.payload));
}

export const api = {
  // settings
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  resetStyles: () => call<Settings>("reset_styles"),
  altgrBlocksCommand: () => call<boolean>("altgr_blocks_command"),
  altgrBlocksProofread: () => call<boolean>("altgr_blocks_proofread"),
  validateHotkey: (accelerator: string) => call<string | null>("validate_hotkey", { accelerator }),
  listInputDevices: () => call<string[]>("list_input_devices"),
  /** Pause global shortcuts while the user records a new one. */
  suspendHotkeys: (suspend: boolean) => call<void>("suspend_hotkeys", { suspend }),

  // keys & providers
  getKeyStatus: () => call<KeyStatusMap>("get_key_status"),
  setKeys: (provider: KeyProviderId, keys: string) => call<KeyStatusMap>("set_keys", { provider, keys }),
  revealKeys: (provider: KeyProviderId) => call<string>("reveal_keys", { provider }),
  /** `key`: test this key (the one just pasted) instead of the first saved one. */
  testProvider: (provider: string, kind: "llm" | "stt" | "search", key?: string) =>
    call<TestResult>("test_provider", { provider, kind, key: key ?? null }),
  listProviderModels: (provider: LlmProviderId) => call<string[]>("list_provider_models", { provider }),

  // local whisper models
  listLocalModels: () => call<LocalModel[]>("list_local_models"),
  downloadLocalModel: (id: string) => call<void>("download_local_model", { id }),
  /** Stops a running download (no-op if none); Rust emits model-progress { done: true, cancelled: true }. */
  cancelLocalModelDownload: (id: string) => call<void>("cancel_local_model_download", { id }),
  deleteLocalModel: (id: string) => call<void>("delete_local_model", { id }),

  // recording / overlay
  startRecording: (mode: RecordMode) => call<void>("start_recording", { mode }),
  stopRecording: () => call<void>("stop_recording"),
  cancelRecording: () => call<void>("cancel_recording"),
  /** Re-run the last transcript with another style and/or translation target. */
  reprocess: (styleId: string, target: string | null) =>
    call<DictationResult>("reprocess", { styleId, target }),
  /** History: redo any saved dictation from its original transcript with another style/language
   *  (the item is updated in place and the result is copied). */
  /** Command card: rewrite the selected text in a style (no spoken instruction needed). */
  commandStyle: (styleId: string, target: string | null) => call<void>("command_style", { styleId, target }),
  reprocessHistory: (id: string, styleId: string, target: string | null) =>
    call<DictationResult>("reprocess_history", { id, styleId, target }),
  /** Paste into the app that was focused when recording started. replace = undo the previous paste
   *  first (only while the field is as that paste left it). False: only copied (Rust shows a card). */
  pasteText: (text: string, replace: boolean) => call<boolean>("paste_text", { text, replace }),
  copyText: (text: string) => call<void>("copy_text", { text }),
  /** Keep the overlay open while the pointer is over it. */
  overlayHold: (hold: boolean) => call<void>("overlay_hold", { hold }),
  /** @deprecated The overlay window has a fixed size now; use overlayHitbox. Kept as a no-op. */
  overlayResize: (width: number, height: number) => call<void>("overlay_resize", { width, height }),
  /**
   * The overlay window is fixed-size and transparent; only this rectangle (CSS px, relative to the
   * window) catches the mouse — everywhere else clicks go through to the apps below.
   * Send null when nothing is visible. Rust emits `overlay-pointer` { inside } as the cursor enters/leaves it.
   */
  overlayHitbox: (rect: { x: number; y: number; width: number; height: number } | null) =>
    call<void>("overlay_hitbox", { rect }),
  /** Windows' blur under the card ("Vidro"); null fades it out. */
  overlayGlass: (rect: { x: number; y: number; width: number; height: number; radius: number; opacity: number } | null) =>
    call<void>("overlay_glass", { rect }),
  glassAvailable: () => call<boolean>("glass_available"),
  /** Card offset inside the fixed overlay window (see the `overlay-anchor` event). */
  getOverlayAnchor: () => call<{ dx: number; dy: number; grow: "up" | "down" } | null>("get_overlay_anchor"),
  /** Shows the capsule ~2 s at the position being edited ("Testar"). */
  testOverlayPosition: (position: string, point: OverlayPoint | null) => call<void>("test_overlay_position", { position, point }),
  /** Where the user last dropped the overlay while recording (card center fraction), if ever. */
  getLastOverlayDrag: () => call<OverlayPoint | null>("get_last_overlay_drag"),
  /** A press landed on the card's body while recording: Rust follows the real cursor from here
   *  (emits `overlay-drag` { active } when it becomes / stops being a drag). False = not allowed now. */
  overlayDragArm: () => call<boolean>("overlay_drag_arm"),
  dismissOverlay: () => call<void>("dismiss_overlay"),

  // history
  getHistory: () => call<DictationResult[]>("get_history"),
  deleteHistoryItem: (id: string) => call<void>("delete_history_item", { id }),
  clearHistory: () => call<void>("clear_history"),
  /** "Tentar de novo" on a failed transcription. onlyCopy: from the history (copy, don't paste). */
  retryFailed: (id: string, onlyCopy: boolean) => call<void>("retry_failed", { id, onlyCopy }),

  // chat (voice search assistant)
  getChat: () => call<ChatMessage[]>("get_chat"),
  chatSend: (text: string) => call<ChatMessage[]>("chat_send", { text }),
  chatToggleRecording: () => call<boolean>("chat_toggle_recording"),
  chatReset: () => call<void>("chat_reset"),

  // app
  openSettings: (page?: SettingsPage) => call<void>("open_settings", { page: page ?? null }),
  appInfo: () => call<AppInfo>("app_info"),
  whatsNewSeen: () => call<void>("whats_new_seen"),
  /** GitHub issue form ("bug" | "idea") prefilled with app/Windows versions (+ an error message). */
  openFeedback: (kind: "bug" | "idea", context?: string, includeLog = true) =>
    call<void>("open_feedback", { kind, context: context ?? null, includeLog }),
  /** The cleaned flowvoice.log excerpt a bug report would include (to show before sending). */
  feedbackLog: () => call<string>("feedback_log"),
  checkUpdate: () => call<UpdateInfo>("check_update"),
  installUpdate: () => call<void>("install_update"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  quit: () => call<void>("quit_app"),
  /** Author photo as a data URL (GitHub avatar, cached for offline), or null. */
  authorAvatar: () => call<string | null>("author_avatar"),
  /** Automatic model replacements not yet shown (also pushed live as EVENTS.modelHealed). */
  takeModelNotices: () => call<ModelNotice[]>("take_model_notices"),
};
