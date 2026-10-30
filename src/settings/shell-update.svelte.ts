// Update state + donation links shared by the sidebar footer (badge) and the About page.
import { api, on } from "../lib/api";
import { i18n, type TKey } from "../lib/i18n.svelte";
import { EVENTS, type UpdateInfo } from "../lib/types";

export const upd = $state({
  info: null as UpdateInfo | null,
  checking: false,
  checked: false,
  installing: false,
  error: "",
  progress: null as { downloaded: number; total: number | null } | null,
});

export async function checkUpdate() {
  if (upd.checking) return;
  upd.checking = true;
  upd.error = "";
  try {
    upd.info = await api.checkUpdate();
    upd.checked = true;
  } catch (e) {
    upd.error = String(e);
  } finally {
    upd.checking = false;
  }
}

export async function installUpdate() {
  if (upd.installing) return;
  upd.installing = true;
  upd.error = "";
  try {
    await api.installUpdate();
  } catch (e) {
    upd.error = String(e);
    upd.installing = false;
  }
}

/** 0..1, or null when the total size is unknown. */
export function updateFraction(): number | null {
  const p = upd.progress;
  if (!p || !p.total) return null;
  return Math.min(1, p.downloaded / p.total);
}

/** Subscribes to the Rust background check (`update-available`) and download progress. */
export function listenUpdates(): () => void {
  const subs = [
    on<UpdateInfo>("update-available", (info) => {
      if (!info) return;
      upd.info = info;
      upd.checked = true;
    }),
    on<{ downloaded: number; total: number | null }>(EVENTS.updateProgress, (p) => (upd.progress = p)),
  ];
  return () => subs.forEach((s) => s.then((f) => f()));
}

// ------------------------------------------------------------------ donations

export type DonateOption = { id: "pix" | "sponsors"; key: TKey; url: string };
const PIX: DonateOption = { id: "pix", key: "sh.pix", url: "https://livepix.gg/caliberda" };
const SPONSORS: DonateOption = { id: "sponsors", key: "sh.sponsors", url: "https://github.com/sponsors/cesarkali" };

/** Pix first for Portuguese, GitHub Sponsors first otherwise. Reactive (reads i18n.lang). */
export function donateOptions(): DonateOption[] {
  return i18n.lang === "pt" ? [PIX, SPONSORS] : [SPONSORS, PIX];
}
