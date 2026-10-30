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
