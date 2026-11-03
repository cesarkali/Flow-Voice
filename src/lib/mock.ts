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
  // Mirrors builtin_styles() in src-tauri/src/settings.rs (ids, names, descriptions).
  { id: "natural", name: "Natural", description: "Corrige e limpa a fala mantendo seu jeito de falar.", kind: "llm", prompt: "", builtin: true },
  { id: "professional", name: "Profissional", description: "Texto formal e organizado, pronto para e-mail e trabalho.", kind: "llm", prompt: "", builtin: true },
  { id: "prompt", name: "Prompt", description: "Enxuga a fala para mandar a uma IA: menos tokens, mesmo contexto.", kind: "llm", prompt: "", builtin: true },
  { id: "raw", name: "Cru", description: "Só as palavras: sem pontuação, sem quebras de linha, sem IA.", kind: "raw", prompt: "", builtin: true },
  { id: "verbatim", name: "Só transcrição", description: "A transcrição exatamente como saiu, sem passar por IA.", kind: "verbatim", prompt: "", builtin: true },
];

const DEFAULT_SETTINGS: Settings = {
  ui_language: "pt",
  theme: "system",
  accent_color: "#D7FF3F",
  user_name: "",
  speech_language: "pt",
  input_device: null,
  stt: {
    engine: "cloud",
    cloud_order: ["groq", "gemini", "mistral", "openai"],
    local_model: "base-q5_1",
    live_preview: true,
    preview_model: "base-q5_1",
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
  prompt_strength: "medium",
  styles: [
    ...clone(DEFAULT_STYLES),
    {
      id: "custom-topicos",
      name: "Tópicos",
      description: "Transforma a fala em uma lista curta de tópicos.",
      kind: "llm",
      prompt: "Transforme o que eu disser em tópicos curtos, um por linha, começando com hífen.",
      builtin: false,
    },
  ],
  app_styles: [
    { app: "outlook", style_id: "professional" },
    { app: "slack", style_id: "natural" },
    { app: "code", style_id: "prompt" },
  ],
  app_styles_enabled: true,
  translation_target: "en",
  hotkeys: {
    dictate: "CommandOrControl+Shift+Space",
    translate: "CommandOrControl+Shift+Y",
    ask: "CommandOrControl+Shift+U",
    command: "CommandOrControl+Alt+K",
    paste_last: "CommandOrControl+Alt+V",
  },
  hotkey_mode: "toggle",
  auto_paste: true,
  remove_fillers: true,
  mute_on_record: false,
  sounds: true,
  start_with_os: true,
  history_enabled: true,
  overlay_position: "bottom",
  overlay_custom: null,
  overlay_drag: true,
  overlay_width: "normal",
  overlay_follow_mouse: true,
  overlay_glass: "solid",
  overlay_glass_light: false,
  window_glass: false,
  dictionary: ["FlowVoice", "Caliberda", "Tauri", "Svelte"],
  onboarded: true,
};

const params = typeof location !== "undefined" ? new URLSearchParams(location.search) : new URLSearchParams();

let settings: Settings = clone(DEFAULT_SETTINGS);
if (params.has("onboarding")) settings.onboarded = false;
if (params.get("theme") === "light" || params.get("theme") === "dark") settings.theme = params.get("theme") as Settings["theme"];
// ?accent=FF2E00: preview with a custom accent color · ?style=<id>: active style
if (params.get("accent")) settings.accent_color = "#" + params.get("accent")!.replace(/^#/, "");
if (params.get("style")) settings.active_style = params.get("style")!;
// ?norules: no per-app styles yet (empty state + suggestions)
if (params.has("norules")) settings.app_styles = [];

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

const ALL = ["*"];
const SMALL_OK = ["en", "es", "pt", "it", "de", "fr", "ca", "nl", "ja", "ru", "pl", "ko", "id", "sv", "tr", "gl", "uk", "zh", "ms", "no", "vi", "fi", "cs", "ro"];
const lm = (m: Omit<LocalModel, "installed" | "note" | "recommended"> & Partial<LocalModel>): LocalModel => ({ installed: false, note: "", recommended: false, ...m });
const localModels: LocalModel[] = [
  lm({ id: "tiny-q5_1", label: "Tiny", file: "ggml-tiny-q5_1.bin", size_mb: 31, ram_mb: 150, langs: ALL, recommended_for: [], speed: 5, quality: 1, tech: "whisper tiny · q5_1" }),
  lm({ id: "base-q5_1", label: "Base", file: "ggml-base-q5_1.bin", size_mb: 57, ram_mb: 300, langs: ALL, recommended_for: [], speed: 5, quality: 2, tech: "whisper base · q5_1", installed: true, recommended: true }),
  lm({ id: "base-en-q5_1", label: "Base (English)", file: "ggml-base.en-q5_1.bin", size_mb: 57, ram_mb: 300, langs: ["en"], recommended_for: ["en"], speed: 5, quality: 3, tech: "whisper base.en · q5_1" }),
  lm({ id: "small-q5_1", label: "Small", file: "ggml-small-q5_1.bin", size_mb: 181, ram_mb: 590, langs: ALL, recommended_for: SMALL_OK, speed: 4, quality: 3, tech: "whisper small · q5_1" }),
  lm({ id: "small-en-q5_1", label: "Small (English)", file: "ggml-small.en-q5_1.bin", size_mb: 181, ram_mb: 590, langs: ["en"], recommended_for: ["en"], speed: 4, quality: 4, tech: "whisper small.en · q5_1" }),
  lm({ id: "distil-ptbr-q5_0", label: "Distil Large v3 PT-BR", file: "ggml-distil-large-v3-ptbr-q5_0.bin", size_mb: 513, ram_mb: 950, langs: ["pt"], recommended_for: ["pt"], speed: 2, quality: 5, tech: "distil-whisper large-v3 pt-BR (freds0) · q5_0", installed: true }),
  lm({ id: "turbo-q5_0", label: "Large v3 Turbo", file: "ggml-large-v3-turbo-q5_0.bin", size_mb: 547, ram_mb: 1020, langs: ALL, recommended_for: ALL, speed: 2, quality: 5, tech: "whisper large-v3-turbo · q5_0" }),
  lm({ id: "medium-q5_0", label: "Medium", file: "ggml-medium-q5_0.bin", size_mb: 514, ram_mb: 1200, langs: ALL, recommended_for: [], speed: 1, quality: 4, tech: "whisper medium · q5_0" }),
];
// ?nomodels: nothing downloaded yet (onboarding live-preview / offline download flows).
if (params.has("nomodels")) localModels.forEach((m) => (m.installed = false));
/** Simulated downloads in flight (id -> next tick), so cancel_local_model_download can stop them. */
const mockDownloads = new Map<string, ReturnType<typeof setTimeout>>();

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
    app: "outlook",
    app_name: "Outlook",
  };
}

