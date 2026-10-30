<script lang="ts">
  import { api } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app, setKeys } from "../../lib/stores.svelte";
  import type { KeyProviderId, LlmProviderId, TestResult } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import { pname, type ProviderInfo } from "../providers";

  let { info, open = $bindable(false) }: { info: ProviderInfo; open?: boolean } = $props();

  const id = $derived(info.id);
  const keyId = $derived(id as KeyProviderId);
  const status = $derived(info.keyed ? app.keys?.[keyId] : undefined);
  const count = $derived(status?.count ?? 0);
  const prov = $derived(info.id !== "tavily" ? app.settings!.providers[info.id as LlmProviderId] : undefined);

  let draft = $state("");
  let revealed = $state(false);
  let saving = $state(false);
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

  async function save() {
    if (!draft.trim() && !revealed) return;
    saving = true;
    try {
      await setKeys(keyId, draft.trim());
      draft = "";
      revealed = false;
      // Immediately test so the user knows it works.
      if (info.llm) runTest("llm");
      else if (id === "tavily") runTest("search");
      else if (info.stt) runTest("stt");
    } finally {
      saving = false;
    }
  }
  async function removeAll() {
    await setKeys(keyId, "");
    draft = "";
    revealed = false;
    results = {};
  }
  async function toggleReveal() {
    if (revealed) {
      revealed = false;
      draft = "";
      return;
    }
    if (count > 0) draft = await api.revealKeys(keyId);
    revealed = true;
  }
  async function runTest(kind: "llm" | "stt" | "search") {
    testing = kind;
    try {
      const r = await api.testProvider(id, kind);
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
    <span class="led" class:on={configured && info.keyed} aria-hidden="true"></span>
    <span class="title">
      <span class="name">{pname(id)}</span>
      {#each info.badges as b}
        <Badge tone={b.tone}>{t(b.key)}</Badge>
      {/each}
    </span>
    <span class="st mono" class:ok={configured && info.keyed}>{statusText}</span>
    <span class="chev"><Icon name="chevronDown" size={16} /></span>
  </button>

  {#if open}
    <div class="body">
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
          <label class="label" for="key-{id}">{t("prov.key")}</label>
          <div class="keyrow">
            <input
              id="key-{id}"
              class="field mono"
              type={revealed ? "text" : "password"}
              autocomplete="off"
              spellcheck="false"
              bind:value={draft}
              placeholder={count ? t("prov.key_replace_placeholder") : `${t("prov.key_placeholder")}${info.keyPrefix ? ` (${info.keyPrefix})` : ""}`}
              onkeydown={(e) => e.key === "Enter" && save()}
            />
            <Button
              variant="ghost"
              icon={revealed ? "eyeOff" : "eye"}
              aria-label={revealed ? t("prov.hide") : t("prov.reveal")}
              title={revealed ? t("prov.hide") : t("prov.reveal")}
              onclick={toggleReveal}
            />
            <Button variant={draft.trim() ? "primary" : "default"} loading={saving} disabled={!draft.trim() && !revealed} onclick={save}>
              {t("prov.save_key")}
            </Button>
          </div>
          {#if count}
            <div class="masked">
              {#each status?.masked ?? [] as m}
                <span class="chip mono"><Icon name="key" size={12} />{m}</span>
              {/each}
              <button type="button" class="link danger" onclick={removeAll}>{t("prov.remove_keys")}</button>
            </div>
          {/if}
          <p class="hint">{t("prov.keys_hint")}</p>
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
            <Icon name={r.ok ? "check" : "alert"} size={15} stroke={r.ok ? 2 : 1.5} />
            <div>
              <span class="what">{testLabel(k)}</span>
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
          <Icon name={howto ? "chevronDown" : "chevronRight"} size={15} />
          {id === "ollama" ? t("prov.how_to_install") : id === "custom" ? t("prov.custom.name") : t("prov.how_to")}
        </button>
        {#if howto}
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
        {/if}
      </div>
    </div>
  {/if}
</article>

<style>
  .card {
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
    transition: border-color var(--t-fast);
  }
  .card + :global(.card) {
    margin-top: 8px;
  }
  .card.open {
    border-color: var(--line-strong);
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
  .led {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--surface-3);
    border: 1px solid var(--line-strong);
    flex: none;
  }
  .led.on {
    background: var(--accent-fg);
    border-color: var(--accent-fg);
    box-shadow: 0 0 6px var(--accent-line);
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
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .st.ok {
    color: var(--accent-fg);
  }
  .chev {
    color: var(--text-3);
    transition: transform var(--t) var(--ease);
  }
  .open .chev {
    transform: rotate(180deg);
  }
  .body {
    padding: 2px 16px 16px 33px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    animation: enter 180ms var(--ease) both;
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
  .keyrow,
  .with-btn {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .keyrow .field,
  .with-btn .field {
    flex: 1;
    min-width: 0;
  }
  .masked {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
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
  }
  .link {
    font-size: 12px;
    color: var(--text-3);
    padding: 0 6px;
    height: 24px;
    border-radius: 5px;
  }
  .link:hover {
    color: var(--text);
    background: var(--hover);
  }
  .link.danger:hover {
    color: var(--danger-fg);
    background: var(--danger-soft);
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
    animation: enter 160ms var(--ease) both;
  }
  .result :global(.icon) {
    margin-top: 1px;
  }
  .result.ok {
    border-color: var(--accent-line);
    background: var(--accent-soft);
    color: var(--accent-fg);
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
      transform: translateY(-3px);
    }
  }
</style>
