// In-memory stand-in for the Rust core, used when the UI runs in a plain
// browser (`npm run dev` without Tauri). Realistic enough to design against.
import type {
  AppInfo,
  ChatMessage,
  DictationResult,
  KeyProviderId,
  KeyStatusMap,
  LlmProviderId,
  LocalModel,
  OverlayEvent,
  RecordMode,
  Settings,
  Style,
  TestResult,
  UpdateInfo,
} from "./types";
import { EVENTS } from "./types";

type Handler = (payload: unknown) => void;
const handlers = new Map<string, Set<Handler>>();

export function mockEmit(event: string, payload?: unknown) {
  handlers.get(event)?.forEach((h) => {
    try {
      h(payload);
    } catch (e) {
      console.error(e);
    }
  });
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));

// ---------------------------------------------------------------- defaults

export const DEFAULT_STYLES: Style[] = [
  {
    id: "natural",
    name: "Natural",
    description: "Corrige pontuação e gramática mantendo o seu jeito de falar.",
    kind: "llm",
    prompt:
      "Reescreva a transcrição corrigindo pontuação, gramática e repetições, sem mudar o sentido nem o tom. Mantenha o idioma original. Responda só com o texto final.",
    builtin: true,
  },
  {
    id: "formal",
    name: "Profissional",
    description: "Linguagem formal e clara para e-mails e documentos.",
    kind: "llm",
    prompt:
      "Reescreva a transcrição em linguagem profissional, clara e educada, adequada para e-mail de trabalho. Não invente informações. Responda só com o texto final.",
    builtin: true,
  },
  {
    id: "casual",
    name: "Mensagem",
    description: "Curto e leve, do jeito que se escreve no chat.",
    kind: "llm",
    prompt:
      "Reescreva a transcrição como uma mensagem curta e descontraída de chat, sem formalidades. Não use emojis. Responda só com o texto final.",
    builtin: true,
  },
  {
    id: "raw",
    name: "Bruto",
    description: "Sem IA: texto corrido, sem pontuação nem quebras.",
    kind: "raw",
    prompt: "",
    builtin: true,
  },
  {
    id: "verbatim",
    name: "Literal",
    description: "Sem IA: exatamente o que o reconhecimento de voz entendeu.",
    kind: "verbatim",
    prompt: "",
    builtin: true,
  },
];

const DEFAULT_SETTINGS: Settings = {
  ui_language: "pt",
  theme: "system",
  speech_language: "pt",
  input_device: null,
  stt: {
    engine: "cloud",
    cloud_order: ["groq", "gemini", "mistral", "openai"],
    local_model: "base",
    live_preview: true,
    preview_model: "tiny",
  },
  llm_order: ["groq", "gemini", "openrouter", "mistral", "openai", "ollama", "custom"],
  providers: {
    groq: { model: "openai/gpt-oss-20b", stt_model: "whisper-large-v3-turbo", base_url: "" },
    gemini: { model: "gemini-3.1-flash-lite", stt_model: "gemini-3.1-flash-lite", base_url: "" },
    openai: { model: "gpt-6-luna", stt_model: "gpt-transcribe", base_url: "" },
    openrouter: { model: "google/gemma-4-31b-it:free", stt_model: "", base_url: "" },
    mistral: { model: "mistral-small-latest", stt_model: "voxtral-mini-latest", base_url: "" },
    ollama: { model: "gemma3:4b", stt_model: "", base_url: "http://localhost:11434/v1" },
    custom: { model: "", stt_model: "", base_url: "" },
  },
  active_style: "natural",
  styles: clone(DEFAULT_STYLES),
  translation_target: "en",
  hotkeys: { dictate: "CommandOrControl+Shift+Space", translate: "CommandOrControl+Shift+T", ask: "Alt+Space" },
  hotkey_mode: "toggle",
  auto_paste: true,
  remove_fillers: true,
  mute_on_record: false,
  sounds: true,
  start_with_os: true,
  history_enabled: true,
  overlay_position: "bottom",
  dictionary: ["FlowVoice", "Caliberda", "Tauri", "Svelte"],
  onboarded: true,
};

const params = typeof location !== "undefined" ? new URLSearchParams(location.search) : new URLSearchParams();

let settings: Settings = clone(DEFAULT_SETTINGS);
if (params.has("onboarding")) settings.onboarded = false;
if (params.get("theme") === "light" || params.get("theme") === "dark") settings.theme = params.get("theme") as Settings["theme"];