const CMD_SOURCE =
  "oi pessoal, segue o relatório do trimestre. os números ficaram abaixo do esperado mas a gente tem um plano pra recuperar no próximo";
const CMD_RAW = "deixa mais formal e organiza em tópicos";
const CMD_TEXT =
  "Prezados,\n\nSegue o relatório do trimestre:\n- Os resultados ficaram abaixo do esperado.\n- Já temos um plano de recuperação para o próximo período.";
const CMD_TEXT_EN =
  "Dear all,\n\nPlease find the quarterly report below:\n- Results came in below expectations.\n- We already have a recovery plan for the next period.";
function makeCommand(target: string | null = null): DictationResult {
  return {
    ...makeResult("command", settings.active_style, null),
    raw: CMD_RAW,
    text: target ? CMD_TEXT_EN : CMD_TEXT,
    target,
    source: CMD_SOURCE,
    app: "gmail",
    app_name: "Gmail",
    duration_ms: 2600,
  };
}

const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();
let history: DictationResult[] = [
  {
    ...makeResult("dictate", "professional", null),
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
// More varied samples for designing the history list and the home stats.
const H = (id: string, m: number, o: Partial<DictationResult>): DictationResult => ({
  id,
  ts: minutesAgo(m),
  mode: "dictate",
  raw: "",
  text: "",
  style_id: "natural",
  target: null,
  provider: "groq · openai/gpt-oss-20b",
  stt_provider: "groq · whisper-large-v3-turbo",
  pasted: true,
  duration_ms: 4000,
  warnings: [],
  ...o,
});
history.splice(
  2,
  0,
  H("h5", 52, {
    style_id: "natural",
    app: "slack",
    app_name: "Slack",
    raw: "fala galera então amanhã o deploy vai ser às dez e meia beleza qualquer coisa me chama",
    text: "Galera, amanhã o deploy é às 10h30. Qualquer coisa me chama.",
    duration_ms: 4600,
  }),
  H("h6", 95, {
    style_id: "custom-topicos",
    app: "notion",
    app_name: "Notion",
    raw: "pra reunião de hoje a gente precisa falar do orçamento do Q4 do novo onboarding e de quem vai cobrir as férias da Ana",
    text: "- Orçamento do Q4\n- Novo onboarding\n- Cobertura das férias da Ana",
    duration_ms: 6900,
  }),
  // A transcription that failed, its recording kept to try again (like Rust's FailedStore).
  H("hf1", 70, {
    raw: "",
    text: "",
    app: "outlook",
    app_name: "Outlook",
    provider: null,
    stt_provider: "",
    pasted: false,
    duration_ms: 41000,
    failed: true,
    error: "Falha na transcrição: Groq: limite de uso atingido",
    retry: "ready",
  }),
  // Right-to-left dictation (dir="auto" in the list).
  H("h-ar", 110, {
    raw: "مرحبا، سأرسل التقرير غدا صباحا",
    text: "مرحبًا، سأرسل التقرير غدًا صباحًا.",
    app: "whatsapp",
    app_name: "WhatsApp",
    duration_ms: 2800,
  }),
  H("h7", 130, {
    mode: "ask",
    raw: "qual a diferença entre tauri e electron em consumo de memória",
    text: "qual a diferença entre tauri e electron em consumo de memória",
    style_id: "natural",
    provider: "groq · openai/gpt-oss-20b",
    pasted: false,
    duration_ms: 3200,
  }),
);
history.push(
  H("h8", 60 * 30, {
    mode: "translate",
    target: "es",
    raw: "obrigado pelo retorno vou analisar a proposta com o time e te respondo até segunda",
    text: "Gracias por la respuesta. Voy a analizar la propuesta con el equipo y te respondo antes del lunes.",
    duration_ms: 5200,
  }),
  H("h9", 60 * 50, {
    style_id: "professional",
    app: "gmail",
    app_name: "Gmail",
    raw: "oi Marcos tudo bem então sobre aquele contrato eu dei uma olhada e acho que a cláusula de rescisão tá muito vaga a gente precisa definir prazo e multa direitinho antes de assinar e também incluir o reajuste anual pelo IPCA",
    text: "Olá, Marcos. Revisei o contrato e considero a cláusula de rescisão vaga demais: precisamos definir com clareza o prazo e a multa antes da assinatura. Sugiro também incluir o reajuste anual pelo IPCA.",
    duration_ms: 12800,
  }),
  H("h10", 60 * 24 * 4, {
    style_id: "verbatim",
    raw: "nota mental testar o atalho com o Slack aberto",
    text: "nota mental testar o atalho com o Slack aberto",
    provider: null,
    stt_provider: "local · base-q5_1",
    duration_ms: 2400,
  }),
  H("h12", 60 * 6, {
    mode: "command",
    app: "whatsapp",
    app_name: "WhatsApp",
    raw: "traduz pro inglês",
    source: "Chego aí em 20 minutos, pode ir pedindo.",
    text: "I'll be there in 20 minutes, go ahead and order.",
    duration_ms: 1500,
  }),
  H("h13", 60 * 7, {
    style_id: "prompt",
    app: "code",
    app_name: "VS Code",
    raw: "cria um teste que verifica se o atalho novo não conflita com os outros",
    text: "Crie um teste que verifique se o novo atalho não conflita com os outros.",
    duration_ms: 3300,
  }),
  H("h14", 60 * 8, {
    app: "teams",
    app_name: "Microsoft Teams",
    raw: "beleza fechado então às três",
    text: "Beleza, fechado então às 3.",
    duration_ms: 1400,
  }),
  H("h11", 60 * 24 * 9, {
    raw: "é o seguinte o bug acontece quando o usuário clica duas vezes rápido no botão de salvar aí ele duplica o registro",
    text: "O bug acontece quando o usuário clica duas vezes rápido no botão de salvar: o registro é duplicado.",
    pasted: false,
    duration_ms: 6100,
  }),
);
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
if (params.has("empty")) chat = [];

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

export type SimScenario =
  | "dictate"
  | "live"
  | "translate"
  | "ask"
  | "error"
  | "sttfail"
  | "done"
  | "transcribing"
  | "refining"
  | "command"
  | "cmddone"
  | "cmdnosel";

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
      if (scenario === "sttfail") {
        // Like Rust: the recording is kept in memory and offered again.
        const id = `hf-${Date.now()}`;
        history.unshift({ ...makeResult("dictate", settings.active_style, null, false), id, raw: "", text: "", failed: true, error: "Falha na transcrição: Groq: limite de uso atingido", retry: "ready" });
        mockEmit(EVENTS.historyChanged);
        emit({
          phase: "error",
          message: "Falha na transcrição: Groq: limite de uso atingido",
          detail: "groq: limite de uso atingido — HTTP 429: Rate limit reached for model `whisper-large-v3`",
          raw: null,
          retry: id,
          key: false,
        });
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

  if (scenario === "cmddone") {
    lastResult = makeCommand();
    return emit({ phase: "done", result: lastResult });
  }
  if (scenario === "cmdnosel") {
    return emit({ phase: "error", message: "Selecione um texto antes de usar o Comando.", detail: null, raw: null, hint: true });
  }
  if (scenario === "command") {
    // Like Rust: listening first, the copied selection a moment later, then the edited text.
    emit({ phase: "listening", mode: "command", style_id: settings.active_style, target: null, live: true, app: "Gmail", selection: "pending" });
    later(450, () => mockEmit(EVENTS.overlaySelection, { text: "oi pessoal, segue o relatório do trimestre. os números…" }));
    let t = 0;
    let words = 0;
    const said = CMD_RAW.charAt(0).toUpperCase() + CMD_RAW.slice(1);
    levelTimer = window.setInterval(() => {
      t += 33;
      mockEmit(EVENTS.overlayLevel, 0.3 + Math.abs(Math.sin(t / 120)) * 0.4 + Math.random() * 0.1);
      if (t > 700 && t % 330 < 33 && words < said.split(" ").length) {
        words++;
        mockEmit(EVENTS.overlayPartial, { text: said.split(" ").slice(0, words).join(" ") });
      }
    }, 33);
    stopRequested = () => {
      clearInterval(levelTimer);
      stopRequested = null;
      emit({ phase: "transcribing" });
      later(700, () => emit({ phase: "refining", provider: "Groq", mode: "command" }));
      later(1900, () => {
        lastResult = makeCommand();
        history.unshift(lastResult);
        mockEmit(EVENTS.historyChanged);
        emit({ phase: "done", result: lastResult });
      });
    };
    if (autoStop) later(4200, () => stopRequested?.());
    return;
  }
  if (scenario === "transcribing") return emit({ phase: "transcribing" });
  if (scenario === "refining") return emit({ phase: "refining", provider: "Groq" });
  if (scenario === "done") {
    lastResult = makeResult("dictate", settings.active_style, null);
    lastResult.warnings = ["Groq lento; resposta via Gemini."];
    return emit({ phase: "done", result: lastResult });
  }

  const live = scenario !== "dictate";
  emit({ phase: "listening", mode, style_id: settings.active_style, target, live, app: mode === "ask" ? null : "Outlook" });
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
  // overlay_hitbox is a cheap native call in Rust (it only stores a rect).
  await sleep(cmd === "overlay_hitbox" || cmd === "overlay_resize" ? 1 : cmd.startsWith("get_") ? 30 : 60);
  const r = await route(cmd, args);
  return r as T;
}

async function route(cmd: string, a: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "get_settings":
      return clone(settings);
    case "save_settings":
      settings = clone(a.settings as Settings);
      settings.styles.forEach((st) => st.builtin && (st.prompt = ""));
      // ?echo[=ms]: emit settings-changed like Rust does, but late, so stale echoes can arrive after newer saves.
      if (params.has("echo")) {
        const echo = clone(settings);
        setTimeout(() => mockEmit(EVENTS.settingsChanged, echo), Number(params.get("echo")) || 1200);
      }
      return clone(settings);
    case "reset_styles":
      settings.styles = clone(DEFAULT_STYLES);
      if (!settings.styles.some((s) => s.id === settings.active_style)) settings.active_style = "natural";
      mockEmit(EVENTS.settingsChanged, clone(settings));
      return clone(settings);
    case "suspend_hotkeys":
      return;
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
      if (mockDownloads.has(m.id)) return;
      const total = m.size_mb * 1024 * 1024;
      let done = 0;
      // ?dlslow stretches the simulated download (~40 s) so QA can capture it mid-way.
      const tick = params.has("dlslow") ? 1000 : 90;
      const step = () => {
        done = Math.min(total, done + total / 40);
        const finished = done >= total;
        if (finished) {
          m.installed = true;
          mockDownloads.delete(m.id);
        } else mockDownloads.set(m.id, setTimeout(step, tick));
        mockEmit(EVENTS.modelProgress, { id: m.id, downloaded: done, total, done: finished, error: null, cancelled: false });
      };
      mockDownloads.set(m.id, setTimeout(step, tick));
      return;
    }
    case "cancel_local_model_download": {
      const timer = mockDownloads.get(a.id as string);
      if (timer === undefined) return;
      clearTimeout(timer);
      mockDownloads.delete(a.id as string);
      mockEmit(EVENTS.modelProgress, { id: a.id, downloaded: 0, total: 0, done: true, error: "cancelado", cancelled: true });
      return;
    }
    case "delete_local_model": {
      const m = localModels.find((x) => x.id === a.id);
      if (m) m.installed = false;
      return;
    }

    case "start_recording": {
      const mode = a.mode as RecordMode;
      simulateOverlay(
        mode === "translate" ? "translate" : mode === "ask" ? "ask" : mode === "command" ? "command" : settings.stt.live_preview ? "live" : "dictate",
      );
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
      lastResult =
        prev.mode === "command"
          ? { ...makeCommand(target), id: prev.id, pasted: prev.pasted }
          : { ...makeResult(prev.mode, String(a.styleId), target, false), id: prev.id };
      // Like the Rust core: a reprocess also announces the new version as a `done` with the same id.
      mockEmit(EVENTS.overlayState, { phase: "done", result: clone(lastResult) } satisfies OverlayEvent);
      return clone(lastResult);
    }
    case "command_style": {
      // Like Rust: the listening card turns into "applying", then the restyled selection.
      stopSimulation();
      mockEmit(EVENTS.overlayState, { phase: "refining", provider: "Groq", mode: "command" } satisfies OverlayEvent);
      await sleep(1100);
      const target = (a.target as string | null) ?? null;
      lastResult = { ...makeCommand(target), style_id: String(a.styleId), raw: "Reescreva no estilo " + String(a.styleId) };
      mockEmit(EVENTS.overlayState, { phase: "done", result: clone(lastResult) } satisfies OverlayEvent);
      return;
    }
    case "feedback_log":
      return "2026-10-31 00:44:42 INFO  FlowVoice 2.0.0 starting\n2026-10-31 00:44:42 INFO  hotkey registered: CommandOrControl+Shift+Space\n2026-10-31 00:45:10 INFO  recording start (Dictate, app Some(\"outlook\"), style professional)\n2026-10-31 00:45:18 WARN  stt groq failed: limite de uso atingido (HTTP 429: Rate limit reached)";
    case "open_feedback":
      window.open(`https://github.com/cesarkali/Flow-Voice/issues/new?template=${a.kind === "idea" ? "feature_request" : "bug_report"}.yml`, "_blank");
      return;
    case "reprocess_history": {
      await sleep(900);
      const i = history.findIndex((h) => h.id === a.id);
      if (i < 0) throw "esse ditado não está mais no histórico";
      const target = (a.target as string | null) ?? null;
      const h = history[i];
      const next: DictationResult = {
        ...h,
        style_id: String(a.styleId),
        target,
        mode: h.mode === "command" ? "command" : target ? "translate" : "dictate",
        text: target ? `[${target.toUpperCase()}] ${h.raw}` : `${h.raw.charAt(0).toUpperCase()}${h.raw.slice(1)}`,
        warnings: [],
      };
      history[i] = next;
      mockEmit(EVENTS.historyChanged);
      return clone(next);
    }
    case "paste_text":
      return true;
    case "take_model_notices":
      return [];
    case "copy_text":
      try {
        await navigator.clipboard.writeText(String(a.text));
      } catch {
        /* ignore */
      }
      return;
    case "overlay_hold":
      return;
    case "overlay_resize":
      return; // no-op: the overlay window has a fixed size (see overlay_hitbox)
    case "overlay_hitbox":
      // The browser preview draws it and emulates Rust's overlay-pointer (src/overlay/Preview.svelte).
      mockEmit("mock:overlay-hitbox", a.rect ?? null);
      return;
    case "overlay_glass":
      return;
    case "glass_available":
      return true;
    case "get_overlay_anchor":
      return null; // the harness has no native window to place
    case "test_overlay_position":
      return;
    case "get_last_overlay_drag":
      return { x: 0.82, y: 0.3 };
    case "overlay_drag_arm":
      return false; // the harness has no native window to drag
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
    case "retry_failed": {
      // Like Rust: transcribing again, then the failed item gives way to the dictation.
      const i = history.findIndex((h) => h.id === a.id && h.retry === "ready");
      if (i < 0) return;
      history[i] = { ...history[i], retry: "running" };
      mockEmit(EVENTS.historyChanged);
      mockEmit(EVENTS.overlayState, { phase: "transcribing" } satisfies OverlayEvent);
      later(1200, () => {
        history = history.filter((h) => h.id !== a.id);
        lastResult = makeResult("dictate", settings.active_style, null, !a.onlyCopy);
        history.unshift(lastResult);
        mockEmit(EVENTS.historyChanged);
        mockEmit(EVENTS.overlayState, { phase: "done", result: clone(lastResult) } satisfies OverlayEvent);
      });
      return;
    }

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
    case "check_update": {
      await sleep(900);
      // Markdown like a real release body (CHANGELOG section + the GitHub-only footer).
      const notes = [
        "### Prévia ao vivo mais estável",
        "",
        "- O texto não volta mais atrás: cada modelo tem um pedido por vez.",
        "- Palavras confirmadas: uma palavra só fica firme quando duas respostas seguidas concordam nela.",
        "",
        "### Vidro na cápsula",
        "",
        "- Nova opção em **Geral → Janela flutuante**: **Vidro** ou **Translúcido**, com o desfoque do Windows.",
        "",
        "### Correções",
        "",
        "- A janela às vezes mostrava `state not managed…` ao iniciar.",
        "",
        "---",
        "**Instalar:** baixe `FlowVoiceSetup.exe` abaixo.",
      ].join("\n");
      const info = { available: true, version: "2.1.1", notes } satisfies UpdateInfo;
      mockEmit("update-available", info); // same event the Rust background check emits
      return info;
    }
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
    case "author_avatar":
      // ?noavatar shows the initials fallback; otherwise the live GitHub photo (needs network)
      await sleep(300);
      return (new URLSearchParams(location.search).has("noavatar") ? null : "https://avatars.githubusercontent.com/cesarkali?s=96");
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

// ?qa=<steps>: scripted UI state for visual QA screenshots (headless browser, no interaction).
// Steps separated by "~": click:<css> · clickt:<text> · wait:<ms> · type:<css>=<text> · key:<Key>
// · press:<css>=<Key> · hover:<css> · scroll:<css>=<px>
// Add &dlslow to stretch simulated model downloads to ~40 s (capture mid-download / cancel states).
if (typeof window !== "undefined" && params.get("qa")) {
  const steps = params.get("qa")!.split("~");
  const q = (sel: string) => document.querySelector<HTMLElement>(sel);
  (async () => {
    await sleep(Number(params.get("qadelay")) || 700);
    for (const st of steps) {
      const i = st.indexOf(":");
      const op = st.slice(0, i);
      const arg = st.slice(i + 1);
      try {
        if (op === "wait") await sleep(Number(arg));
        else if (op === "click") q(arg)?.click();
        else if (op === "clickt") {
          // first visible button/link/role element whose text contains arg
          const all = [...document.querySelectorAll<HTMLElement>("button, a, [role=button], [role=tab], [role=option], label")];
          all.find((el) => el.offsetParent !== null && el.textContent?.trim().includes(arg))?.click();
        }
        else if (op === "hover") q(arg)?.dispatchEvent(new PointerEvent("pointerenter", { bubbles: true }));
        else if (op === "key") (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent("keydown", { key: arg, bubbles: true }));
        else if (op === "press") {
          const [sel, k] = [arg.slice(0, arg.indexOf("=")), arg.slice(arg.indexOf("=") + 1)];
          const el = q(sel);
          el?.focus({ preventScroll: true });
          el?.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
        }
        else if (op === "type" || op === "scroll") {
          const [sel, val] = [arg.slice(0, arg.indexOf("=")), arg.slice(arg.indexOf("=") + 1)];
          const el = q(sel);
          if (op === "scroll" && el) el.scrollTop = Number(val);
          if (op === "type" && el instanceof HTMLInputElement) {
            el.focus();
            el.value = val;
            el.dispatchEvent(new Event("input", { bubbles: true }));
          }
        }
      } catch (e) {
        console.warn("[qa]", st, e);
      }
      await sleep(120);
    }
  })();
}
