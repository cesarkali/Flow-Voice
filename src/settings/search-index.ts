// Static index for the global settings search (title bar). Every entry points at a page and,
// when possible, an element id; otherwise the page DOM is searched for the label text.
// Labels are i18n keys, so results follow the UI language, and matching also accepts the
// pt/en/es spellings of every label (typing "theme" finds "Tema" in pt).
import { i18n, t, type TKey } from "../lib/i18n.svelte";
import type { IconName } from "../lib/icons/paths";
import { pt } from "../lib/locales/pt";
import { en } from "../lib/locales/en";
import { es } from "../lib/locales/es";
import { EXTRA } from "../lib/locales/x";
import { app } from "../lib/stores.svelte";
import type { SettingsPage } from "../lib/types";
import { PROVIDERS, pname } from "./providers";

export interface SearchEntry {
  page: SettingsPage;
  /** label i18n key (or `text` for names that are not translated) */
  key?: TKey;
  text?: string;
  hint?: TKey | (() => string);
  /** element id to scroll to; without it the label text is looked up in the page */
  id?: string;
  /** extra words (any language) that should also match */
  kw?: string;
}

export const PAGE_META: Record<SettingsPage, { key: TKey; icon: IconName }> = {
  home: { key: "nav.home", icon: "home" },
  styles: { key: "nav.styles", icon: "styles" },
  providers: { key: "nav.providers", icon: "key" },
  speech: { key: "nav.speech", icon: "wave" },
  shortcuts: { key: "nav.shortcuts", icon: "keyboard" },
  general: { key: "nav.general", icon: "sliders" },
  about: { key: "nav.about", icon: "info" },
};