const keys: Record<KeyProviderId, string[]> = {
  groq: ["gsk_8Kd2mQpL0aZr7TnV1xYc4bW9fA2"],
  gemini: [],
  openai: [],
  openrouter: ["sk-or-v1-3f9a0c7e21b84d5a9e6c"],
  mistral: [],
  custom: [],
  tavily: [],
};
if (params.has("onboarding")) keys.groq = [];

const mask = (k: string) => (k.length <= 8 ? "••••" : `${k.slice(0, k.indexOf("_") > 0 && k.indexOf("_") < 7 ? k.indexOf("_") + 1 : 4)}…${k.slice(-4)}`);

function keyStatus(): KeyStatusMap {
  const out = {} as KeyStatusMap;
  (Object.keys(keys) as KeyProviderId[]).forEach((p) => (out[p] = { count: keys[p].length, masked: keys[p].map(mask) }));
  return out;
}

const localModels: LocalModel[] = [
  { id: "tiny", label: "Whisper Tiny", file: "ggml-tiny.bin", size_mb: 75, installed: true, note: "Prévia ao vivo", recommended: false },
  { id: "base", label: "Whisper Base", file: "ggml-base.bin", size_mb: 142, installed: false, note: "Rápido em qualquer PC", recommended: false },
  { id: "small", label: "Whisper Small", file: "ggml-small.bin", size_mb: 466, installed: false, note: "Equilíbrio", recommended: true },
  { id: "large-v3-turbo-q5", label: "Whisper Large v3 Turbo (q5)", file: "ggml-large-v3-turbo-q5_0.bin", size_mb: 574, installed: false, note: "Máxima precisão", recommended: false },
];

// ---------------------------------------------------------------- sample texts

const RAW =
  "então é eu queria saber se a gente consegue marcar aquela reunião pra quinta-feira tipo de tarde porque na quarta eu não consigo né";
const BY_STYLE: Record<string, string> = {
  natural: "Eu queria saber se a gente consegue marcar aquela reunião para quinta-feira à tarde, porque na quarta eu não consigo.",
  formal:
    "Gostaria de saber se seria possível agendar a reunião para quinta-feira à tarde, pois na quarta-feira não terei disponibilidade.",
  casual: "Dá pra marcar aquela reunião na quinta à tarde? Na quarta não rola pra mim.",
  raw: "eu queria saber se a gente consegue marcar aquela reunião pra quinta-feira de tarde porque na quarta eu não consigo",
  verbatim: RAW,
};
const BY_LANG: Record<string, string> = {
  en: "I wanted to know if we could schedule that meeting for Thursday afternoon, since I can't make it on Wednesday.",
  es: "Quería saber si podemos agendar esa reunión para el jueves por la tarde, porque el miércoles no puedo.",
  pt: BY_STYLE.natural,
  fr: "Je voulais savoir si on pouvait fixer cette réunion jeudi après-midi, car mercredi je ne peux pas.",
  de: "Ich wollte fragen, ob wir das Meeting auf Donnerstagnachmittag legen können, weil ich am Mittwoch nicht kann.",
  it: "Volevo sapere se possiamo fissare quella riunione giovedì pomeriggio, perché mercoledì non riesco.",
  ja: "水曜日は都合が悪いので、その会議を木曜日の午後に設定できるか知りたいです。",
  zh: "我想知道能不能把那个会议安排在周四下午，因为周三我没空。",
};

function textFor(styleId: string, target: string | null) {
  if (target) return BY_LANG[target] ?? BY_LANG.en;
  return BY_STYLE[styleId] ?? `${BY_STYLE.natural}`;
}

let lastResult: DictationResult | null = null;

function makeResult(mode: RecordMode, styleId: string, target: string | null, pasted = true): DictationResult {
  const style = settings.styles.find((s) => s.id === styleId);
  const llm = style?.kind === "llm" || !!target;
  return {
    id: crypto.randomUUID(),
    ts: new Date().toISOString(),
    mode,
    raw: RAW,
    text: textFor(styleId, target),
    style_id: styleId,
    target,
    provider: llm ? "groq · openai/gpt-oss-20b" : null,
    stt_provider: settings.stt.engine === "local" ? `local · ${settings.stt.local_model}` : "groq · whisper-large-v3-turbo",
    pasted,
    duration_ms: 5400,
    warnings: [],
  };
}

