// Shared app state for the settings window. Edits to `app.settings` are
// auto-saved (debounced) — components just mutate the object.
//
// Save pipeline (hardened against the "onboarding reverts itself" bug):
// - saves are queued, so two flushes never overlap and Rust always receives them in order;
// - JSON comparisons use a canonical, key-sorted form (Rust serializes in struct order,
//   JS keeps insertion order — a plain JSON.stringify compare made identical settings look different);
// - `settings-changed` events are ignored while a save is pending/in flight, and late echoes of
//   anything this window saved recently are dropped (they carry stale state, e.g. onboarded:false);
// - once onboarding is completed in this window it can never be reverted by an event.
import { api, on } from "./api";
import { applyAccent, applyTheme } from "./theme";
import { setLanguage } from "./i18n.svelte";
import type { KeyProviderId, KeyStatusMap, LlmProviderId, Settings } from "./types";
import { EVENTS } from "./types";

/** Emitted by the tray item "Refazer configuração inicial" (no payload). */
export const RESTART_ONBOARDING = "restart-onboarding";

export const app = $state({
  settings: null as Settings | null,
  keys: null as KeyStatusMap | null,
  save: "idle" as "idle" | "saving" | "saved" | "error",
  saveError: "",
});

/** Canonical JSON (object keys sorted recursively, arrays kept in order). */
export function canon(v: unknown): string {
  return JSON.stringify(v, (_k, val) => {
    if (val && typeof val === "object" && !Array.isArray(val)) {
      const out: Record<string, unknown> = {};
      for (const k of Object.keys(val).sort()) out[k] = (val as Record<string, unknown>)[k];
      return out;
    }
    return val;
  });
}

/** Canonical JSON of what the backend is known to hold. */
let lastJson = "";
let timer = 0;
let timerPending = false;
let savedTimer = 0;
let busy = 0; // queued + in-flight saves
let queue: Promise<unknown> = Promise.resolve();
/** Settings this window sent/received from its own saves, with the time; used to drop late echoes. */
const recent = new Map<string, number>();
const ECHO_WINDOW_MS = 8000;
/** Set once onboarding was completed here; no event may bring the wizard back. */
let onboardedLocally = false;

function remember(json: string) {
  const now = Date.now();
  recent.set(json, now);
  for (const [k, ts] of recent) if (now - ts > ECHO_WINDOW_MS) recent.delete(k);
}

function adopt(s: Settings) {
  lastJson = canon(s);
  if (onboardedLocally && !s.onboarded) {
    // Keep the wizard closed; the autosave effect will persist `true` again.
    s.onboarded = true;
  }
  app.settings = s;
  applyTheme(s.theme);
  applyAccent(s.accent_color);
  setLanguage(s.ui_language);
}

/** Runs `fn` after every previously queued save finished. */
function enqueue<T>(fn: () => Promise<T>): Promise<T> {
  busy++;
  const run = queue.then(fn, fn);
  queue = run.catch(() => {}).finally(() => busy--);
  return run;
}

async function doFlush(): Promise<boolean> {
  if (!app.settings) return true;
  const snap = $state.snapshot(app.settings) as Settings;
  const json = canon(snap);
  if (json === lastJson) return true;
  remember(json);
  app.save = "saving";
  try {
    const saved = await api.saveSettings(snap);
    const savedJson = canon(saved);
    remember(savedJson);
    lastJson = savedJson;
    // Adopt the normalized copy only if the user didn't keep editing meanwhile.
    if (app.settings && canon($state.snapshot(app.settings)) === json && savedJson !== json) adopt(saved);
    app.save = "saved";
    clearTimeout(savedTimer);
    savedTimer = window.setTimeout(() => (app.save = "idle"), 1600);
    return true;
  } catch (e) {
    app.save = "error";
    app.saveError = String(e);
    lastJson = "";
    return false;
  }
}

function flush(): Promise<boolean> {
  timerPending = false;
  clearTimeout(timer);
  return enqueue(doFlush);
}

