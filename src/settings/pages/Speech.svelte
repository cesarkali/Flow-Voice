<script lang="ts">
  import { onMount } from "svelte";
  import { api, on } from "../../lib/api";
  import { langName, SPEECH_LANGS, t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app, hasKey } from "../../lib/stores.svelte";
  import { EVENTS, type LocalModel, type ModelProgress, type SettingsPage } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Progress from "../../lib/ui/Progress.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import { STT_PROVIDERS } from "../providers";

  let { onnavigate }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);

  let models = $state<LocalModel[]>([]);
  let progress = $state<Record<string, ModelProgress>>({});
  let devices = $state<string[]>([]);
  let confirmDel = $state<string | null>(null);

  async function loadModels() {
    models = await api.listLocalModels();
  }
  onMount(() => {
    loadModels();
    api.listInputDevices().then((d) => (devices = d));
    const un = on<ModelProgress>(EVENTS.modelProgress, (p) => {
      progress = { ...progress, [p.id]: p };
      if (p.done || p.error) loadModels();
    });
    return () => un.then((f) => f());
  });

  const installed = $derived(models.filter((m) => m.installed));
  const cloudReady = $derived(STT_PROVIDERS.some((p) => hasKey(p)));
  const localReady = $derived(models.some((m) => m.installed && m.id === s.stt.local_model));

  async function download(m: LocalModel) {
    progress = { ...progress, [m.id]: { id: m.id, downloaded: 0, total: m.size_mb * 1048576, done: false, error: null } };
    try {
      await api.downloadLocalModel(m.id);
    } catch (e) {
      progress = { ...progress, [m.id]: { ...progress[m.id], error: String(e) } };
    }
  }
  async function del(m: LocalModel) {
    if (confirmDel !== m.id) {
      confirmDel = m.id;
      setTimeout(() => confirmDel === m.id && (confirmDel = null), 3000);
      return;
    }
    confirmDel = null;
    await api.deleteLocalModel(m.id);
    loadModels();
  }
  const pct = (p?: ModelProgress) => (p && p.total ? p.downloaded / p.total : 0);
  const downloading = (id: string) => {
    const p = progress[id];
    return !!p && !p.done && !p.error;
  };
  const fmtSize = (mb: number) => (mb >= 1000 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`);
</script>

<header class="ph">
  <h1>{t("speech.title")}</h1>
  <p class="muted">{t("speech.subtitle")}</p>
</header>

<div class="engines" role="radiogroup" aria-label={t("speech.title")}>
  <button type="button" role="radio" aria-checked={s.stt.engine === "cloud"} class:sel={s.stt.engine === "cloud"} onclick={() => (s.stt.engine = "cloud")}>
    <span class="eh">
      <Icon name="cloud" size={22} />
      <span class="radio" aria-hidden="true"></span>
    </span>
    <span class="et">{t("speech.cloud_title")} <Badge tone="accent">{t("c.recommended")}</Badge></span>
    <span class="eb">{t("speech.cloud_body")}</span>
  </button>
  <button type="button" role="radio" aria-checked={s.stt.engine === "local"} class:sel={s.stt.engine === "local"} onclick={() => (s.stt.engine = "local")}>
    <span class="eh">
      <Icon name="lock" size={22} />
      <span class="radio" aria-hidden="true"></span>
    </span>
    <span class="et">{t("speech.local_title")}</span>
    <span class="eb">{t("speech.local_body")}</span>
  </button>
</div>

{#if s.stt.engine === "cloud" && !cloudReady}
  <p class="warn">
    <Icon name="alert" size={15} />
    <span>{t("speech.cloud_no_key")}</span>
    <Button size="sm" variant="ghost" iconRight="arrowRight" onclick={() => onnavigate?.("providers")}>{t("nav.providers")}</Button>
  </p>
{:else if s.stt.engine === "local" && models.length && !localReady}
  <p class="warn"><Icon name="alert" size={15} /><span>{t("speech.local_no_model")}</span></p>
{/if}

<Section label={t("nav.speech")}>
  <Row label={t("speech.language")} hint={t("speech.language_hint")}>
    <Select label={t("speech.language")} bind:value={s.speech_language} options={SPEECH_LANGS.map((l) => ({ value: l, label: langName(l) }))} />
  </Row>
  <Row label={t("speech.live")} hint={t("speech.live_hint")}>
    <Toggle label={t("speech.live")} bind:checked={s.stt.live_preview} />
  </Row>
  {#if s.stt.live_preview}
    <Row label={t("speech.preview_model")} hint={installed.length ? undefined : t("speech.preview_install")}>
      <Select
        label={t("speech.preview_model")}
        mono
        bind:value={s.stt.preview_model}
        options={models.map((m) => ({ value: m.id, label: m.label + (m.installed ? "" : " ↓"), disabled: !m.installed }))}
      />
    </Row>
  {/if}
  <Row label={t("speech.mic")}>
    <Select
      label={t("speech.mic")}
      width="260px"
      bind:value={s.input_device}
      options={[{ value: null, label: t("speech.mic_default") }, ...devices.map((d) => ({ value: d as string | null, label: d }))]}
    />
  </Row>
</Section>

<Section label={t("speech.models")}>
  <p class="hint">{t("speech.models_hint")}</p>
  <ul class="models">
    {#each models as m (m.id)}
      {@const p = progress[m.id]}
      {@const inUse = s.stt.local_model === m.id}
      <li class:inuse={inUse && m.installed}>
        <div class="mi">
          <div class="mn">
            <span class="ml">{m.label}</span>
            {#if m.recommended}<Badge tone="accent">{t("c.recommended")}</Badge>{/if}
          </div>
          <div class="mm mono">
            <span>{fmtSize(m.size_mb)}</span>
            <span>·</span>
            <span>{m.note}</span>
          </div>
          {#if downloading(m.id)}
            <div class="pg"><Progress value={pct(p)} label={m.label} /></div>
          {/if}
          {#if p?.error}<p class="perr mono">{p.error}</p>{/if}
        </div>
        <div class="ma">
          {#if downloading(m.id)}
            <span class="mono pct tnum">{t("speech.downloading", { pct: Math.round(pct(p) * 100) })}</span>
          {:else if m.installed}
            {#if inUse}
              <span class="using mono"><Icon name="check" size={14} stroke={2} />{t("speech.in_use")}</span>
            {:else}
              <Button size="sm" onclick={() => (s.stt.local_model = m.id)}>{t("speech.use")}</Button>
            {/if}
            <Button
              size="sm"
              variant={confirmDel === m.id ? "danger" : "ghost"}
              icon="trash"
              aria-label={t("speech.delete_model")}
              title={confirmDel === m.id ? t("c.confirm_again") : t("speech.delete_model")}
              onclick={() => del(m)}
            />
          {:else}
            <Button size="sm" icon="download" onclick={() => download(m)}>{t("speech.download")}</Button>
          {/if}
        </div>
      </li>
    {/each}
  </ul>
</Section>

<style>
  .ph h1 {
    font-size: 26px;
  }
  .ph p {
    margin-top: 6px;
    font-size: 13.5px;
  }
  .engines {
    margin-top: 22px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .engines button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 16px 16px 18px;
    text-align: left;
    border-radius: 12px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    transition:
      border-color var(--t-fast),
      background-color var(--t-fast);
  }
  .engines button:hover {
    border-color: var(--text-3);
  }
  .engines button.sel {
    border-color: var(--accent-line);
    background: linear-gradient(var(--accent-soft), var(--accent-soft)), var(--surface);
  }
  .eh {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-2);
    margin-bottom: 6px;
  }
  .sel .eh {
    color: var(--accent-fg);
  }
  .radio {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    display: grid;
    place-items: center;
  }
  .radio::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    transform: scale(0);
    transition: transform var(--t) var(--spring);
  }
  .sel .radio {
    border-color: var(--accent);
  }
  .sel .radio::after {
    transform: scale(1);
  }
  .et {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .eb {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .warn {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    color: var(--danger-fg);
  }
  .warn span {
    flex: 1;
  }
  .hint {
    font-size: 12.5px;
    color: var(--text-2);
    margin-bottom: 12px;
  }
  .models {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .models li {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-top: 1px solid var(--line);
  }
  .models li:last-child {
    border-bottom: 1px solid var(--line);
  }
  .mi {
    flex: 1;
    min-width: 0;
  }
  .mn {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ml {
    font-size: 13.5px;
    font-weight: 500;
  }
  .mm {
    margin-top: 3px;
    display: flex;
    gap: 6px;
    font-size: 11px;
    color: var(--text-3);
  }
  .pg {
    margin-top: 8px;
    max-width: 320px;
  }
  .perr {
    margin-top: 6px;
    font-size: 11px;
    color: var(--danger-fg);
  }
  .ma {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pct {
    font-size: 12px;
    color: var(--accent-fg);
    min-width: 40px;
    text-align: right;
  }
  .using {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--accent-fg);
    margin-right: 4px;
  }
</style>
