<script lang="ts">
  import { api } from "../api";
  import { t } from "../i18n.svelte";
  import Icon from "../icons/Icon.svelte";
  import Keys from "./Keys.svelte";
  import { acceleratorFromEvent, keycapsOf, sameHotkey } from "./hotkeys";

  let {
    value = $bindable(""),
    label,
    onchange,
    size = "md",
    clearable = false,
    taken = [],
  }: {
    value: string;
    label: string;
    onchange?: (v: string) => void;
    size?: "md" | "lg";
    /** Shows a remove button; an empty value means "no shortcut". */
    clearable?: boolean;
    /** The other actions' shortcuts: the same combination can't go to two of them. */
    taken?: { accel: string; label: string }[];
  } = $props();

  let capturing = $state(false);
  // Leaving the page mid-capture must resume the global shortcuts.
  $effect(() => () => {
    if (capturing) api.suspendHotkeys(false).catch(() => {});
  });
  let live = $state<string[]>([]);
  let error = $state("");
  let btn: HTMLButtonElement;
  let saved = $state(false);
  let savedTimer: ReturnType<typeof setTimeout> | undefined;

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

  function clear() {
    error = "";
    value = "";
    onchange?.("");
    btn?.focus();
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
    const clash = taken.find((o) => sameHotkey(o.accel, accel));
    if (clash) {
      error = t("sc.taken", { action: clash.label });
      return;
    }
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
    saved = true;
    clearTimeout(savedTimer);
    savedTimer = setTimeout(() => (saved = false), 900);
  }
</script>

<svelte:window onkeydown={onKey} onkeyup={onKey} />

<div class="hk">
  <div class="line">
  <button
    bind:this={btn}
    type="button"
    class="cap {size}"
    class:capturing
    class:withclear={clearable && !!value && !capturing}
    class:saved
    class:err={!!error}
    aria-label="{label}: {value}. {t('sc.click_to_change')}"
    onclick={() => (capturing ? stop() : start())}
    onblur={stop}
  >
    {#if capturing}
      {#if live.length}
        <Keys caps={live} size={size === "lg" ? "lg" : "md"} animate pressed />
        <span class="plus" aria-hidden="true">+</span>
        <span class="ghost {size}" aria-hidden="true"></span>
      {:else}
        <span class="ghosts" aria-hidden="true">
          <span class="ghost {size}"></span><span class="ghost {size}"></span>
        </span>
        <span class="prompt">{t("sc.press")}</span>
      {/if}
      <span class="esc mono" aria-hidden="true">{t("ctl.esc_cancel")}</span>
    {:else}
      {#if value}
        {#key value}
          <Keys accel={value} size={size === "lg" ? "lg" : "md"} animate={saved} />
        {/key}
      {:else}
        <span class="none">{t("sc.off")}</span>
      {/if}
      <span class="edit"><Icon name="edit" size={15} /></span>
    {/if}
  </button>
  {#if clearable && value && !capturing}
    <button type="button" class="clear" aria-label={t("sc.clear")} title={t("sc.clear")} onclick={clear}>
      <Icon name="x" size={14} />
    </button>
  {/if}
  </div>
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
  /* the remove button sits inside the cap's right edge, so clearable rows line up with the rest */
  .line {
    position: relative;
    display: inline-flex;
  }
  .withclear {
    padding-right: 38px;
  }
  .clear {
    position: absolute;
    right: 5px;
    top: 50%;
    translate: 0 -50%;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--r-sm);
    color: var(--text-3);
    transition:
      color var(--t-fast),
      background var(--t-fast);
  }
  .clear:hover {
    color: var(--text);
    background: var(--hover);
  }
  .none {
    font-size: 13px;
    color: var(--text-3);
    padding-left: 4px;
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
      box-shadow var(--t) var(--ease),
      transform 140ms var(--spring);
  }
  .cap:active {
    transform: scale(0.98);
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
    transform: translateX(-4px);
    transition:
      opacity var(--t-fast),
      transform var(--t) var(--spring);
  }
  .cap:hover .edit,
  .cap:focus-visible .edit {
    opacity: 1;
    transform: none;
  }
  .prompt {
    color: var(--text-2);
    font-size: 13px;
  }
  .ghosts {
    display: inline-flex;
    gap: 3px;
  }
  .ghost {
    display: inline-block;
    width: 26px;
    height: 22px;
    border-radius: 5px;
    border: 1px dashed var(--accent-line);
    animation: ghost-in 240ms var(--spring) both;
  }
  .ghost.lg {
    width: 34px;
    height: 30px;
    border-radius: 7px;
  }
  .ghosts .ghost:nth-child(2) {
    width: 40px;
    animation-delay: 40ms;
  }
  .ghosts .ghost.lg:nth-child(2) {
    width: 54px;
  }
  @keyframes ghost-in {
    from {
      opacity: 0;
      transform: scale(0.7);
    }
  }
  .plus {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-3);
    margin: 0 -4px;
  }
  .esc {
    margin-left: auto;
    font-size: 10.5px;
    color: var(--text-3);
    animation: ghost-in 300ms var(--ease) 120ms both;
    white-space: nowrap;
  }
  .saved {
    animation: saved 900ms var(--ease);
  }
  @keyframes saved {
    0% {
      border-color: var(--accent-line);
      box-shadow: 0 0 0 0 var(--accent-soft);
    }
    30% {
      border-color: var(--accent-line);
      box-shadow: 0 0 0 5px var(--accent-soft);
    }
    100% {
      box-shadow: 0 0 0 0 transparent;
    }
  }
  .msg {
    font-size: 12.5px;
    color: var(--danger-fg);
    animation: ghost-in 200ms var(--ease) both;
  }
</style>