/** Call once per window. Loads settings + keys and starts the autosave effect. */
export async function initApp() {
  const [s, k] = await Promise.all([api.getSettings(), api.getKeyStatus()]);
  app.keys = k;
  adopt(s);

  $effect.root(() => {
    $effect(() => {
      if (!app.settings) return;
      const json = canon($state.snapshot(app.settings));
      if (json === lastJson) return;
      // live side effects
      applyTheme(app.settings.theme);
      applyAccent(app.settings.accent_color);
      setLanguage(app.settings.ui_language);
      clearTimeout(timer);
      timerPending = true;
      timer = window.setTimeout(flush, 400);
    });
  });

  on<Settings>(EVENTS.settingsChanged, (s) => {
    if (!s) return;
    const json = canon(s);
    if (json === lastJson) return;
    // Our own edits win while they are waiting to be saved or being saved.
    if (timerPending || busy > 0) return;
    // A late echo of something this window saved earlier carries stale state.
    if (recent.has(json)) return;
    adopt(s);
  });
  // Tray "Refazer configuração inicial": Rust already saved onboarded:false; reopen the wizard here.
  // Saving false again is harmless and wins over any `true` that was queued/in flight before the event.
  on(RESTART_ONBOARDING, () => void restartOnboarding());
  // Flush pending edits when the window is hidden/closed.
  addEventListener("pagehide", () => void flush());
}

/** Saves pending edits now. Resolves false if the save failed (app.saveError has the reason). */
export function saveNow(): Promise<boolean> {
  return flush();
}

/**
 * Finishes onboarding: persists `onboarded: true` (after any queued save) and only then flips
 * the local flag, so App's gate switches immediately and nothing can revert it.
 * Throws with the backend error if the save fails (the wizard stays open to show it).
 */
export async function completeOnboarding(): Promise<void> {
  if (!app.settings) return;
  clearTimeout(timer);
  timerPending = false;
  const ok = await enqueue(async () => {
    const snap = { ...($state.snapshot(app.settings!) as Settings), onboarded: true };
    const json = canon(snap);
    remember(json);
    app.save = "saving";
    try {
      const saved = await api.saveSettings(snap);
      remember(canon(saved));
      onboardedLocally = true;
      adopt({ ...saved, onboarded: true });
      app.save = "saved";
      clearTimeout(savedTimer);
      savedTimer = window.setTimeout(() => (app.save = "idle"), 1600);
      return "";
    } catch (e) {
      app.save = "error";
      app.saveError = String(e);
      return String(e);
    }
  });
  if (ok) throw new Error(ok);
}

/**
 * Reopens the setup wizard (keys and settings are kept; the wizard starts from the current values).
 * Clears the "completed here" guard and saves `onboarded: false`.
 */
export async function restartOnboarding(): Promise<boolean> {
  if (!app.settings) return false;
  onboardedLocally = false;
  app.settings.onboarded = false;
  return flush();
}

export async function refreshKeys() {
  app.keys = await api.getKeyStatus();
}

export async function setKeys(provider: KeyProviderId, keys: string) {
  app.keys = await api.setKeys(provider, keys);
}

export function hasKey(p: KeyProviderId | LlmProviderId): boolean {
  if (p === "ollama") return true;
  return (app.keys?.[p as KeyProviderId]?.count ?? 0) > 0;
}

/** LLM providers that are usable right now (have a key, or are local and set up). */
export function usableLlm(): LlmProviderId[] {
  const s = app.settings;
  if (!s) return [];
  return s.llm_order.filter((p, i) => {
    // Ollama has no key to tell us it's set up; treat it as active only when the user put it first.
    if (p === "ollama") return i === 0 && !!s.providers.ollama.base_url && !!s.providers.ollama.model;
    if (p === "custom") return !!s.providers.custom.base_url && !!s.providers.custom.model;
    return hasKey(p);
  });
}