const STATIC: SearchEntry[] = [
  // pages themselves
  ...(Object.keys(PAGE_META) as SettingsPage[]).map((p) => ({ page: p, key: PAGE_META[p].key, hint: "sh.search_page" as TKey })),

  // Home
  { page: "home", key: "home.test_now", kw: "testar test probar ditar dictate dictar microfone" },
  { page: "home", key: "home.setup_line", kw: "voz ia estilo voice ai style" },
  { page: "home", key: "home.stat_today", kw: "estatisticas stats" },
  { page: "home", key: "home.stat_words", kw: "estatisticas stats palavras words" },
  { page: "home", key: "home.stat_saved", kw: "estatisticas stats tempo time" },
  { page: "home", key: "home.history", kw: "historico history ditados dictations" },
  { page: "home", key: "home.search", kw: "historico history" },
  { page: "home", key: "home.clear", hint: "home.history", kw: "apagar limpar delete clear borrar" },

  // Styles
  { page: "styles", key: "home.sty_builtin_group", kw: "estilos styles prontos builtin" },
  { page: "styles", key: "home.sty_custom_group", kw: "personalizado custom prompt" },
  { page: "styles", key: "home.sty_create", kw: "novo new nuevo prompt instrucao" },
  { page: "styles", key: "apps.sec", hint: "apps.enable_hint", id: "set-sty-apps", kw: "aplicativo app programa outlook slack por app per app" },
  { page: "styles", key: "styles.translation_target", hint: "styles.translation_hint", kw: "traduzir translate traducir idioma" },
  { page: "styles", key: "styles.remove_fillers", hint: "styles.remove_fillers_hint", kw: "vicios fillers muletillas limpeza" },
  { page: "styles", key: "styles.sec_dictionary", hint: "styles.dictionary_hint", kw: "vocabulario vocabulary termos palavras nomes" },

  // Providers
  { page: "providers", key: "prov.sec_ai", kw: "chave key api clave llm ia" },
  { page: "providers", key: "prov.sec_search", kw: "web busca search tavily" },
  { page: "providers", key: "prov.order_title", hint: "prov.order_hint", kw: "failover fallback prioridade priority" },
  { page: "providers", key: "prov.order_llm", hint: "prov.order_hint" },
  { page: "providers", key: "prov.order_stt", hint: "prov.order_hint" },

  // Speech
  { page: "speech", key: "speech.cloud_title", hint: "speech.cloud_body", kw: "motor engine nuvem cloud" },
  { page: "speech", key: "speech.local_title", hint: "speech.local_body", kw: "offline whisper privado private" },
  { page: "speech", key: "speech.language", hint: "speech.language_hint", id: "set-speech-language", kw: "idioma language" },
  { page: "speech", key: "speech.mic", id: "set-speech-mic", kw: "microfone microphone micrófono audio entrada input" },
  { page: "speech", key: "speech.live", hint: "ctl.live_hint", id: "set-speech-live", kw: "previa preview ao vivo" },
  { page: "speech", key: "speech.models", hint: "ctl.models_hint", kw: "whisper baixar download modelo" },

  // Shortcuts
  { page: "shortcuts", key: "sc.dictate", hint: "sc.dictate_hint", id: "set-sc-dictate", kw: "atalho hotkey tecla shortcut atajo" },
  { page: "shortcuts", key: "sc.translate", hint: () => t("sc.translate_hint", { lang: "…" }), id: "set-sc-translate", kw: "atalho hotkey traduzir" },
  { page: "shortcuts", key: "sc.ask", hint: "sc.ask_hint", id: "set-sc-ask", kw: "atalho hotkey assistente chat pergunta" },
  { page: "shortcuts", key: "sc.mode", hint: "sc.mode_toggle_hint", id: "set-sc-mode", kw: "segurar hold toggle alternar push to talk" },
  { page: "shortcuts", key: "sc.command", hint: "sc.command_hint", id: "set-sc-command", kw: "comando command selecao selecionado editar reescrever" },
  { page: "shortcuts", key: "sc.paste_last", hint: "sc.paste_last_hint", id: "set-sc-paste-last", kw: "colar de novo paste again ultimo repetir" },

  // General
  { page: "general", key: "gen.name", hint: "gen.name_hint", kw: "nome name nombre saudacao" },
  { page: "general", key: "gen.accent", hint: "gen.accent_hint", kw: "cor color destaque acento accent paleta" },
  { page: "general", key: "gen.ui_language", kw: "idioma language interface lingua automatico windows francais deutsch" },
  { page: "about", key: "sh.about_privacy", kw: "privacidade privacy dados data telemetria" },
  { page: "general", key: "gen.theme", hint: "ctl.theme_hint", kw: "tema theme escuro claro dark light modo" },
  { page: "general", key: "ctl.sec_overlay", kw: "overlay janela flutuante floating capsula" },
  { page: "general", key: "gen.overlay_position", hint: "ctl.ov_pos_hint", kw: "overlay posicao position personalizada custom livre free arrastar mover onde canto corner" },
  { page: "general", key: "ctl.ov_drag", hint: "ctl.ov_drag_hint", kw: "arrastar drag mover move cápsula capsule overlay gravação recording monitor" },
  { page: "general", key: "ctl.ov_test", hint: "ctl.ov_test_hint", kw: "testar test posicao position overlay" },
  { page: "general", key: "ctl.ov_width", hint: "ctl.ov_width_hint", kw: "overlay largura width" },
  { page: "general", key: "ctl.ov_follow", hint: "ctl.ov_follow_hint", kw: "seguir mouse follow monitor tela em uso ativo principal active main screen display" },
  { page: "general", key: "ctl.ov_glass", hint: "ctl.ov_glass_hint", kw: "vidro glass cristal transparente translucido translúcido desfoque blur acrylic acrílico tema aparência cápsula" },
  { page: "general", key: "ctl.win_glass", hint: "ctl.win_glass_hint", kw: "vidro janela configurações acrílico acrylic mica glass window transparente" },
  { page: "general", key: "ctl.ov_glass_light", hint: "ctl.ov_glass_light_hint", kw: "luz sombra light shadow macos brilho vidro glass" },
  { page: "general", key: "gen.auto_paste", hint: "gen.auto_paste_hint", kw: "colar paste pegar" },
  { page: "general", key: "gen.mute", hint: "ctl.mute_hint", kw: "silenciar mute audio som volume" },
  { page: "general", key: "gen.sounds", hint: "ctl.sounds_hint", kw: "sons sounds sonidos bip" },
  { page: "general", key: "gen.start_os", hint: "gen.start_os_hint", kw: "iniciar startup boot windows" },
  { page: "general", key: "gen.history", hint: "gen.history_hint", kw: "historico history" },
  { page: "general", key: "gen.sec_updates", hint: "gen.check", kw: "atualizar update version versao" },

  // About
  { page: "about", key: "sh.about_support_title", hint: "sh.donate", kw: "doar doacao donate donar apoiar pix sponsors patrocinar" },
  { page: "about", key: "sh.about_author", kw: "autor author julio caliberda cesarkali" },
  { page: "about", key: "about.profile", hint: "sh.about_author", kw: "github perfil" },
  { page: "about", key: "about.site", hint: "sh.about_author" },
  { page: "about", key: "about.instagram", hint: "sh.about_author" },
  { page: "about", key: "about.repo", hint: "sh.about_project", kw: "github codigo source" },
  { page: "about", key: "about.version", hint: "sh.about_project" },
  { page: "about", key: "about.data_dir", hint: "sh.about_project", kw: "pasta folder dados data" },
  { page: "about", key: "sh.about_notes", kw: "changelog novidades" },
];

