<script lang="ts" generics="T extends string">
  // Segmented control with a measured sliding thumb. The thumb only animates
  // when the selection changes; size changes (first paint, language switch,
  // window resize) snap it into place without a jump.
  import { tick } from "svelte";
  import Icon from "../icons/Icon.svelte";
  import type { IconName } from "../icons/paths";

  let {
    value = $bindable(),
    options,
    label,
    onchange,
    size = "md",
  }: {
    value: T;
    options: { value: T; label: string; icon?: IconName }[];
    label: string;
    onchange?: (v: T) => void;
    size?: "sm" | "md";
  } = $props();

  const idx = $derived(Math.max(0, options.findIndex((o) => o.value === value)));

  let root: HTMLDivElement;
  let thumb = $state({ x: 0, w: 0 });
  let ready = $state(false);
  let animate = $state(false);

  function measure() {
    const b = root?.querySelectorAll<HTMLButtonElement>(":scope > button")[idx];
    if (!b) return;
    thumb = { x: b.offsetLeft, w: b.offsetWidth };
  }

  // Selection changed → animated move.
  let lastIdx = -1;
  $effect(() => {
    const i = idx;
    void options.length;
    tick().then(() => {
      animate = ready && lastIdx !== -1 && lastIdx !== i;
      lastIdx = i;
      measure();
      ready = true;
    });
  });

  // Size changed → snap (no transition).
  $effect(() => {
    const ro = new ResizeObserver(() => {
      animate = false;
      measure();
    });
    ro.observe(root);
    for (const b of root.querySelectorAll(":scope > button")) ro.observe(b);
    return () => ro.disconnect();
  });

  function key(e: KeyboardEvent) {
    let i = idx;
    if (e.key === "ArrowRight" || e.key === "ArrowDown") i = (idx + 1) % options.length;
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") i = (idx - 1 + options.length) % options.length;
    else if (e.key === "Home") i = 0;
    else if (e.key === "End") i = options.length - 1;
    else return;
    e.preventDefault();
    pick(options[i].value);
    const btns = (e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>(":scope > button");
    btns[i]?.focus();
  }
  function pick(v: T) {
    if (v === value) return;
    value = v;
    onchange?.(v);
  }
</script>

<div
  bind:this={root}
  class="seg {size}"
  role="radiogroup"
  aria-label={label}
  tabindex="-1"
  onkeydown={key}
  style="--n:{options.length}"
>
  <span
    class="thumb"
    class:ready
    class:animate
    aria-hidden="true"
    style="transform: translateX({thumb.x}px); width: {thumb.w}px"
  ></span>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={o.value === value}
      tabindex={o.value === value ? 0 : -1}
      class:sel={o.value === value}
      onclick={() => pick(o.value)}
    >
      {#if o.icon}<Icon name={o.icon} size={15} />{/if}
      <span>{o.label}</span>
    </button>
  {/each}
</div>

<style>
  .seg {
    --seg-spring: var(--spring);
    position: relative;
    display: inline-grid;
    grid-template-columns: repeat(var(--n), minmax(0, 1fr));
    padding: 2px;
    border-radius: var(--r);
    background: var(--surface-2);
    border: 1px solid var(--line);
    flex: none;
    isolation: isolate;
  }
  @supports (transition-timing-function: linear(0, 1)) {
    .seg {
      --seg-spring: linear(0, 0.063, 0.206, 0.379, 0.549, 0.698, 0.818, 0.907, 0.968, 1.007, 1.028, 1.037, 1.038, 1.034, 1.028, 1.021, 1.015, 1.01, 1.006, 1.003, 1);
    }
  }
  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 0;
    z-index: -1;
    border-radius: 6px;
    background: var(--surface);
    box-shadow:
      0 0 0 1px var(--line-strong),
      0 1px 2px rgba(0, 0, 0, 0.18);
    opacity: 0;
    will-change: transform;
  }
  .thumb.ready {
    opacity: 1;
  }
  .thumb.animate {
    transition:
      transform 420ms var(--seg-spring),
      width 420ms var(--seg-spring);
  }
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    padding: 0 14px;
    font-size: 13px;
    color: var(--text-2);
    border-radius: 6px;
    white-space: nowrap;
    transition: color var(--t) var(--ease);
  }
  button > span {
    display: inline-block;
    transition: transform 120ms var(--ease);
  }
  button:active > span {
    transform: scale(0.96);
  }
  .sm button {
    height: 24px;
    padding: 0 10px;
    font-size: 12.5px;
  }
  button:hover {
    color: var(--text);
  }
  button.sel {
    color: var(--text);
  }
  button :global(svg) {
    transition: color var(--t) var(--ease);
  }
  button.sel :global(svg) {
    color: var(--accent-fg);
  }
  button:focus-visible {
    outline-offset: -2px;
  }
</style>