const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();
let history: DictationResult[] = [
  {
    ...makeResult("dictate", "formal", null),
    id: "h1",
    ts: minutesAgo(4),
  },
  {
    id: "h2",
    ts: minutesAgo(38),
    mode: "translate",
    raw: "me manda o relatório até sexta que eu reviso no fim de semana",
    text: "Send me the report by Friday and I'll review it over the weekend.",
    style_id: "natural",
    target: "en",
    provider: "groq · openai/gpt-oss-20b",
    stt_provider: "groq · whisper-large-v3-turbo",
    pasted: true,
    duration_ms: 3900,
    warnings: [],
  },
  {
    id: "h3",
    ts: minutesAgo(60 * 3),
    mode: "dictate",
    raw: "tipo assim lembrar de comprar café pão e aquela pilha AA pro controle",
    text: "Lembrar de comprar café, pão e pilha AA para o controle.",
    style_id: "natural",
    target: null,
    provider: "gemini · gemini-3.1-flash-lite",
    stt_provider: "groq · whisper-large-v3-turbo",
    pasted: false,
    duration_ms: 4100,
    warnings: ["Groq atingiu o limite; usado Gemini como reserva."],
  },
  {
    id: "h4",
    ts: minutesAgo(60 * 26),
    mode: "dictate",
    raw: "ok fechado a gente se fala amanhã",
    text: "ok fechado a gente se fala amanhã",
    style_id: "raw",
    target: null,
    provider: null,
    stt_provider: "local · small",
    pasted: true,
    duration_ms: 1800,
    warnings: [],
  },
];
if (params.has("empty")) history = [];

let chat: ChatMessage[] = [
  { role: "user", content: "Qual a previsão do tempo pra São Paulo amanhã?", sources: [] },
  {
    role: "assistant",
    content:
      "Amanhã em **São Paulo** a previsão é de tempo **parcialmente nublado**, com chance de garoa no fim da tarde.\n\n- Mínima: 16 °C\n- Máxima: 24 °C\n- Chuva: 40% a partir das 17h\n\nSe for sair à noite, vale levar um casaco leve. Detalhes em [Climatempo](https://www.climatempo.com.br).",
    sources: [
      { title: "Climatempo — São Paulo", url: "https://www.climatempo.com.br/previsao-do-tempo/cidade/558/saopaulo-sp" },
      { title: "INMET — Previsão", url: "https://portal.inmet.gov.br" },
    ],
  },
];

// ---------------------------------------------------------------- overlay simulation

let simTimers: number[] = [];
let levelTimer = 0;
let stopRequested: (() => void) | null = null;

function clearSim() {
  simTimers.forEach((t) => clearTimeout(t));
  simTimers = [];
  clearInterval(levelTimer);
  stopRequested = null;
}

const later = (ms: number, fn: () => void) => simTimers.push(window.setTimeout(fn, ms));

const LIVE_WORDS =
  "Eu queria saber se a gente consegue marcar aquela reunião pra quinta-feira de tarde porque na quarta eu não consigo e também preciso revisar a apresentação antes".split(
    " ",
  );

export type SimScenario = "dictate" | "live" | "translate" | "ask" | "error" | "done" | "transcribing" | "refining";

/** Plays the overlay flow by emitting the same events Rust would. */
export function simulateOverlay(scenario: SimScenario = "live", autoStop = true) {
  clearSim();
  const emit = (e: OverlayEvent) => mockEmit(EVENTS.overlayState, e);
  const mode: RecordMode = scenario === "translate" ? "translate" : scenario === "ask" ? "ask" : "dictate";
  const target = mode === "translate" ? settings.translation_target : null;

  const finish = () => {
    clearInterval(levelTimer);
    stopRequested = null;
    emit({ phase: "transcribing" });
    later(900, () => {
      if (mode === "ask") {
        emit({ phase: "answer", query: "Qual a previsão do tempo pra amanhã?" });
        return;
      }
      if (scenario === "error") {
        emit({
          phase: "error",
          message: "Não foi possível refinar o texto.",
          detail:
            'groq: HTTP 429 Too Many Requests — {"error":{"message":"Rate limit reached for model `openai/gpt-oss-20b` in organization on tokens per minute (TPM): Limit 8000, Used 7920, Requested 412. Please try again in 2.4s.","type":"tokens","code":"rate_limit_exceeded"}}\ngemini: sem chave configurada',
          raw: RAW,
        });
        return;
      }
      emit({ phase: "refining", provider: "Groq" });
      later(1300, () => {
        lastResult = makeResult(mode, settings.active_style, target);
        history.unshift(lastResult);
        mockEmit(EVENTS.historyChanged);
        emit({ phase: "done", result: lastResult });
      });
    });
  };

  if (scenario === "transcribing") return emit({ phase: "transcribing" });
  if (scenario === "refining") return emit({ phase: "refining", provider: "Groq" });
  if (scenario === "done") {
    lastResult = makeResult("dictate", settings.active_style, null);
    lastResult.warnings = ["Groq lento; resposta via Gemini."];
    return emit({ phase: "done", result: lastResult });
  }

  const live = scenario !== "dictate";
  emit({ phase: "listening", mode, style_id: settings.active_style, target, live });
  let t = 0;
  let words = 0;
  levelTimer = window.setInterval(() => {
    t += 33;
    const speaking = Math.sin(t / 900) > -0.55;
    const lvl = speaking ? Math.min(1, 0.25 + Math.abs(Math.sin(t / 130)) * 0.45 + Math.random() * 0.3) : Math.random() * 0.06;
    mockEmit(EVENTS.overlayLevel, lvl);
    if (live && t % 330 < 33 && words < LIVE_WORDS.length) {
      words++;
      mockEmit(EVENTS.overlayPartial, { text: LIVE_WORDS.slice(0, words).join(" ") });
    }
  }, 33);
  stopRequested = finish;
  if (autoStop) later(scenario === "ask" ? 2600 : 7000, finish);
}

