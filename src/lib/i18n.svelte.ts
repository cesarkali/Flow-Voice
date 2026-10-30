// Tiny i18n: t(key, params) reads a rune, so any template using it re-renders
// when the language changes (live switch, no reload).
import type { UiLanguage } from "./types";
import { pt } from "./locales/pt";
import { en } from "./locales/en";
import { es } from "./locales/es";
import { EXTRA, type ExtraKey } from "./locales/x";

export type TKey = keyof typeof pt | ExtraKey;

const DICTS: Record<UiLanguage, Record<string, string>> = { pt: { ...pt }, en: { ...en }, es: { ...es } };
for (const ext of EXTRA) {
  for (const lang of ["pt", "en", "es"] as const) Object.assign(DICTS[lang], ext[lang]);
}
const PT: Record<string, string> = DICTS.pt;
const LOCALE: Record<UiLanguage, string> = { pt: "pt-BR", en: "en-US", es: "es" };

export const i18n = $state({ lang: "pt" as UiLanguage });

export function setLanguage(lang: UiLanguage) {
  if (!DICTS[lang]) lang = "pt";
  i18n.lang = lang;
  document.documentElement.lang = LOCALE[lang];
}

export function t(key: TKey, params?: Record<string, string | number>): string {
  let s = DICTS[i18n.lang]?.[key] ?? PT[key] ?? key;
  if (params) for (const k in params) s = s.replaceAll(`{${k}}`, String(params[k]));
  return s;
}

export function locale() {
  return LOCALE[i18n.lang];
}

const cap = (s: string) => s.charAt(0).toLocaleUpperCase() + s.slice(1);

/** Human name of an ISO-639-1 language in the current UI language ("auto" handled). */
export function langName(code: string | null | undefined): string {
  if (!code) return "";
  if (code === "auto") return t("lang.auto");
  try {
    const dn = new Intl.DisplayNames([locale()], { type: "language" });
    // Whisper writes Javanese as "jw"; Intl only knows "jv".
    return cap(dn.of(code === "jw" ? "jv" : code) ?? code);
  } catch {
    return code.toUpperCase();
  }
}

/** "há 4 min", "ontem", "12 de out." */
export function relTime(iso: string): string {
  const d = new Date(iso);
  const diff = (Date.now() - d.getTime()) / 1000;
  const rtf = new Intl.RelativeTimeFormat(locale(), { numeric: "auto", style: "short" });
  if (diff < 45) return rtf.format(0, "second");
  if (diff < 3600) return rtf.format(-Math.round(diff / 60), "minute");
  if (diff < 3600 * 20) return rtf.format(-Math.round(diff / 3600), "hour");
  if (diff < 3600 * 24 * 6) return rtf.format(-Math.round(diff / 86400), "day");
  return d.toLocaleDateString(locale(), { day: "numeric", month: "short" });
}

export function clockTime(iso: string): string {
  return new Date(iso).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" });
}

/** "auto" + every ISO-639-1 code Whisper accepts (jw = Javanese, as whisper.cpp spells it). */
export const SPEECH_LANGS = [
  "auto",
  "af", "am", "ar", "as", "az", "ba", "be", "bg", "bn", "bo", "br", "bs", "ca", "cs", "cy", "da", "de", "el", "en", "es",
  "et", "eu", "fa", "fi", "fo", "fr", "gl", "gu", "ha", "haw", "he", "hi", "hr", "ht", "hu", "hy", "id", "is", "it", "ja",
  "jw", "ka", "kk", "km", "kn", "ko", "la", "lb", "ln", "lo", "lt", "lv", "mg", "mi", "mk", "ml", "mn", "mr", "ms", "mt",
  "my", "ne", "nl", "nn", "no", "oc", "pa", "pl", "ps", "pt", "ro", "ru", "sa", "sd", "si", "sk", "sl", "sn", "so", "sq",
  "sr", "su", "sv", "sw", "ta", "te", "tg", "th", "tk", "tl", "tr", "tt", "uk", "ur", "uz", "vi", "yi", "yo", "zh", "yue",
];
/** Translation targets (any language the LLM writes well). The first 10 are the overlay's quick picks. */
export const TARGET_LANGS = [
  "en", "es", "pt", "fr", "de", "it", "ja", "zh", "ko", "ru", "nl", "pl", "uk", "tr", "ar", "he", "fa", "hi", "bn", "ur",
  "id", "ms", "vi", "th", "sv", "da", "no", "fi", "cs", "el", "ro", "hu", "ca", "fil",
];
