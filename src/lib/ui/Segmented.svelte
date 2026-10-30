<script lang="ts" generics="T extends string">
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

  function key(e: KeyboardEvent) {
    let i = idx;
    if (e.key === "ArrowRight" || e.key === "ArrowDown") i = (idx + 1) % options.length;
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") i = (idx - 1 + options.length) % options.length;
    else return;
    e.preventDefault();
    pick(options[i].value);
    const btns = (e.currentTarget as HTMLElement).querySelectorAll("button");
    btns[i]?.focus();
  }
  function pick(v: T) {
    value = v;
    onchange?.(v);
  }
</script>

<div
  class="seg {size}"
  role="radiogroup"
  aria-label={label}
  tabindex="-1"
  onkeydown={key}
  style="--n:{options.length};--i:{idx}"
>
  <span class="thumb" aria-hidden="true"></span>
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
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: inline-grid;
    grid-template-columns: repeat(var(--n), minmax(0, 1fr));
    padding: 2px;
    border-radius: var(--r);
    background: var(--surface-2);
    border: 1px solid var(--line);
    flex: none;
  }
  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc((100% - 4px) / var(--n));
    transform: translateX(calc(var(--i) * 100%));
    border-radius: 6px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: transform var(--t) var(--ease);
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
    transition: color var(--t-fast) var(--ease);
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
    font-weight: 500;
  }
</style>
