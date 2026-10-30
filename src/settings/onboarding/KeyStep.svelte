<script lang="ts">
  import { api } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import type { KeyProviderId, TestResult } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import ProvMark from "../pages/ProvMark.svelte";
  import { addKeys, pname, providerInfo } from "../providers";

  let { provider, ok = $bindable(false) }: { provider: KeyProviderId; ok?: boolean } = $props();
  const info = $derived(providerInfo(provider));
  const has = $derived((app.keys?.[provider]?.count ?? 0) > 0);

  let draft = $state("");
  let busy = $state(false);
  let result = $state<TestResult | null>(null);

  $effect(() => {
    // reset when switching provider
    provider;
    result = null;
    ok = false;
    draft = "";
  });

  async function saveTest() {
    busy = true;
    result = null;
    try {
      // Appends (never replaces) — a pasted comma list becomes several keys.
      if (draft.trim()) await addKeys(provider, draft);
      result = await api.testProvider(provider, info.llm ? "llm" : "stt");
      ok = result.ok;
      if (ok) draft = "";
    } catch (e) {
      result = { ok: false, latency_ms: 0, model: "", message: String(e) };
    } finally {
      busy = false;
    }
  }
</script>

<div class="ks">
  <ol>
    {#each info.steps as step, i}
      <li>
        <span class="n mono">{i + 1}</span>
        <p>{t(step)}</p>
        {#if i === 0}
          <Button size="sm" variant="primary" iconRight="external" onclick={() => api.openUrl(info.url)}>{t("c.open_site")}</Button>
        {/if}
      </li>
    {/each}
  </ol>
  {#if info.note}<p class="note"><Icon name="info" size={14} />{t(info.note)}</p>{/if}

  <div class="keyrow">
    <input
      class="field mono"
      type="password"
      autocomplete="off"
      spellcheck="false"
      aria-label="{t('prov.key')} {pname(provider)}"
      placeholder={has ? t("obx.key_add_placeholder") : `${t("prov.key_placeholder")}${info.keyPrefix ? ` (${info.keyPrefix})` : ""}`}
      bind:value={draft}
      onkeydown={(e) => e.key === "Enter" && saveTest()}
    />
    <Button variant={draft.trim() ? "primary" : "default"} loading={busy} disabled={!draft.trim() && !has} onclick={saveTest}>
      {busy ? t("c.testing") : t("ob.save_test")}
    </Button>
  </div>

  {#if has}
    <div class="saved">
      {#each app.keys?.[provider]?.masked ?? [] as m, i (m + i)}
        <span class="chip mono" style="--i:{i}"><Icon name="key" size={12} />{m}</span>
      {/each}
    </div>
  {/if}

  {#if result}
    <div class="res" class:ok={result.ok} role="status">
      <ProvMark ok={result.ok} size={18} />
      <div>
        <p>{result.ok ? t("ob.test_ok", { ms: result.latency_ms }) : t("ob.test_fail")}</p>
        {#if !result.ok && result.message}<code class="mono">{result.message}</code>{/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ks {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13.5px;
  }
  li p {
    flex: 1;
  }
  .n {
    width: 22px;
    height: 22px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    font-size: 11px;
    color: var(--text-2);
  }
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: 12px;
    color: var(--text-2);
  }
  .keyrow {
    display: flex;
    gap: 8px;
  }
  .keyrow .field {
    flex: 1;
    height: 38px;
  }
  .keyrow :global(.btn) {
    height: 38px;
  }
  .saved {
    margin-top: -8px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border-radius: 5px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    font-size: 11.5px;
    color: var(--text-2);
    animation: fv-pop var(--t-slow) var(--spring) backwards;
    animation-delay: calc(var(--i) * 40ms);
  }
  .res {
    display: flex;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--r);
    border: 1px solid var(--danger-line);
    background: var(--danger-soft);
    color: var(--danger-fg);
    font-size: 13px;
    transform-origin: top center;
    animation: enter 260ms var(--ease-out) both;
  }
  .res.ok {
    border-color: var(--accent-line);
    background: var(--accent-soft);
    color: var(--accent-fg);
  }
  .res div {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  code {
    font-size: 11.5px;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translate3d(0, -6px, 0) scale(0.98);
    }
  }
</style>
