<script lang="ts">
  // Shared dialog: scrim fade + panel rise/scale on open and close. Esc and scrim click close it.
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "../icons/Icon.svelte";
  import { t } from "../i18n.svelte";

  let {
    open = $bindable(false),
    title,
    width = "520px",
    onclose,
    children,
    footer,
  }: {
    open: boolean;
    title: string;
    width?: string;
    onclose?: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  function close() {
    open = false;
    onclose?.();
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && close()} />

{#if open}
  <div class="scrim" transition:fade={{ duration: 160 }} onclick={close} role="presentation"></div>
  <div
    class="panel"
    style:width
    role="dialog"
    aria-modal="true"
    aria-label={title}
    in:scale={{ duration: 220, start: 0.94, opacity: 0, easing: cubicOut }}
    out:scale={{ duration: 140, start: 0.97, opacity: 0 }}
  >
    <header>
      <h2>{title}</h2>
      <button class="x" aria-label={t("c.close")} onclick={close}><Icon name="x" size={16} /></button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
    z-index: 90;
  }
  .panel {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 64px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-pop);
    z-index: 91;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px 8px;
  }
  h2 {
    margin: 0;
    font: 600 15px/1.3 var(--font-display);
  }
  .x {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  .body {
    padding: 8px 18px 16px;
    overflow: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--line);
  }
</style>
