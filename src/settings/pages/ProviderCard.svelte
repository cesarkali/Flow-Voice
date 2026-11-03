<script lang="ts">
  import { slide } from "svelte/transition";
  import { api } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import ProviderLogo from "../../lib/icons/ProviderLogo.svelte";
  import { app } from "../../lib/stores.svelte";
  import type { KeyProviderId, LlmProviderId, TestResult } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import { pname, type ProviderInfo } from "../providers";
  import ProviderKeys from "./ProviderKeys.svelte";
  import ProvMark from "./ProvMark.svelte";

  let { info, open = $bindable(false) }: { info: ProviderInfo; open?: boolean } = $props();

  const id = $derived(info.id);
  const keyId = $derived(id as KeyProviderId);
  const status = $derived(info.keyed ? app.keys?.[keyId] : undefined);
  const count = $derived(status?.count ?? 0);
  const prov = $derived(info.id !== "tavily" ? app.settings!.providers[info.id as LlmProviderId] : undefined);

  let howto = $state(false);
  let testing = $state<string | null>(null);
  let results = $state<Record<string, TestResult & { kind: string }>>({});
  let fetched = $state<string[] | null>(null);
  let fetching = $state(false);
  let fetchErr = $state("");
  let cmdCopied = $state(false);

  const statusText = $derived(
    !info.keyed ? t("prov.status_local") : count === 0 ? t("prov.status_none") : count === 1 ? t("prov.status_one") : t("prov.status_many", { n: count }),
  );
  const configured = $derived(!info.keyed || count > 0);

  const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const dur = (ms: number) => (reduce ? 0 : ms);
  const easeOut = (x: number) => 1 - Math.pow(1 - x, 4);

  // Immediately test a newly added key (that one, not the first saved) so the user knows it works.
  function testAfterAdd(key: string) {
    if (info.llm) runTest("llm", key);
    else if (id === "tavily") runTest("search", key);
    else if (info.stt) runTest("stt", key);
  }
  async function runTest(kind: "llm" | "stt" | "search", key?: string) {
    testing = kind;
    try {
      const r = await api.testProvider(id, kind, key);
      results = { ...results, [kind]: { ...r, kind } };
    } catch (e) {
      results = { ...results, [kind]: { ok: false, latency_ms: 0, model: "", message: String(e), kind } };
    } finally {
      testing = null;
    }
  }
  async function fetchModels() {
    fetching = true;
    fetchErr = "";
    try {
      fetched = await api.listProviderModels(id as LlmProviderId);
    } catch (e) {
      fetchErr = String(e);
    } finally {
      fetching = false;
    }
  }
  async function copyCmd(text: string) {
    await api.copyText(text);
    cmdCopied = true;
    setTimeout(() => (cmdCopied = false), 1400);
  }
  const suggestions = $derived([...new Set([...(fetched ?? []), ...info.modelSuggestions])]);
  const kinds = $derived(
    [info.llm && "llm", info.stt && "stt", id === "tavily" && "search"].filter(Boolean) as ("llm" | "stt" | "search")[],
  );
  const testLabel = (k: string) => (k === "llm" ? t("prov.test_llm") : k === "stt" ? t("prov.test_stt") : t("prov.test_search"));
</script>

