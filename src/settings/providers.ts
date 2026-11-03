// Static provider catalog: links, tutorials (as i18n keys), defaults and model suggestions.
import { api } from "../lib/api";
import { t, type TKey } from "../lib/i18n.svelte";
import { app, setKeys } from "../lib/stores.svelte";
import type { KeyProviderId, LlmProviderId, SttProviderId } from "../lib/types";

export type ProviderId = LlmProviderId | "tavily";

export interface ProviderInfo {
  id: ProviderId;
  name: string;
  /** i18n key of the name when it needs translating */
  nameKey?: TKey;
  badges: { key: TKey; tone: "accent" | "neutral" | "solid" }[];
  desc: TKey;
  url: string;
  steps: TKey[];
  /** command block shown after a given step (ollama) */
  command?: { afterStep: number; text: string };
  note?: TKey;
  keyed: boolean;
  keyPrefix?: string;
  stt: boolean;
  llm: boolean;
  baseUrl: boolean;
  canListModels: boolean;
  modelSuggestions: string[];
  sttSuggestions: string[];
}

export const PROVIDERS: ProviderInfo[] = [
  {
    id: "groq",
    name: "Groq",
    badges: [
      { key: "prov.badge_recommended", tone: "solid" },
      { key: "prov.badge_free", tone: "accent" },
    ],
    desc: "prov.groq.desc",
    url: "https://console.groq.com/keys",
    steps: ["prov.groq.s1", "prov.groq.s2", "prov.groq.s3", "prov.groq.s4"],
    keyed: true,
    keyPrefix: "gsk_…",
    stt: true,
    llm: true,
    baseUrl: false,
    canListModels: false,
    modelSuggestions: ["openai/gpt-oss-20b", "openai/gpt-oss-120b"],
    sttSuggestions: ["whisper-large-v3-turbo", "whisper-large-v3"],
  },
  {
    id: "gemini",
    name: "Google Gemini",
    badges: [{ key: "prov.badge_free", tone: "accent" }],
    desc: "prov.gemini.desc",
    url: "https://aistudio.google.com/apikey",
    steps: ["prov.gemini.s1", "prov.gemini.s2", "prov.gemini.s3", "prov.gemini.s4"],
    note: "prov.gemini.note",
    keyed: true,
    keyPrefix: "AIza…",
    stt: true,
    llm: true,
    baseUrl: false,
    canListModels: false,
    modelSuggestions: ["gemini-3.1-flash-lite", "gemini-3.8-flash", "gemini-3.5-flash-lite"],
    sttSuggestions: ["gemini-3.1-flash-lite", "gemini-3.8-flash", "gemini-3.5-flash-lite"],
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    badges: [{ key: "prov.badge_free_models", tone: "accent" }],
    desc: "prov.openrouter.desc",
    url: "https://openrouter.ai/settings/keys",
    steps: ["prov.openrouter.s1", "prov.openrouter.s2", "prov.openrouter.s3"],
    note: "prov.openrouter.note",
    keyed: true,
    keyPrefix: "sk-or-…",
    stt: false,
    llm: true,
    baseUrl: false,
    canListModels: true,
    modelSuggestions: ["google/gemma-4-31b-it:free", "openrouter/free", "google/gemma-4-26b-a4b-it:free"],
    sttSuggestions: [],
  },
  {
    id: "mistral",
    name: "Mistral",
    badges: [{ key: "prov.badge_no_card", tone: "accent" }],
    desc: "prov.mistral.desc",
    url: "https://console.mistral.ai",
    steps: ["prov.mistral.s1", "prov.mistral.s2", "prov.mistral.s3", "prov.mistral.s4"],
    keyed: true,
    stt: true,
    llm: true,
    baseUrl: false,
    canListModels: false,
    modelSuggestions: ["mistral-small-latest", "mistral-medium-latest"],
    sttSuggestions: ["voxtral-mini-latest"],
  },
  {
    id: "openai",
    name: "OpenAI",
    badges: [{ key: "prov.badge_paid", tone: "neutral" }],
    desc: "prov.openai.desc",
    url: "https://platform.openai.com/api-keys",
    steps: ["prov.openai.s1", "prov.openai.s2", "prov.openai.s3", "prov.openai.s4"],
    keyed: true,
    keyPrefix: "sk-…",
    stt: true,
    llm: true,
    baseUrl: false,
    canListModels: false,
    modelSuggestions: ["gpt-6-luna"],
    sttSuggestions: ["gpt-transcribe"],
  },
  {
    id: "ollama",
    name: "Ollama",
    badges: [{ key: "prov.badge_local", tone: "neutral" }],
    desc: "prov.ollama.desc",
    url: "https://ollama.com/download",
    steps: ["prov.ollama.s1", "prov.ollama.s2", "prov.ollama.s3"],
    command: { afterStep: 1, text: "ollama pull gemma3:4b" },
    keyed: false,
    stt: false,
    llm: true,
    baseUrl: true,
    canListModels: true,
    modelSuggestions: ["gemma3:4b", "gemma3:12b", "llama3.2:3b", "qwen3:8b"],
    sttSuggestions: [],
  },
  {
    id: "custom",
    name: "Custom",
    nameKey: "prov.custom.name",
    badges: [],
    desc: "prov.custom.desc",
    url: "",
    steps: ["prov.custom.s1", "prov.custom.s2", "prov.custom.s3"],
    keyed: true,
    stt: false,
    llm: true,
    baseUrl: true,
    canListModels: false,
    modelSuggestions: [],
    sttSuggestions: [],
  },
  {
    id: "tavily",
    name: "Tavily",
    badges: [{ key: "prov.badge_search", tone: "accent" }],
    desc: "prov.tavily.desc",
    url: "https://app.tavily.com",
    steps: ["prov.tavily.s1", "prov.tavily.s2", "prov.tavily.s3"],
    keyed: true,
    keyPrefix: "tvly-…",
    stt: false,
    llm: false,
    baseUrl: false,
    canListModels: false,
    modelSuggestions: [],
    sttSuggestions: [],
  },
];