export function stopSimulation() {
  clearSim();
  mockEmit(EVENTS.overlayState, { phase: "hidden" } satisfies OverlayEvent);
}

// ---------------------------------------------------------------- command router

let chatRecording = false;

async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  await sleep(cmd.startsWith("get_") ? 30 : 60);
  const r = await route(cmd, args);
  return r as T;
}

async function route(cmd: string, a: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "get_settings":
      return clone(settings);
    case "save_settings":
      settings = clone(a.settings as Settings);
      return clone(settings);
    case "reset_styles":
      settings.styles = clone(DEFAULT_STYLES);
      if (!settings.styles.some((s) => s.id === settings.active_style)) settings.active_style = "natural";
      mockEmit(EVENTS.settingsChanged, clone(settings));
      return clone(settings);
    case "validate_hotkey": {
      const acc = String(a.accelerator);
      const others = Object.values(settings.hotkeys);
      if (acc === "Alt+F4" || acc === "CommandOrControl+C" || acc === "CommandOrControl+V")
        return "Esse atalho é reservado pelo sistema.";
      if (others.filter((o) => o === acc).length > 1) return "Esse atalho já está em uso por outra ação.";
      return null;
    }
    case "list_input_devices":
      return ["Microfone (Realtek High Definition Audio)", "Headset (HyperX Cloud II)", "Microfone USB (Blue Yeti)"];

    case "get_key_status":
      return keyStatus();
    case "set_keys": {
      const p = a.provider as KeyProviderId;
      keys[p] = String(a.keys)
        .split(/[,\n]/)
        .map((s) => s.trim())
        .filter(Boolean);
      return keyStatus();
    }
    case "reveal_keys":
      return keys[a.provider as KeyProviderId].join(", ");
    case "test_provider": {
      await sleep(700 + Math.random() * 600);
      const p = String(a.provider);
      const kind = a.kind as string;
      const has = p === "ollama" || (keys[p as KeyProviderId]?.length ?? 0) > 0;
      if (!has)
        return {
          ok: false,
          latency_ms: 0,
          model: "",
          message: "Nenhuma chave salva para este provedor.",
        } satisfies TestResult;
      if (p === "openrouter" && kind === "llm")
        return {
          ok: false,
          latency_ms: 812,
          model: settings.providers.openrouter.model,
          message:
            'HTTP 429: {"error":{"message":"Rate limit exceeded: free-models-per-day. Add 10 credits to unlock 1000 free model requests per day","code":429}}',
        } satisfies TestResult;
      const prov = settings.providers[p as LlmProviderId];
      return {
        ok: true,
        latency_ms: Math.round(180 + Math.random() * 500),
        model: kind === "stt" ? prov?.stt_model : kind === "search" ? "tavily-search" : prov?.model,
        message: kind === "stt" ? "Transcreveu o áudio de teste." : "Respondeu corretamente.",
      } satisfies TestResult;
    }
    case "list_provider_models":
      await sleep(500);
      if (a.provider === "ollama") return ["gemma3:4b", "gemma3:12b", "llama3.2:3b", "qwen3:8b"];
      return [
        "google/gemma-4-31b-it:free",
        "google/gemma-4-26b-a4b-it:free",
        "openrouter/free",
        "meta-llama/llama-4-scout:free",
        "deepseek/deepseek-chat-v3.1:free",
        "qwen/qwen3-235b-a22b:free",
      ];

    case "list_local_models":
      return clone(localModels);
    case "download_local_model": {
      const m = localModels.find((x) => x.id === a.id)!;
      const total = m.size_mb * 1024 * 1024;
      let done = 0;
      const step = () => {
        done = Math.min(total, done + total / 40);
        const finished = done >= total;
        if (finished) m.installed = true;
        mockEmit(EVENTS.modelProgress, { id: m.id, downloaded: done, total, done: finished, error: null });
        if (!finished) setTimeout(step, 90);
      };
      setTimeout(step, 90);
      return;
    }
    case "delete_local_model": {
      const m = localModels.find((x) => x.id === a.id);
      if (m) m.installed = false;
      return;
    }

    case "start_recording": {
      const mode = a.mode as RecordMode;
      simulateOverlay(mode === "translate" ? "translate" : mode === "ask" ? "ask" : settings.stt.live_preview ? "live" : "dictate");
      return;
    }
    case "stop_recording":
      stopRequested?.();
      return;
    case "cancel_recording":
      stopSimulation();
      return;
    case "reprocess": {
      await sleep(900);
      const target = (a.target as string | null) ?? null;
      const prev = lastResult ?? makeResult("dictate", "natural", null);
      lastResult = { ...makeResult(prev.mode, String(a.styleId), target, false), id: prev.id };
      return clone(lastResult);
    }
    case "paste_text":
      return;
    case "copy_text":
      try {
        await navigator.clipboard.writeText(String(a.text));
      } catch {
        /* ignore */
      }
      return;
    case "overlay_hold":
    case "overlay_resize":
      return;
    case "dismiss_overlay":
      stopSimulation();
      return;

    case "get_history":
      return settings.history_enabled ? clone(history) : [];
    case "delete_history_item":
      history = history.filter((h) => h.id !== a.id);
      return;
    case "clear_history":
      history = [];
      return;

    case "get_chat":
      return clone(chat);
    case "chat_send": {
      const text = String(a.text);
      chat.push({ role: "user", content: text, sources: [] });
      mockEmit(EVENTS.chatUpdated, clone(chat));
      await sleep(1600);
      chat.push({
        role: "assistant",
        content: `Pesquisei sobre **${text.replace(/[?.!]+$/, "")}**. Resumo do que encontrei:\n\n1. O assunto aparece em fontes recentes com informações consistentes.\n2. Os detalhes variam conforme a região — confira a fonte principal.\n\nUse \`Ctrl+C\` para copiar esta resposta ou peça para eu aprofundar algum ponto.`,
        sources: [
          { title: "Wikipedia", url: "https://pt.wikipedia.org" },
          { title: "G1", url: "https://g1.globo.com" },
          { title: "Folha de S.Paulo", url: "https://www.folha.uol.com.br" },
        ],
      });
      mockEmit(EVENTS.chatUpdated, clone(chat));
      return clone(chat);
    }
    case "chat_toggle_recording":
      chatRecording = !chatRecording;
      mockEmit(EVENTS.chatRecording, { recording: chatRecording });
      if (chatRecording)
        setTimeout(() => {
          if (!chatRecording) return;
          chatRecording = false;
          mockEmit(EVENTS.chatRecording, { recording: false });
          mockEmit(EVENTS.chatTranscript, { text: "E no fim de semana, vai chover?" });
        }, 2500);
      return chatRecording;
    case "chat_reset":
      chat = [];
      mockEmit(EVENTS.chatUpdated, []);
      return;

    case "open_settings":
      mockEmit(EVENTS.navigate, { page: a.page ?? "home" });
      return;
    case "app_info":
      return {
        version: "2.0.0",
        author: "Júlio Caliberda",
        github: "https://github.com/cesarkali/Flow-Voice",
        website: "https://caliberda.com.br",
        instagram: "https://www.instagram.com/cesar.kali/",
        data_dir: "C:\\Users\\julio\\AppData\\Roaming\\br.com.caliberda.flowvoice",
        os: "windows",
      } satisfies AppInfo;
    case "check_update":
      await sleep(900);
      return { available: true, version: "2.0.1", notes: "Correções no colar automático e nova voz de teste." } satisfies UpdateInfo;
    case "install_update": {
      const total = 9_400_000;
      let d = 0;
      const tick = () => {
        d = Math.min(total, d + 600_000);
        mockEmit(EVENTS.updateProgress, { downloaded: d, total });
        if (d < total) setTimeout(tick, 120);
      };
      tick();
      return;
    }
    case "open_url":
      window.open(String(a.url), "_blank", "noopener");
      return;
    case "quit_app":
      return;
  }
  console.warn("[mock] unknown command", cmd, a);
  return null;
}

export const mock = {
  invoke,
  on(event: string, handler: Handler): () => void {
    if (!handlers.has(event)) handlers.set(event, new Set());
    handlers.get(event)!.add(handler);
    return () => handlers.get(event)?.delete(handler);
  },
};
