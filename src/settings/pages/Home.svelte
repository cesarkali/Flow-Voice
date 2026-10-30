<script lang="ts">
  import { onMount } from "svelte";
  import { api, on } from "../../lib/api";
  import { clockTime, langName, relTime, t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app, hasKey, usableLlm } from "../../lib/stores.svelte";
  import { EVENTS, type DictationResult, type SettingsPage } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import Keys from "../../lib/ui/Keys.svelte";
  import { pname, STT_PROVIDERS } from "../providers";

  let { onnavigate }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);

  let history = $state<DictationResult[]>([]);
  let loaded = $state(false);
  let q = $state("");
  let open = $state<string | null>(null);
  let confirmClear = $state(false);
  let copied = $state<string | null>(null);

  async function load() {
    history = await api.getHistory();
    loaded = true;
  }

  onMount(() => {
    load();
    const un = on(EVENTS.historyChanged, load);
    return () => un.then((f) => f());
  });

  const greeting = $derived.by(() => {
    const h = new Date().getHours();
    return h < 12 ? t("home.morning") : h < 18 ? t("home.afternoon") : t("home.evening");
  });

  const stt = $derived.by(() => {
    if (s.stt.engine === "local") return { name: t("speech.local_title"), model: s.stt.local_model };
    const p = s.stt.cloud_order.find((id) => hasKey(id));
    return p ? { name: pname(p), model: s.providers[p].stt_model } : null;
  });
  const llm = $derived.by(() => {
    const p = usableLlm()[0];
    return p ? { name: pname(p), model: s.providers[p].model } : null;
  });
  const activeStyle = $derived(s.styles.find((x) => x.id === s.active_style));
  const styleName = (id: string) => s.styles.find((x) => x.id === id)?.name ?? id;
  const needsSetup = $derived((s.stt.engine === "cloud" && !STT_PROVIDERS.some((p) => hasKey(p))) || !llm);

  const filtered = $derived.by(() => {
    const term = q.trim().toLowerCase();
    if (!term) return history;
    return history.filter((h) => h.text.toLowerCase().includes(term) || h.raw.toLowerCase().includes(term));
  });

  async function copy(h: DictationResult) {
    await api.copyText(h.text);
    copied = h.id;
    setTimeout(() => copied === h.id && (copied = null), 1400);
  }
  async function remove(h: DictationResult) {
    history = history.filter((x) => x.id !== h.id);
    await api.deleteHistoryItem(h.id);
  }
  async function clear() {
    if (!confirmClear) {
      confirmClear = true;
      setTimeout(() => (confirmClear = false), 3000);
      return;
    }
    confirmClear = false;
    history = [];
    await api.clearHistory();
  }
</script>

<header class="hero">
  <div>
    <h1>{greeting}</h1>
    <p class="muted">{t("home.ready")}</p>
  </div>
</header>

