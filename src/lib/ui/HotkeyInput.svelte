<script lang="ts">
  import { api } from "../api";
  import { t } from "../i18n.svelte";
  import Icon from "../icons/Icon.svelte";
  import Keys from "./Keys.svelte";
  import { acceleratorFromEvent, keycapsOf } from "./hotkeys";

  let {
    value = $bindable(""),
    label,
    onchange,
    size = "md",
  }: { value: string; label: string; onchange?: (v: string) => void; size?: "md" | "lg" } = $props();

  let capturing = $state(false);
  let live = $state<string[]>([]);
  let error = $state("");
  let btn: HTMLButtonElement;

  function start() {
    error = "";
    live = [];
    capturing = true;
    api.suspendHotkeys(true).catch(() => {});
  }
  function stop() {
    if (capturing) api.suspendHotkeys(false).catch(() => {});
    capturing = false;
    live = [];
  }

  async function onKey(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "keyup") {
      // releasing modifiers without a key: show remaining
      live = keycapsOf(acceleratorFromEvent(e).mods.join("+"));
      return;
    }
    if (e.key === "Escape" && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      stop();
      return;
    }
    const r = acceleratorFromEvent(e);
    if (r.needsModifier) {
      error = t("sc.need_modifier");
      return;
    }
    if (!r.accel) {
      live = keycapsOf(r.mods.join("+"));
      return;
    }
    const accel = r.accel;
    stop();
    try {
      const problem = await api.validateHotkey(accel);
      if (problem) {
        error = problem;
        return;
      }
    } catch (err) {
      error = String(err);
      return;
    }
    error = "";
    value = accel;
    onchange?.(accel);
    btn?.focus();
  }
</script>

<svelte:window onkeydown={onKey} onkeyup={onKey} />

<div class="hk">
  <button
    bind:this={btn}
    type="button"
    class="cap {size}"
    class:capturing
    class:err={!!error}
    aria-label="{label}: {value}. {t('sc.click_to_change')}"
    onclick={() => (capturing ? stop() : start())}
    onblur={stop}
  >
    {#if capturing}
      {#if live.length}
        <Keys caps={live} size={size === "lg" ? "lg" : "md"} />
      {:else}
        <span class="rec" aria-hidden="true"></span>
        <span class="prompt">{t("sc.press")}</span>
      {/if}
    {:else}
      <Keys accel={value} size={size === "lg" ? "lg" : "md"} />
      <span class="edit"><Icon name="edit" size={15} /></span>
    {/if}
  </button>
  {#if error}
    <p class="msg" role="alert">{error}</p>
  {/if}
</div>

<style>
  .hk {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .cap {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-width: 210px;
    height: 38px;
    padding: 0 10px 0 8px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    transition:
      border-color var(--t-fast) var(--ease),
      box-shadow var(--t-fast) var(--ease);
  }
  .cap.lg {
    height: 52px;
    min-width: 280px;
    padding: 0 14px 0 10px;
  }
  .cap:hover {
    border-color: var(--text-3);
  }
  .capturing {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .err {
    border-color: var(--danger-line);
  }
  .edit {
    margin-left: auto;
    color: var(--text-3);
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .cap:hover .edit,
  .cap:focus-visible .edit {
    opacity: 1;
  }
  .prompt {
    color: var(--text-2);
    font-size: 13px;
  }
  .rec {
    width: 8px;
    height: 8px;
    margin-left: 4px;
    border-radius: 50%;
    background: var(--accent);
    animation: blink 1s steps(2, start) infinite;
  }
  @keyframes blink {
    to {
      opacity: 0.2;
    }
  }
  .msg {
    font-size: 12.5px;
    color: var(--danger-fg);
  }
</style>