<article class="card" class:open class:configured id="prov-{id}">
  <button type="button" class="top" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="logo" aria-hidden="true">
      <ProviderLogo {id} size={28} />
      <span class="led" class:on={configured && info.keyed}></span>
    </span>
    <span class="title">
      <span class="name">{pname(id)}</span>
      {#each info.badges as b}
        <Badge tone={b.tone}>{t(b.key)}</Badge>
      {/each}
    </span>
    <span class="st mono" class:ok={configured && info.keyed}>
      {#key statusText}<span class="stx">{statusText}</span>{/key}
    </span>
    <span class="chev"><Icon name="chevronDown" size={16} /></span>
  </button>

  {#if open}
    <div class="clip" transition:slide={{ duration: dur(280), easing: easeOut }}>
    <div class="body stagger">
      <p class="desc">{t(info.desc)}</p>

      {#if info.baseUrl && prov}
        <label class="fld">
          <span class="label">{t("prov.base_url")}</span>
          <input
            class="field mono"
            bind:value={prov.base_url}
            placeholder={id === "ollama" ? "http://localhost:11434/v1" : "https://api.example.com/v1"}
            spellcheck="false"
          />
        </label>
      {/if}

      {#if info.keyed}
        <div class="fld">
          <span class="label">{t("prov.key")}</span>
          <ProviderKeys provider={keyId} keyPrefix={info.keyPrefix} onadded={testAfterAdd} onremoved={(left) => left === 0 && (results = {})} />
        </div>
      {/if}

      {#if prov && (info.llm || info.stt)}
        <div class="models">
          {#if info.llm}
            <label class="fld">
              <span class="label">{t("prov.model")}</span>
              <span class="with-btn">
                <input class="field mono" list="ml-{id}" bind:value={prov.model} spellcheck="false" />
                {#if info.canListModels}
                  <Button variant="ghost" icon="refresh" loading={fetching} title={t("prov.fetch_models")} aria-label={t("prov.fetch_models")} onclick={fetchModels} />
                {/if}
              </span>
              <datalist id="ml-{id}">
                {#each suggestions as m}<option value={m}></option>{/each}
              </datalist>
              {#if fetched}<span class="hint">{t("prov.models_found", { n: fetched.length })}</span>{/if}
              {#if fetchErr}<span class="hint err mono">{fetchErr}</span>{/if}
            </label>
          {/if}
          {#if info.stt}
            <label class="fld">
              <span class="label">{t("prov.stt_model")}</span>
              <input class="field mono" list="sl-{id}" bind:value={prov.stt_model} spellcheck="false" />
              <datalist id="sl-{id}">
                {#each info.sttSuggestions as m}<option value={m}></option>{/each}
              </datalist>
            </label>
          {/if}
        </div>
      {/if}

      <div class="tests">
        {#each kinds as k}
          <Button size="sm" icon="play" loading={testing === k} disabled={!!testing && testing !== k} onclick={() => runTest(k)}>
            {testLabel(k)}
          </Button>
        {/each}
      </div>
      {#each kinds as k}
        {@const r = results[k]}
        {#if r}
          <div class="result" class:ok={r.ok} role="status">
            {#key r}<ProvMark ok={r.ok} size={16} />{/key}
            <div>
              <span class="what">{t(r.ok ? "sh.res_ok" : "sh.res_fail", { what: t(`sh.res_${k}`) })}</span>
              {#if r.ok}
                <span class="mono">{r.latency_ms} ms · {r.model}</span>
              {:else}
                <code class="mono">{r.message}</code>
              {/if}
            </div>
          </div>
        {/if}
      {/each}

      {#if info.note}
        <p class="note"><Icon name="info" size={14} />{t(info.note)}</p>
      {/if}

      <div class="howto" class:show={howto}>
        <button type="button" class="howbtn" aria-expanded={howto} onclick={() => (howto = !howto)}>
          <span class="hchev" class:on={howto}><Icon name="chevronRight" size={15} /></span>
          {id === "ollama" ? t("prov.how_to_install") : id === "custom" ? t("sh.how_to_setup") : t("prov.how_to")}
        </button>
        {#if howto}
          <div transition:slide={{ duration: dur(220), easing: easeOut }}>
          <ol>
            {#each info.steps as step, i}
              <li>
                <span class="n mono">{i + 1}</span>
                <div>
                  <p>{t(step)}</p>
                  {#if info.command && info.command.afterStep === i}
                    <div class="cmd">
                      <code class="mono">{info.command.text}</code>
                      <Button size="sm" variant="ghost" icon={cmdCopied ? "check" : "copy"} aria-label={t("c.copy")} onclick={() => copyCmd(info.command!.text)} />
                    </div>
                  {/if}
                </div>
              </li>
            {/each}
          </ol>
          {#if info.url}
            <div class="site">
              <Button size="sm" variant="primary" iconRight="external" onclick={() => api.openUrl(info.url)}>{t("c.open_site")}</Button>
              <span class="mono faint url">{info.url.replace(/^https?:\/\//, "")}</span>
            </div>
          {/if}
          </div>
        {/if}
      </div>
    </div>
    </div>
  {/if}
</article>

<style>
  .card {
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
    transition:
      border-color var(--t) var(--ease),
      box-shadow var(--t-slow) var(--ease);
  }
  .card:not(.open):hover {
    border-color: var(--line-strong);
  }
  .card + :global(.card) {
    margin-top: 8px;
  }
  .card.open {
    border-color: var(--line-strong);
    box-shadow: 0 10px 28px -18px rgba(0, 0, 0, 0.5);
  }
  .clip {
    overflow: hidden;
  }
  .top {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 14px;
    height: 52px;
    text-align: left;
    border-radius: 10px;
  }
  .top:hover .chev {
    color: var(--text);
  }
  .logo {
    position: relative;
    flex: none;
    display: grid;
    transition: transform var(--t-slow) var(--spring);
  }
  .top:hover .logo {
    transform: rotate(-5deg) scale(1.06);
  }
  .led {
    position: absolute;
    right: -3px;
    bottom: -3px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--surface-3);
    border: 2px solid var(--surface);
    box-sizing: border-box;
    flex: none;
    transform: scale(0);
    transition:
      background-color var(--t-slow) var(--ease),
      border-color var(--t-slow) var(--ease),
      box-shadow var(--t-slow) var(--ease),
      transform var(--t-slow) var(--spring);
  }
  .led.on {
    background: var(--ok);
    transform: scale(1);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .name {
    font-size: 14.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .st {
    display: inline-grid;
    height: 22px;
    padding: 0 9px;
    align-items: center;
    border-radius: 11px;
    border: 1px solid var(--line);
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    transition:
      color var(--t-slow) var(--ease),
      border-color var(--t-slow) var(--ease),
      background-color var(--t-slow) var(--ease);
  }
  .st.ok {
    color: var(--ok);
    border-color: var(--ok-line);
    background: var(--ok-soft);
  }
  .stx {
    grid-area: 1 / 1;
    animation: fv-rise var(--t-slow) var(--ease-out) both;
  }
  .chev {
    display: grid;
    color: var(--text-3);
    transition:
      transform var(--t-slow) var(--spring),
      color var(--t-fast) var(--ease);
  }
  .open .chev {
    transform: rotate(180deg);
  }
  .body {
    padding: 2px 16px 16px 54px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    --stagger: 28ms;
  }
  .desc {
    font-size: 13px;
    color: var(--text-2);
    max-width: 60ch;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .with-btn {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .with-btn .field {
    flex: 1;
    min-width: 0;
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  .hint.err {
    color: var(--danger-fg);
  }
  .models {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }
  .tests {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .result {
    display: flex;
    gap: 9px;
    align-items: flex-start;
    padding: 9px 11px;
    border-radius: var(--r);
    border: 1px solid var(--danger-line);
    background: var(--danger-soft);
    color: var(--danger-fg);
    font-size: 12.5px;
    transform-origin: top center;
    animation: enter 280ms var(--ease-out) both;
  }
  .result :global(.icon) {
    margin-top: 1px;
  }
  .result.ok {
    border-color: var(--ok-line);
    background: var(--ok-soft);
    color: var(--ok);
  }
  .result > div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .what {
    font-weight: 600;
  }
  .result .mono {
    color: var(--text);
    font-size: 11.5px;
  }
  .result code {
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .note :global(.icon) {
    margin-top: 2px;
  }
  .howto {
    border-top: 1px solid var(--line);
    padding-top: 10px;
  }
  .howbtn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    height: 26px;
    padding-right: 8px;
    border-radius: var(--r-sm);
  }
  .hchev {
    display: grid;
    transition: transform var(--t) var(--ease);
  }
  .hchev.on {
    transform: rotate(90deg);
  }
  .howbtn:hover {
    color: var(--accent-fg);
  }
  ol {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  ol li {
    display: flex;
    gap: 12px;
    font-size: 13px;
  }
  .n {
    width: 20px;
    height: 20px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    font-size: 10.5px;
    color: var(--text-2);
  }
  ol p {
    padding-top: 1px;
  }
  .cmd {
    margin-top: 6px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 4px 4px 11px;
    border-radius: var(--r-sm);
    background: var(--bg);
    border: 1px solid var(--line);
  }
  .cmd code {
    font-size: 12px;
    user-select: text;
  }
  .site {
    margin-top: 14px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .url {
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translate3d(0, -6px, 0) scale(0.985);
    }
  }
</style>
