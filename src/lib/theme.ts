import type { Theme } from "./types";

/** Applies the theme to <html data-theme>. "system" removes the attribute so prefers-color-scheme decides. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") delete root.dataset.theme;
  else root.dataset.theme = theme;
}

/** Resolved theme right now (useful for things that can't use CSS variables). */
export function resolvedTheme(): "dark" | "light" {
  const t = document.documentElement.dataset.theme;
  if (t === "dark" || t === "light") return t;
  return matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

export const DEFAULT_ACCENT = "#D7FF3F";

/** Curated accent presets (first = default phosphor lime). */
export const ACCENT_PRESETS = [
  "#D7FF3F", // lima
  "#5BE3FF", // ciano
  "#4DFFB0", // menta
  "#FFB13D", // âmbar
  "#FF6B4A", // coral
  "#FF5FA2", // rosa
  "#B69CFF", // lavanda
  "#F2F2EE", // gelo
];

type Rgb = [number, number, number];

function parseHex(hex: string): Rgb | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  let h = m[1];
  if (h.length === 3) h = h.split("").map((c) => c + c).join("");
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

const lum = ([r, g, b]: Rgb) => {
  const f = (v: number) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
};
const contrast = (a: Rgb, b: Rgb) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};
const rgba = ([r, g, b]: Rgb, a: number) => `rgba(${r}, ${g}, ${b}, ${a})`;
const hex = ([r, g, b]: Rgb) => "#" + [r, g, b].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");

/** Darkens until it reads on the light paper background (WCAG ~4.5:1). */
function readableOnLight(c: Rgb): Rgb {
  const paper: Rgb = [244, 243, 239];
  let out = c;
  for (let k = 1; k <= 20 && contrast(out, paper) < 4.5; k++) {
    out = c.map((v) => v * (1 - k * 0.045)) as Rgb;
  }
  return out;
}

export function isValidAccent(color: string): boolean {
  return parseHex(color) !== null;
}

const ACCENT_VARS = [
  "--accent",
  "--on-accent",
  "--accent-fg",
  "--accent-soft",
  "--accent-line",
  "--cap-accent",
  "--cap-accent-line",
  "--cap-accent-soft",
  "--cap-wave",
];
let currentAccent: string | null = null;
let watching = false;

/**
 * Applies the user's accent color on top of base.css. The default lime keeps the hand-tuned
 * values from base.css; any other color gets derived soft/line/foreground variants for the
 * resolved theme. Uses CSSOM custom properties on <html> (an injected <style> element would be
 * blocked by the app's CSP inside Tauri).
 */
export function applyAccent(color: string | null | undefined) {
  currentAccent = color ?? null;
  const root = document.documentElement.style;
  for (const v of ACCENT_VARS) root.removeProperty(v);
  if (!watching) {
    watching = true;
    matchMedia("(prefers-color-scheme: light)").addEventListener("change", () => applyAccent(currentAccent));
    new MutationObserver(() => applyAccent(currentAccent)).observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
  }
  const c = parseHex(color ?? "");
  if (!c || hex(c).toUpperCase() === DEFAULT_ACCENT) return;
  const on = lum(c) > 0.45 ? "#0b0b0c" : "#ffffff";
  const vars: Record<string, string> =
    resolvedTheme() === "light"
      ? (() => {
          const fg = readableOnLight(c);
          return {
            "--accent": hex(c),
            "--on-accent": on,
            "--accent-fg": hex(fg),
            "--accent-soft": rgba(c, 0.16),
            "--accent-line": rgba(fg, 0.45),
            "--cap-accent": hex(c),
            "--cap-accent-line": hex(fg),
            "--cap-accent-soft": rgba(fg, 0.18),
            "--cap-wave": "#161616",
          };
        })()
      : {
          "--accent": hex(c),
          "--on-accent": on,
          "--accent-fg": hex(c),
          "--accent-soft": rgba(c, 0.1),
          "--accent-line": rgba(c, 0.35),
          "--cap-accent": hex(c),
          "--cap-accent-line": hex(c),
          "--cap-accent-soft": rgba(c, 0.22),
          "--cap-wave": hex(c),
        };
  for (const [k, v] of Object.entries(vars)) root.setProperty(k, v);
}
