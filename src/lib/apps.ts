// App ids for "Estilos por aplicativo". Mirrors src-tauri/src/apps.rs (normalize, friendly
// names): keep both tables in sync. Rust normalizes again on save, so this is only for the UI.
import type { DictationResult, Settings } from "./types";

const ALIASES: Record<string, string> = {
  olk: "outlook",
  hxoutlook: "outlook",
  "ms-teams": "teams",
  msteams: "teams",
  "microsoft teams": "teams",
  "whatsapp.root": "whatsapp",
  whatsappdesktop: "whatsapp",
  "telegram desktop": "telegram",
  discordptb: "discord",
  discordcanary: "discord",
  winword: "word",
  "code - insiders": "code",
  vscode: "code",
  "vs code": "code",
  "visual studio code": "code",
  "google chrome": "chrome",
  "microsoft edge": "msedge",
  edge: "msedge",
  "mozilla firefox": "firefox",
  "brave browser": "brave",
  "chatgpt desktop": "chatgpt",
  "claude desktop": "claude",
};

const KNOWN: Record<string, string> = {
  outlook: "Outlook",
  thunderbird: "Thunderbird",
  gmail: "Gmail",
  mailspring: "Mailspring",
  slack: "Slack",
  discord: "Discord",
  whatsapp: "WhatsApp",
  telegram: "Telegram",
  teams: "Microsoft Teams",
  signal: "Signal",
  code: "VS Code",
  cursor: "Cursor",
  windsurf: "Windsurf",
  zed: "Zed",
  claude: "Claude",
  chatgpt: "ChatGPT",
  gemini: "Gemini",
  chrome: "Chrome",
  msedge: "Edge",
  firefox: "Firefox",
  brave: "Brave",
  opera: "Opera",
  vivaldi: "Vivaldi",
  arc: "Arc",
  word: "Word",
  excel: "Excel",
  powerpnt: "PowerPoint",
  onenote: "OneNote",
  notion: "Notion",
  obsidian: "Obsidian",
  linkedin: "LinkedIn",
  notepad: "Bloco de Notas",
  "notepad++": "Notepad++",
  windowsterminal: "Terminal",
  explorer: "Explorador de Arquivos",
};

/** "C:\\…\\OUTLOOK.EXE" / "Slack.exe" / "Outlook" → "outlook". */
export function normalizeApp(input: string): string {
  let base = input.trim().split(/[\\/]/).pop()?.trim().toLowerCase() ?? "";
  if (base.endsWith(".exe")) base = base.slice(0, -4).trim();
  return ALIASES[base] ?? base;
}

export function appName(id: string): string {
  return KNOWN[id] ?? (id ? id.charAt(0).toUpperCase() + id.slice(1) : "");
}

/** Two-letter monogram for the app tile ("VS Code" → "VS", "Outlook" → "Ou"). */
export function appMonogram(id: string): string {
  const name = appName(id).replace(/[^\p{L}\p{N} ]/gu, "").trim();
  const words = name.split(/\s+/).filter(Boolean);
  if (words.length > 1) return (words[0][0] + words[1][0]).toUpperCase();
  return name.slice(0, 2).replace(/^./, (c) => c.toUpperCase());
}

/** Stable hue per app, so tiles are told apart at a glance without real icons. */
export function appHue(id: string): number {
  let h = 0;
  for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return h;
}

/**
 * One-click suggestions. `prefer` lists style ids in order; the first one the user has wins
 * (custom installs may lack a "Mensagem" style, so chat apps fall back to Natural).
 */
const SUGGEST: { app: string; prefer: string[] }[] = [
  { app: "outlook", prefer: ["professional", "formal"] },
  { app: "gmail", prefer: ["professional", "formal"] },
  { app: "thunderbird", prefer: ["professional", "formal"] },
  { app: "slack", prefer: ["casual", "message", "natural"] },
  { app: "whatsapp", prefer: ["casual", "message", "natural"] },
  { app: "discord", prefer: ["casual", "message", "natural"] },
  { app: "telegram", prefer: ["casual", "message", "natural"] },
  { app: "teams", prefer: ["casual", "message", "natural"] },
  { app: "code", prefer: ["prompt"] },
  { app: "cursor", prefer: ["prompt"] },
  { app: "windsurf", prefer: ["prompt"] },
  { app: "claude", prefer: ["prompt"] },
  { app: "chatgpt", prefer: ["prompt"] },
];

export function suggestions(s: Settings, recent: string[] = [], max = 6): { app: string; style_id: string }[] {
  const has = new Set(s.app_styles.map((r) => r.app));
  const out = SUGGEST.filter((x) => !has.has(x.app))
    .map((x) => ({ app: x.app, style_id: x.prefer.find((id) => s.styles.some((st) => st.id === id)) ?? "" }))
    .filter((x) => x.style_id);
  // Apps the user actually dictates into come first.
  out.sort((a, b) => rank(recent, a.app) - rank(recent, b.app));
  return out.slice(0, max);
}
const rank = (recent: string[], id: string) => {
  const i = recent.indexOf(id);
  return i < 0 ? 999 : i;
};

/** Apps seen in the history, most recent first (FlowVoice itself excluded). */
export function recentApps(history: DictationResult[]): { app: string; name: string; count: number }[] {
  const map = new Map<string, { app: string; name: string; count: number }>();
  for (const h of history) {
    const id = h.app ? normalizeApp(h.app) : "";
    if (!id || id === "flowvoice") continue;
    const e = map.get(id);
    if (e) e.count++;
    else map.set(id, { app: id, name: h.app_name || appName(id), count: 1 });
  }
  return [...map.values()];
}
