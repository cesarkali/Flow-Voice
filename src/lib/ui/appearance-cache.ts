// Remembers the last theme + accent so a window paints with them on its very first frame,
// before the settings round-trip to Rust (avoids a lime -> custom accent / dark -> light flash).
// Each window shares the app origin, so the settings window keeps it fresh for the chat window.
import { applyAccent, applyTheme } from "../theme";
import type { Theme } from "../types";

const KEY = "fv.appearance";

/**
 * Flags red/coral/magenta-red accents on <html data-warm-accent>, so success states switch to a
 * green (base.css `--ok*`) instead of reading as errors in the accent color.
 */
export function markWarmAccent(accent: string | null | undefined) {
  const m = /^#?([0-9a-f]{6})$/i.exec((accent ?? "").trim());
  let warm = false;
  if (m) {
    const [r, g, b] = [0, 2, 4].map((i) => parseInt(m[1].slice(i, i + 2), 16) / 255);
    const max = Math.max(r, g, b);
    const d = max - Math.min(r, g, b);
    if (d > 0.25 && max === r) {
      const h = (((g - b) / d) * 60 + 360) % 360;
      warm = h < 28 || h > 330;
    }
  }
  document.documentElement.toggleAttribute("data-warm-accent", warm);
}

export function restoreAppearance() {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return;
    const { theme, accent } = JSON.parse(raw) as { theme?: Theme; accent?: string };
    if (theme) applyTheme(theme);
    if (accent) applyAccent(accent);
    markWarmAccent(accent);
  } catch {
    /* storage unavailable: first frame uses the defaults */
  }
}

export function rememberAppearance(theme: Theme, accent: string) {
  markWarmAccent(accent);
  try {
    localStorage.setItem(KEY, JSON.stringify({ theme, accent }));
  } catch {
    /* ignore */
  }
}
