// The author's identity + photo, shared by the sidebar card (App.svelte) and the About page.
// The photo comes from Rust (GitHub avatar, cached on disk for offline use) and is fetched once
// per window; `author.avatar` stays null until it arrives (or forever, if never downloaded).
import { api } from "../lib/api";

export const AUTHOR = {
  name: "Júlio Caliberda",
  handle: "cesarkali",
  initials: "JC",
  github: "https://github.com/cesarkali",
} as const;

export const author = $state({ avatar: null as string | null, loaded: false });

let started = false;

/** Starts loading the avatar (idempotent). Call from any component that shows it. */
export function loadAuthorAvatar(): void {
  if (started) return;
  started = true;
  api
    .authorAvatar()
    .then((a) => (author.avatar = a || null))
    .catch(() => {})
    .finally(() => (author.loaded = true));
}