export const providerInfo = (id: string) => PROVIDERS.find((p) => p.id === id)!;

/** Display name of a provider (translated when needed). Unknown ids are returned as-is. */
export function pname(id: string): string {
  const p = PROVIDERS.find((x) => x.id === id);
  if (!p) return id;
  return p.nameKey ? t(p.nameKey) : p.name;
}

export const STT_PROVIDERS: SttProviderId[] = ["groq", "gemini", "mistral", "openai"];

export const isKeyProvider = (id: string): id is KeyProviderId => id !== "ollama";

// ---------------------------------------------------------------- multiple keys
// Rust stores a provider's keys as one comma-joined list and accepts comma, semicolon or
// newline separators. The UI edits them as rows; these helpers keep both views in sync.

/** Splits a pasted/stored list of keys (comma, semicolon or newline separated). */
export function splitKeys(raw: string): string[] {
  return raw
    .split(/[,;\n\r]+/)
    .map((k) => k.trim())
    .filter(Boolean);
}

/** True when the text holds more than one key. */
export const isKeyList = (raw: string) => splitKeys(raw).length > 1;

/** Full keys currently saved for a provider (in failover order). */
export async function savedKeys(provider: KeyProviderId): Promise<string[]> {
  if ((app.keys?.[provider]?.count ?? 0) === 0) return [];
  return splitKeys(await api.revealKeys(provider));
}

/** Appends keys (deduplicated) after the saved ones. Returns the new ones. */
export async function addKeys(provider: KeyProviderId, raw: string): Promise<string[]> {
  const current = await savedKeys(provider);
  const fresh = splitKeys(raw).filter((k, i, all) => !current.includes(k) && all.indexOf(k) === i);
  if (fresh.length) await setKeys(provider, [...current, ...fresh].join(","));
  return fresh;
}

/** Removes the key at `index` (same order as KeyStatus.masked). */
export async function removeKeyAt(provider: KeyProviderId, index: number): Promise<void> {
  const current = await savedKeys(provider);
  current.splice(index, 1);
  await setKeys(provider, current.join(","));
}
