// Which local whisper model to offer during onboarding, for the user's speech language.
import type { LocalModel } from "../../lib/types";

export const fitsLang = (m: LocalModel, lang: string) => m.langs.includes("*") || m.langs.includes(lang);
/** 2 = tuned for this language, 1 = recommended for every language, 0 = merely supports it. */
const affinity = (m: LocalModel, lang: string) => (m.recommended_for.includes(lang) ? 2 : m.recommended_for.includes("*") ? 1 : 0);

/** Models that transcribe `lang`, best first: recommended ones, then accuracy, language fit, speed, size. */
export function rankFor(models: LocalModel[], lang: string): LocalModel[] {
  return models
    .filter((m) => fitsLang(m, lang))
    .sort(
      (a, b) =>
        Number(affinity(b, lang) > 0) - Number(affinity(a, lang) > 0) ||
        b.quality - a.quality ||
        affinity(b, lang) - affinity(a, lang) ||
        b.speed - a.speed ||
        a.size_mb - b.size_mb,
    );
}

export interface ModelPicks {
  /** Most accurate model for the language. */
  top?: LocalModel;
  /** Clearly lighter alternative (≤ half the size) for modest PCs. */
  light?: LocalModel;
  /** A good model for the language that is already on disk (nothing to download). */
  installed?: LocalModel;
  /** Options shown to the user, in display order. */
  options: LocalModel[];
}

export function pickModels(models: LocalModel[], lang: string): ModelPicks {
  const r = rankFor(models, lang);
  const top = r[0];
  const rest = r.filter((m) => m !== top);
  const light = top
    ? (rest.find((m) => m.size_mb <= top.size_mb * 0.5 && m.quality >= 3) ?? rest.find((m) => m.size_mb < top.size_mb))
    : undefined;
  const installed = r.find((m) => m.installed && (m.quality >= 3 || affinity(m, lang) > 0));
  const options = [installed, top, light].filter((m, i, a): m is LocalModel => !!m && a.indexOf(m) === i);
  return { top, light, installed, options };
}

export const fmtSize = (mb: number) => (mb >= 1000 ? `${(mb / 1024).toFixed(1).replace(/\.0$/, "")} GB` : `${Math.round(mb)} MB`);

/** "português" for ("pt", "pt"), "Portuguese" for ("pt", "en"). */
export function langName(code: string, ui: string): string {
  try {
    return new Intl.DisplayNames([ui], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}
