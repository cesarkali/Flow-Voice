<script lang="ts">
  // Saved API keys of one provider as rows (masked, removable, revealable) + an "add key" field.
  // Rust keeps them as one comma-joined list; the helpers in providers.ts translate.
  import { flip } from "svelte/animate";
  import { slide } from "svelte/transition";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app, setKeys } from "../../lib/stores.svelte";
  import type { KeyProviderId } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import { addKeys, isKeyList, pname, removeKeyAt, savedKeys } from "../providers";

  let {
    provider,
    keyPrefix,
    onadded,
    onremoved,
  }: { provider: KeyProviderId; keyPrefix?: string; onadded?: () => void; onremoved?: (left: number) => void } = $props();

  const masked = $derived(app.keys?.[provider]?.masked ?? []);
  // Stable ids for keyed rows even when two masks look the same.
  const rows = $derived(masked.map((m, i) => ({ m, i, id: `${m}#${masked.slice(0, i).filter((x) => x === m).length}` })));

  let draft = $state("");
  let busy = $state(false);
  let revealed = $state<string[] | null>(null);
  let msg = $state<{ text: string; tone: "ok" | "warn" } | null>(null);
  let msgTimer = 0;
  const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const dur = (ms: number) => (reduce ? 0 : ms);

  function say(text: string, tone: "ok" | "warn") {
    msg = { text, tone };
    clearTimeout(msgTimer);
    msgTimer = window.setTimeout(() => (msg = null), 3200);
  }

  async function add(raw = draft) {
    if (!raw.trim() || busy) return;
    busy = true;
    try {
      const n = await addKeys(provider, raw);
      draft = "";
      if (n === 0) say(t("obx.key_dup"), "warn");
      else {
        if (n > 1) say(t("obx.keys_added", { n }), "ok");
        if (revealed) revealed = await savedKeys(provider);
        onadded?.();
      }
    } catch (e) {
      say(String(e), "warn");
    } finally {
      busy = false;
    }
  }

  function onpaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData("text") ?? "";
    if (!isKeyList(text)) return;
    // A pasted list becomes several rows right away.
    e.preventDefault();
    add(`${draft},${text}`);
  }

  async function remove(i: number) {
    if (busy) return;
    busy = true;
    try {
      await removeKeyAt(provider, i);
      if (revealed) revealed = masked.length ? await savedKeys(provider) : null;
      onremoved?.(masked.length);
    } finally {
      busy = false;
    }
  }

  async function removeAll() {
    if (busy) return;
    busy = true;
    try {
      await setKeys(provider, "");
      revealed = null;
      onremoved?.(0);
    } finally {
      busy = false;
    }
  }

  async function toggleReveal() {
    revealed = revealed ? null : await savedKeys(provider);
  }
</script>

<div class="pk">
  {#if rows.length}
    <div class="head" transition:slide={{ duration: dur(200) }}>
      <span class="label">{t("obx.keys_saved")} <span class="cnt mono">{rows.length}</span></span>
      <span class="tools">
        <button type="button" class="link" onclick={toggleReveal} aria-pressed={!!revealed}>
          <Icon name={revealed ? "eyeOff" : "eye"} size={14} />{revealed ? t("prov.hide") : t("prov.reveal")}
        </button>
        {#if rows.length > 1}
          <button type="button" class="link danger" onclick={removeAll}>{t("obx.remove_all")}</button>
        {/if}
      </span>
    </div>
    <ul class="rows" aria-label="{t('obx.keys_saved')} {pname(provider)}">
      {#each rows as r (r.id)}
        <li animate:flip={{ duration: dur(220) }} in:slide={{ duration: dur(220) }} out:slide={{ duration: dur(180) }}>
          <div class="row">
            <span class="idx mono" class:first={r.i === 0}>{r.i + 1}</span>
            {#key !!revealed}
              <span class="val mono" class:full={!!revealed}>{revealed?.[r.i] ?? r.m}</span>
            {/key}
            <button type="button" class="rm" aria-label="{t('obx.key_remove')} {r.i + 1}" title={t("obx.key_remove")} onclick={() => remove(r.i)}>
              <Icon name="x" size={14} />
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="addrow">
    <input
      class="field mono"
      type="password"
      autocomplete="off"
      spellcheck="false"
      aria-label="{t('obx.key_add')} — {pname(provider)}"
      placeholder={rows.length ? t("obx.key_add_placeholder") : `${t("prov.key_placeholder")}${keyPrefix ? ` (${keyPrefix})` : ""}`}
      bind:value={draft}
      {onpaste}
      onkeydown={(e) => e.key === "Enter" && add()}
    />
    <Button variant={draft.trim() ? "primary" : "default"} icon="plus" loading={busy && !!draft.trim()} disabled={!draft.trim()} onclick={() => add()}>
      {t("obx.key_add")}
    </Button>
  </div>
  {#if msg}
    <p class="msg" class:warn={msg.tone === "warn"} role="status" transition:slide={{ duration: dur(160) }}>{msg.text}</p>
  {/if}
  <p class="hint">{t("obx.keys_multi")}</p>
</div>

<style>
  .pk {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .cnt {
    margin-left: 4px;
    color: var(--text-3);
  }
  .tools {
    display: flex;
    gap: 2px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-2);
    padding: 0 7px;
    height: 24px;
    border-radius: 5px;
    transition:
      color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease);
  }
  .link:hover {
    color: var(--text);
    background: var(--hover);
  }
  .link.danger:hover {
    color: var(--danger-fg);
    background: var(--danger-soft);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--bg);
    overflow: hidden;
  }
  .rows li + li .row {
    border-top: 1px solid var(--line);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 4px 0 10px;
  }
  .idx {
    width: 18px;
    height: 18px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    font-size: 10px;
    color: var(--text-3);
    transition:
      color var(--t) var(--ease),
      border-color var(--t) var(--ease);
  }
  .idx.first {
    color: var(--accent-fg);
    border-color: var(--accent-line);
  }
  .val {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    animation: fv-fade var(--t) var(--ease) both;
  }
  .val.full {
    color: var(--text);
    user-select: text;
  }
  .rm {
    width: 28px;
    height: 28px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--text-3);
    opacity: 0.6;
    transition:
      opacity var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease),
      transform var(--t) var(--spring);
  }
  .row:hover .rm,
  .rm:focus-visible {
    opacity: 1;
  }
  .rm:hover {
    color: var(--danger-fg);
    background: var(--danger-soft);
  }
  .rm:active {
    transform: scale(0.9);
  }
  .addrow {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .addrow .field {
    flex: 1;
    min-width: 0;
  }
  .msg {
    font-size: 12px;
    color: var(--ok);
  }
  .msg.warn {
    color: var(--danger-fg);
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
</style>