<dl class="status">
  <div>
    <dt class="label">{t("home.voice")}</dt>
    <dd class:none={!stt}>
      <span class="led" class:on={!!stt}></span>
      <span class="v">{stt ? stt.name : t("home.none")}</span>
    </dd>
    {#if stt}<dd class="model mono">{stt.model}</dd>{/if}
  </div>
  <div>
    <dt class="label">{t("home.ai")}</dt>
    <dd class:none={!llm}>
      <span class="led" class:on={!!llm}></span>
      <span class="v">{llm ? llm.name : t("home.none")}</span>
    </dd>
    {#if llm}<dd class="model mono">{llm.model}</dd>{/if}
  </div>
  <div>
    <dt class="label">{t("home.style")}</dt>
    <dd><span class="v">{styleName(s.active_style)}</span></dd>
    {#if activeStyle}<dd class="model mono">{activeStyle.kind === "llm" ? t("styles.kind_llm") : t("styles.kind_off")}</dd>{/if}
  </div>
</dl>

{#if needsSetup}
  <div class="setup" role="status">
    <Icon name="key" size={18} />
    <p>{t("home.setup_needed")}</p>
    <Button size="sm" variant="primary" iconRight="arrowRight" onclick={() => onnavigate?.("providers")}>{t("home.setup")}</Button>
  </div>
{/if}

<div class="try">
  <Button variant="primary" size="lg" icon="mic" onclick={() => api.startRecording("dictate")}>{t("home.test_now")}</Button>
  <div class="hks">
    <div class="hk"><span class="muted">{t("home.hk_dictate")}</span><Keys accel={s.hotkeys.dictate} size="sm" /></div>
    <div class="hk">
      <span class="muted">{t("home.hk_translate")} <span class="faint">→ {langName(s.translation_target)}</span></span>
      <Keys accel={s.hotkeys.translate} size="sm" />
    </div>
    <div class="hk"><span class="muted">{t("home.hk_ask")}</span><Keys accel={s.hotkeys.ask} size="sm" /></div>
  </div>
</div>

<section class="hist">
  <header>
    <h2 class="label">{t("home.history")}</h2>
    {#if history.length}
      <div class="tools">
        <label class="search">
          <Icon name="search" size={15} />
          <input bind:value={q} placeholder={t("home.search")} aria-label={t("home.search")} spellcheck="false" />
        </label>
        <Button size="sm" variant={confirmClear ? "danger" : "ghost"} icon="trash" onclick={clear}>
          {confirmClear ? t("c.confirm_again") : t("home.clear")}
        </Button>
      </div>
    {/if}
  </header>

  {#if !s.history_enabled}
    <p class="empty-line muted">{t("home.history_off")}</p>
  {:else if loaded && !history.length}
    <div class="empty">
      <div class="empty-keys"><Keys accel={s.hotkeys.dictate} size="lg" /></div>
      <h3>{t("home.empty_title")}</h3>
      <p class="muted">{t("home.empty_body")}</p>
    </div>
  {:else if loaded && !filtered.length}
    <p class="empty-line muted">{t("home.no_results", { q })}</p>
  {:else}
    <ul class="items">
      {#each filtered as h (h.id)}
        <li class:open={open === h.id}>
          <div class="item">
            <button
              type="button"
              class="main"
              aria-expanded={open === h.id}
              onclick={() => (open = open === h.id ? null : h.id)}
            >
              <div class="meta">
                <span class="time tnum" title={new Date(h.ts).toLocaleString()}>{relTime(h.ts)}</span>
                <span class="dot">·</span>
                {#if h.mode === "translate"}
                  <span class="tag"><Icon name="translate" size={13} />{langName(h.target)}</span>
                {:else if h.mode === "ask"}
                  <span class="tag">{t("mode.ask")}</span>
                {:else}
                  <span class="tag">{styleName(h.style_id)}</span>
                {/if}
                {#if !h.pasted}<span class="faint">· {t("home.not_pasted")}</span>{/if}
              </div>
              <p class="text">{h.text}</p>
            </button>
            <div class="acts">
              <Button
                size="sm"
                variant="ghost"
                icon={copied === h.id ? "check" : "copy"}
                aria-label={t("c.copy")}
                title={t("c.copy")}
                onclick={() => copy(h)}
              />
              <Button size="sm" variant="ghost" icon="trash" aria-label={t("c.delete")} title={t("c.delete")} onclick={() => remove(h)} />
            </div>
          </div>
          {#if open === h.id}
            <div class="detail">
              <div class="label">{t("home.raw")}</div>
              <p class="raw">{h.raw}</p>
              <div class="tech mono">
                <span>{clockTime(h.ts)}</span>
                <span>{(h.duration_ms / 1000).toFixed(1)}s</span>
                <span>{h.stt_provider}</span>
                {#if h.provider}<span>{t("home.via", { provider: h.provider })}</span>{/if}
              </div>
              {#each h.warnings as w}
                <p class="warn"><Icon name="alert" size={13} />{w}</p>
              {/each}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .hero h1 {
    font-size: 30px;
    line-height: 1.15;
    letter-spacing: -0.03em;
  }
  .hero p {
    margin-top: 4px;
  }
  .status {
    margin: 26px 0 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }
  .status > div {
    padding: 12px 14px 13px 0;
    min-width: 0;
  }
  .status > div + div {
    padding-left: 16px;
    border-left: 1px solid var(--line);
  }
  dd {
    margin: 6px 0 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    min-width: 0;
  }
  dd .v {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  dd.model {
    margin-top: 2px;
    padding-left: 14px;
    font-size: 11px;
    color: var(--text-3);
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  dd.none {
    color: var(--text-3);
  }
  .led {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-3);
    flex: none;
  }
  .led.on {
    background: var(--accent-fg);
    box-shadow: 0 0 6px var(--accent-line);
  }
  .setup {
    margin-top: 16px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 12px 11px 14px;
    border-radius: var(--r);
    border: 1px solid var(--accent-line);
    background: var(--accent-soft);
    color: var(--accent-fg);
  }
  .setup p {
    flex: 1;
    font-size: 13px;
    color: var(--text);
  }
  .try {
    margin-top: 26px;
    display: flex;
    align-items: center;
    gap: 26px;
    flex-wrap: wrap;
  }
  .hks {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12.5px;
  }
  .hk {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }

  .hist {
    margin-top: 40px;
  }
  .hist header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 30px;
    margin-bottom: 8px;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 9px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    color: var(--text-3);
    width: 210px;
  }
  .search:focus-within {
    border-color: var(--accent-line);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    font-size: 12.5px;
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--text-3);
  }
  .items {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    border-top: 1px solid var(--line);
  }
  li:last-child {
    border-bottom: 1px solid var(--line);
  }
  .item {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  .main {
    flex: 1;
    min-width: 0;
    text-align: left;
    padding: 12px 0 12px 2px;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--text-2);
    font-family: var(--mono);
  }
  .dot {
    color: var(--text-3);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .text {
    margin-top: 4px;
    font-size: 13.5px;
    line-height: 1.5;
    color: var(--text);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    user-select: text;
  }
  .open .text {
    -webkit-line-clamp: unset;
    line-clamp: unset;
  }
  .main:hover .text {
    color: var(--text);
  }
  .acts {
    display: flex;
    gap: 2px;
    padding-top: 10px;
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .item:hover .acts,
  .item:focus-within .acts,
  .open .acts {
    opacity: 1;
  }
  .detail {
    padding: 2px 0 14px 2px;
    animation: enter 160ms var(--ease) both;
  }
  .raw {
    margin-top: 4px;
    padding: 9px 11px;
    font-size: 13px;
    color: var(--text-2);
    border-left: 2px solid var(--line-strong);
    background: var(--surface);
    user-select: text;
  }
  .tech {
    margin-top: 10px;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    font-size: 11px;
    color: var(--text-3);
  }
  .warn {
    margin-top: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
  }
  .empty {
    padding: 36px 0 30px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }
  .empty-keys {
    margin-bottom: 18px;
  }
  .empty h3 {
    font-size: 16px;
  }
  .empty p {
    margin-top: 6px;
    max-width: 44ch;
    font-size: 13px;
  }
  .empty-line {
    padding: 16px 0;
    border-top: 1px solid var(--line);
    font-size: 13px;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
