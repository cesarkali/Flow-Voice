<script lang="ts">
  // UI language picker with SVG flags; a thumb slides to the selected language.
  import type { UiLanguage } from "../types";
  import Flag from "./Flag.svelte";

  let { value = $bindable("pt"), label }: { value: UiLanguage; label: string } = $props();

  const LANGS: { code: UiLanguage; name: string }[] = [
    { code: "pt", name: "Português" },
    { code: "en", name: "English" },
    { code: "es", name: "Español" },
  ];
  const idx = $derived(Math.max(0, LANGS.findIndex((l) => l.code === value)));

  // The thumb measures the active option (names have different widths, bold is wider).
  let root = $state<HTMLDivElement>();
  let thumb = $state({ x: 0, w: 0, ready: false });
  function measure() {
    const b = root?.querySelector<HTMLElement>(`[data-code="${value}"]`);
    if (b) thumb = { x: b.offsetLeft, w: b.offsetWidth, ready: true };
  }
  $effect(() => {
    void value;
    measure();
  });
  $effect(() => {
    if (!root) return;
    const ro = new ResizeObserver(measure);
    ro.observe(root);
    return () => ro.disconnect();
  });

  function key(e: KeyboardEvent) {
    const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    value = LANGS[(idx + d + LANGS.length) % LANGS.length].code;
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-code="${value}"]`)?.focus();
  }
</script>

<div class="lp" bind:this={root} role="radiogroup" aria-label={label} tabindex="-1" onkeydown={key}>
  <span
    class="thumb"
    class:ready={thumb.ready}
    aria-hidden="true"
    style:width="{thumb.w}px"
    style:transform="translateX({thumb.x}px)"
  ></span>
  {#each LANGS as l (l.code)}
    <button
      type="button"
      role="radio"
      data-code={l.code}
      aria-checked={value === l.code}
      tabindex={value === l.code ? 0 : -1}
      class:on={value === l.code}
      onclick={() => (value = l.code)}
    >
      <span class="fl"><Flag code={l.code} size={20} /></span>
      <span>{l.name}</span>
    </button>
  {/each}
</div>

<style>
  .lp {
    position: relative;
    display: flex;
    padding: 3px;
    border: 1px solid var(--line-strong);
    border-radius: var(--r-lg);
    background: var(--surface-2);
    outline: none;
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 0;
    border-radius: calc(var(--r-lg) - 3px);
    background: var(--surface);
    box-shadow: 0 1px 0 var(--line-strong), 0 0 0 1px var(--accent-line);
    opacity: 0;
  }
  .thumb.ready {
    opacity: 1;
    transition:
      transform 320ms var(--spring),
      width 320ms var(--spring);
  }
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 1 1 auto;
    gap: 7px;
    height: 36px;
    padding: 0 10px;
    white-space: nowrap;
    border: 0;
    background: none;
    color: var(--text-2);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    transition: color var(--t) var(--ease);
  }
  button:hover {
    color: var(--text);
  }
  button.on {
    color: var(--text);
    font-weight: 600;
  }
  .fl {
    display: inline-flex;
    transition: transform 240ms var(--spring), filter var(--t) var(--ease);
    filter: saturate(0.55) brightness(0.9);
  }
  button.on .fl,
  button:hover .fl {
    filter: none;
  }
  button.on .fl {
    transform: scale(1.12) rotate(-4deg);
  }
  button:active .fl {
    transform: scale(0.92);
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    border-radius: var(--r);
  }
  @media (prefers-reduced-motion: reduce) {
    .thumb,
    .fl {
      transition: none;
    }
  }
</style>