/** Entries that depend on data (provider names, the user's styles). */
function dynamic(): SearchEntry[] {
  const out: SearchEntry[] = PROVIDERS.map((p) => ({
    page: "providers" as const,
    text: pname(p.id),
    hint: p.desc,
    id: `prov-${p.id}`,
    kw: `${p.id} ${p.stt ? "stt voz voice" : ""} ${p.llm ? "llm ia" : ""} chave key api`,
  }));
  for (const st of app.settings?.styles ?? []) {
    out.push({ page: "styles", text: st.name, hint: () => st.description || t("nav.styles"), kw: st.id });
  }
  return out;
}

// ---------------------------------------------------------------- matching

/** Lowercase + strip accents. */
export function fold(s: string): string {
  return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

const ALL: Record<string, string>[] = [pt, en, es].map((d) => ({ ...d }) as Record<string, string>);
EXTRA.forEach((x) => ["pt", "en", "es"].forEach((l, i) => Object.assign(ALL[i], (x as Record<string, Record<string, string>>)[l])));
const allLangs = (k: TKey) => ALL.map((d) => d[k] ?? "").join(" ");

export interface SearchHit {
  uid: string;
  page: SettingsPage;
  label: string;
  hint: string;
  pageLabel: string;
  icon: IconName;
  id?: string;
  /** the result is a page itself (navigate only, nothing to highlight) */
  pageOnly: boolean;
  /** [start, end) of the matched span inside label (for highlighting) */
  mark: [number, number] | null;
}

/** Ranked results for `query` (empty query → none). */
export function searchSettings(query: string, limit = 8): SearchHit[] {
  void i18n.lang; // re-run when the language changes
  const q = fold(query.trim());
  if (!q) return [];
  const tokens = q.split(/\s+/).filter(Boolean);
  const scored: { e: SearchEntry; score: number; i: number; label: string }[] = [];
  const entries = [...STATIC, ...dynamic()];
  entries.forEach((e, i) => {
    const label = e.key ? t(e.key) : (e.text ?? "");
    const fl = fold(label);
    const pageLabel = t(PAGE_META[e.page].key);
    const hint = typeof e.hint === "function" ? e.hint() : e.hint ? t(e.hint) : "";
    const rest = fold(`${e.key ? allLangs(e.key) : ""} ${e.kw ?? ""} ${hint} ${pageLabel}`);
    let score = 0;
    for (const tok of tokens) {
      const at = fl.indexOf(tok);
      if (at === 0) score += 6;
      else if (at > 0 && /[\s(/·-]/.test(fl[at - 1])) score += 4;
      else if (at > 0) score += 2;
      else if (rest.includes(tok)) score += 1;
      else return;
    }
    if (fl === q) score += 6;
    // page entries rank just under rows of the same strength
    if (e.hint === "sh.search_page") score -= 0.5;
    scored.push({ e, score, i, label });
  });
  scored.sort((a, b) => b.score - a.score || a.i - b.i);
  const seen = new Set<string>();
  const out: SearchHit[] = [];
  for (const { e, label } of scored) {
    const uid = `${e.page}|${e.id ?? label}`;
    if (seen.has(uid)) continue;
    seen.add(uid);
    out.push({
      uid,
      page: e.page,
      label,
      hint: typeof e.hint === "function" ? e.hint() : e.hint ? t(e.hint) : "",
      pageLabel: t(PAGE_META[e.page].key),
      icon: PAGE_META[e.page].icon,
      id: e.id,
      pageOnly: e.hint === "sh.search_page",
      mark: markOf(label, tokens),
    });
    if (out.length >= limit) break;
  }
  return out;
}

/** Span of the longest token inside label, in original (unfolded) indices. */
function markOf(label: string, tokens: string[]): [number, number] | null {
  // fold char by char so indices map back to the original string
  const map: number[] = [];
  let f = "";
  [...label].forEach((ch, idx) => {
    const fc = fold(ch);
    for (let k = 0; k < fc.length; k++) map.push(idx);
    f += fc;
  });
  const chars = [...label];
  const tok = [...tokens].sort((a, b) => b.length - a.length).find((x) => f.includes(x));
  if (!tok) return null;
  const s = f.indexOf(tok);
  const startChar = map[s];
  const endChar = map[s + tok.length - 1] + 1;
  // convert code-point indices to string indices
  const start = chars.slice(0, startChar).join("").length;
  const end = chars.slice(0, endChar).join("").length;
  return [start, end];
}
