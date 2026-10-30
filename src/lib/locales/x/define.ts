// Per-area string tables merged into the main dictionaries by i18n.svelte.ts.
// en/es must have exactly the same keys as pt (type-checked).
export function defineExt<K extends string>(d: {
  pt: Record<K, string>;
  en: Record<NoInfer<K>, string>;
  es: Record<NoInfer<K>, string>;
}) {
  return d;
}
