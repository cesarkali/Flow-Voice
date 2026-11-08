<script lang="ts">
  // Discreet toast: "Gemini: o modelo X foi descontinuado — trocado para Y automaticamente".
  // Shows notices that happened while this window was closed, then live ones (EVENTS.modelHealed).
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { api, on } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import { EVENTS, type ModelNotice } from "../lib/types";

  let items = $state<(ModelNotice & { key: number })[]>([]);
  let seq = 0;

  function push(n: ModelNotice) {
    const key = ++seq;
    items = [...items.filter((i) => i.message !== n.message), { ...n, key }].slice(-3);
    setTimeout(() => dismiss(key), 12000);
  }
  function dismiss(key: number) {
    items = items.filter((i) => i.key !== key);
  }

  onMount(() => {
    api.takeModelNotices().then((list) => list.forEach(push)).catch(() => {});
    const un = on<ModelNotice>(EVENTS.modelHealed, (n) => {
      push(n);
      api.takeModelNotices().catch(() => {}); // shown here: don't replay on next open
    });
    return () => {
      un.then((f) => f());
    };
  });
</script>

<div class="stack" aria-live="polite">
  {#each items as n (n.key)}
    <div class="toast" role="status" transition:fly={{ y: 8, duration: 180 }}>
      <span class="ico"><Icon name="refresh" size={15} /></span>
      <span class="msg">{n.message}</span>
      <button type="button" class="x" aria-label={t("c.close")} onclick={() => dismiss(n.key)}>
        <Icon name="x" size={13} />
      </button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: min(420px, calc(100vw - 32px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 10px 10px 12px;
    border-radius: var(--r);
    background: var(--surface-2);
    color: var(--text);
    box-shadow: var(--shadow-pop);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .ico {
    color: var(--accent-fg);
    margin-top: 1px;
    flex: none;
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .x {
    flex: none;
    border: 0;
    background: transparent;
    color: var(--text-3);
    padding: 2px;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
