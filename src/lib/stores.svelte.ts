// Shared app state for the settings window. Edits to `app.settings` are
// auto-saved (debounced) — components just mutate the object.
import { api, on } from "./api";
import { applyTheme } from "./theme";
import { setLanguage } from "./i18n.svelte";
import type { KeyProviderId, KeyStatusMap, LlmProviderId, Settings } from "./types";
import { EVENTS } from "./types";

export const app = $state({
  settings: null as Settings | null,
  keys: null as KeyStatusMap | null,
  save: "idle" as "idle" | "saving" | "saved" | "error",
  saveError: "",
});

let lastJson = "";
let timer = 0;
let savedTimer = 0;

function adopt(s: Settings) {
  lastJson = JSON.stringify(s);
  app.settings = s;
  applyTheme(s.theme);
  setLanguage(s.ui_language);
}

async function flush() {
  if (!app.settings) return;
  const snap = $state.snapshot(app.settings) as Settings;
  const json = JSON.stringify(snap);
  if (json === lastJson) return;
  lastJson = json;
  app.save = "saving";
  try {
    const saved = await api.saveSettings(snap);
    // Adopt the normalized copy only if the user didn't keep editing meanwhile.
    if (JSON.stringify($state.snapshot(app.settings)) === json) {
      lastJson = JSON.stringify(saved);
      if (lastJson !== json) app.settings = saved;
    }
    app.save = "saved";
    clearTimeout(savedTimer);
    savedTimer = window.setTimeout(() => (app.save = "idle"), 1600);
  } catch (e) {
    app.save = "error";
    app.saveError = String(e);
    lastJson = "";
  }
}

/** Call once per window. Loads settings + keys and starts the autosave effect. */
export async function initApp() {
  const [s, k] = await Promise.all([api.getSettings(), api.getKeyStatus()]);
  app.keys = k;
  adopt(s);

  $effect.root(() => {
    $effect(() => {
      if (!app.settings) return;
      const json = JSON.stringify($state.snapshot(app.settings));
      if (json === lastJson) return;
      // live side effects
      applyTheme(app.settings.theme);
      setLanguage(app.settings.ui_language);
      clearTimeout(timer);
      timer = window.setTimeout(flush, 400);
    });
  });

  on<Settings>(EVENTS.settingsChanged, (s) => {
    if (JSON.stringify(s) !== lastJson) adopt(s);
  });
  // Flush pending edits when the window is hidden/closed.
  addEventListener("pagehide", () => flush());
}

export function saveNow() {
  clearTimeout(timer);
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
