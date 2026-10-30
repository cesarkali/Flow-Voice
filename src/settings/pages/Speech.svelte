<script lang="ts">
  import { onMount } from "svelte";
  import { slide } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { api, on } from "../../lib/api";
  import { i18n, langName, locale, SPEECH_LANGS, t, type TKey } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import type { IconName } from "../../lib/icons/paths";
  import { app, hasKey } from "../../lib/stores.svelte";
  import { EVENTS, type LocalModel, type ModelProgress, type SettingsPage } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Flag from "../../lib/ui/Flag.svelte";
  import Progress from "../../lib/ui/Progress.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import { STT_PROVIDERS } from "../providers";

  let { onnavigate }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);

  /** Model used for the live preview; Rust downloads it on its own when live_preview is on. */
  const PREVIEW_ID = "base-q5_1";

  let models = $state<LocalModel[]>([]);
  let progress = $state<Record<string, ModelProgress>>({});
  let devices = $state<string[]>([]);
  let confirmDel = $state<string | null>(null);
  let loaded = $state(false);

  async function loadModels() {
    models = await api.listLocalModels();
    loaded = true;
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

  // ---------------------------------------------------------------- languages
  const nativeName = (code: string) => {
    const c = code === "jw" ? "jv" : code;
    try {
      const n = new Intl.DisplayNames([c], { type: "language" }).of(c) ?? "";
      return n.charAt(0).toLocaleUpperCase(c) + n.slice(1);
    } catch {
      return "";
    }
  };
  const nameOf = (code: string) => langName(code === "jw" ? "jv" : code);

  const langOptions = $derived.by(() => {
    void i18n.lang;
    const suggested = [...new Set(["auto", i18n.lang, "pt", "en", "es"])].filter((c) => SPEECH_LANGS.includes(c));
    const mk = (code: string, group: string) => {
      const label = nameOf(code);
      const native = code === "auto" ? "" : nativeName(code);
      return { value: code, label, hint: native && native !== label ? native : undefined, group };
    };
    const g1 = t("ctl.lang_suggested");
    const g2 = t("ctl.lang_all");
    const rest = SPEECH_LANGS.filter((c) => !suggested.includes(c))
      .map((c) => mk(c, g2))
      .sort((a, b) => a.label.localeCompare(b.label, locale()));
    return [...suggested.map((c) => mk(c, g1)), ...rest];
  });

  const micOptions = $derived([
    { value: null as string | null, label: t("speech.mic_default"), hint: t("ctl.mic_default_hint") },
    ...devices.map((d) => ({ value: d as string | null, label: d })),
  ]);

  // ---------------------------------------------------------------- engines
  const cloudReady = $derived(STT_PROVIDERS.some((p) => hasKey(p)));
  // Rust falls back to another installed model when the configured one is gone.
  const localReady = $derived(models.some((m) => m.installed && m.id === s.stt.local_model) || models.some((m) => m.installed && m.langs.includes("*")));

  const ENGINES: { id: "cloud" | "local"; icon: IconName; title: TKey; body: TKey; feats: TKey[] }[] = [
    { id: "cloud", icon: "cloud", title: "speech.cloud_title", body: "speech.cloud_body", feats: ["ctl.cloud_f1", "ctl.cloud_f2", "ctl.cloud_f3"] },
    { id: "local", icon: "lock", title: "speech.local_title", body: "speech.local_body", feats: ["ctl.local_f1", "ctl.local_f2", "ctl.local_f3"] },
  ];
  function engineKey(e: KeyboardEvent) {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) return;
    e.preventDefault();
    s.stt.engine = s.stt.engine === "cloud" ? "local" : "cloud";
    (e.currentTarget as HTMLElement).querySelector<HTMLButtonElement>(`[data-engine="${s.stt.engine}"]`)?.focus();
  }

  // ---------------------------------------------------------------- models
  /** Friendly names/descriptions; unknown ids fall back to the Rust label/note. */
  const META: Record<string, { name: TKey; desc: TKey }> = {
    "tiny-q5_1": { name: "ctl.m_tiny", desc: "ctl.m_tiny_d" },
    "base-q5_1": { name: "ctl.m_base", desc: "ctl.m_base_d" },
    "base-en-q5_1": { name: "lm.m_base_en", desc: "lm.m_base_en_d" },
    "small-q5_1": { name: "ctl.m_small", desc: "ctl.m_small_d" },
    "small-en-q5_1": { name: "lm.m_small_en", desc: "lm.m_small_en_d" },
    "distil-ptbr-q5_0": { name: "lm.m_ptbr", desc: "lm.m_ptbr_d" },
    "medium-q5_0": { name: "ctl.m_medium", desc: "ctl.m_medium_d" },
    "turbo-q5_0": { name: "ctl.m_turbo", desc: "ctl.m_turbo_d" },
  };
  const nameOfModel = (m: LocalModel) => (META[m.id] ? t(META[m.id].name) : m.label);
  const descOf = (m: LocalModel) => (META[m.id] ? t(META[m.id].desc) : m.note);

  // Which language the list is tuned for: follows "idioma que eu falo" until the user picks another here.
  let pickedLang = $state<string | null>(null);
  const modelLang = $derived(pickedLang ?? s.speech_language ?? "auto");
  const supports = (m: LocalModel, l: string) => m.langs.includes("*") || (l !== "auto" && m.langs.includes(l));
  const recFor = (m: LocalModel, l: string) =>
    supports(m, l) && (l === "auto" ? m.recommended_for.includes("*") : m.recommended_for.includes("*") || m.recommended_for.includes(l));

  let showOther = $state(false);
  const groups = $derived.by(() => {
    const l = modelLang;
    const best = (a: LocalModel, b: LocalModel) => b.quality - a.quality || b.speed - a.speed || a.size_mb - b.size_mb;
    const light = (a: LocalModel, b: LocalModel) => a.size_mb - b.size_mb || b.quality - a.quality;
    const rec = models.filter((m) => recFor(m, l)).sort(best);
    const rest = models.filter((m) => supports(m, l) && !recFor(m, l)).sort(light);
    const other = models.filter((m) => !supports(m, l)).sort(light);
    return { rec, rest, other };
  });
  /** Other-language models: installed ones stay visible (so they can be deleted), the rest on demand. */
  const otherShown = $derived(showOther ? groups.other : groups.other.filter((m) => m.installed || downloading(m.id)));
  const otherHidden = $derived(groups.other.length - otherShown.length);
  const langLabel = (l: string) => nameOf(l);
  const onlyLangs = (m: LocalModel) => (m.langs.includes("*") ? null : m.langs);

  async function download(m: LocalModel) {
    progress = { ...progress, [m.id]: { id: m.id, downloaded: 0, total: m.size_mb * 1048576, done: false, error: null } };
    try {
      await api.downloadLocalModel(m.id);
    } catch (e) {
      progress = { ...progress, [m.id]: { ...progress[m.id], error: String(e) } };
    }
  }

  /** Where the offline engine goes if `m` disappears: best other installed model for the speech language. */
  function replacementFor(m: LocalModel): LocalModel | null {
    const l = s.speech_language;
    const left = models.filter((x) => x.installed && x.id !== m.id && supports(x, l === "auto" ? "*" : l));
    left.sort((a, b) => Number(recFor(b, l)) - Number(recFor(a, l)) || b.quality - a.quality || b.speed - a.speed);
    return left[0] ?? null;
  }
  async function del(m: LocalModel) {
    confirmDel = null;
    const wasInUse = s.stt.local_model === m.id;
    const next = replacementFor(m);
    await api.deleteLocalModel(m.id);
    if (wasInUse) {
      if (next) s.stt.local_model = next.id;
      else if (s.stt.engine === "local" && cloudReady) s.stt.engine = "cloud";
    }
    loadModels();
  }
  const pct = (p?: ModelProgress) => (p && p.total ? p.downloaded / p.total : 0);
  const downloading = (id: string) => {
    const p = progress[id];
    return !!p && !p.done && !p.error;
  };
  const fmtSize = (mb: number) => (mb >= 1000 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`);
  const mbOf = (p: ModelProgress) => t("ctl.mb_of", { a: fmtSize(p.downloaded / 1048576), b: fmtSize(p.total / 1048576) });

  const previewModel = $derived(models.find((m) => m.id === PREVIEW_ID));
  function setLive(on: boolean) {
    if (on) s.stt.preview_model = PREVIEW_ID;
  }
</script>

<header class="ph">
  <h1>{t("speech.title")}</h1>
  <p class="muted">{t("speech.subtitle")}</p>
</header>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="engines" role="radiogroup" aria-label={t("speech.title")} onkeydown={engineKey}>
  {#each ENGINES as e (e.id)}
    {@const sel = s.stt.engine === e.id}
    <button
      type="button"
      role="radio"
      data-engine={e.id}
      aria-checked={sel}
      tabindex={sel ? 0 : -1}
      class:sel
      onclick={() => (s.stt.engine = e.id)}
    >
      <span class="eh">
        <span class="eicon"><Icon name={e.icon} size={20} /></span>
        <span class="radio" aria-hidden="true"></span>
      </span>
      <span class="et">
        {t(e.title)}
        {#if e.id === "cloud"}<Badge tone="accent">{t("c.recommended")}</Badge>{/if}
      </span>
      <span class="eb">{t(e.body)}</span>
      <ul class="feats">
        {#each e.feats as f, i (f)}
          <li style="--i:{i}">
            <Icon name={i === 2 ? "info" : "check"} size={13} stroke={i === 2 ? 1.5 : 2} />
            {t(f)}
          </li>
        {/each}
      </ul>
    </button>
  {/each}
</div>

{#if s.stt.engine === "cloud" && !cloudReady}
  <p class="warn" transition:slide={{ duration: 200, easing: cubicOut }}>
    <Icon name="alert" size={15} />
    <span>{t("speech.cloud_no_key")}</span>
    <Button size="sm" variant="ghost" iconRight="arrowRight" onclick={() => onnavigate?.("providers")}>{t("nav.providers")}</Button>
  </p>
{:else if s.stt.engine === "local" && loaded && !localReady}
  <p class="warn" transition:slide={{ duration: 200, easing: cubicOut }}>
    <Icon name="alert" size={15} /><span>{t("speech.local_no_model")}</span>
  </p>
{/if}

<Section label={t("nav.speech")}>
  <Row id="set-speech-language" label={t("speech.language")} hint={t("speech.language_hint")}>
    <Select label={t("speech.language")} width="240px" bind:value={s.speech_language} options={langOptions} />
  </Row>
  <Row id="set-speech-mic" label={t("speech.mic")}>
    <Select label={t("speech.mic")} width="260px" bind:value={s.input_device} options={micOptions} />
  </Row>
  <Row id="set-speech-live" label={t("speech.live")} hint={t("ctl.live_hint")}>
    <Toggle label={t("speech.live")} bind:checked={s.stt.live_preview} onchange={setLive} />
    {#snippet below()}
      {#if s.stt.live_preview && previewModel}
        {@const p = progress[PREVIEW_ID]}
        <div class="live" transition:slide={{ duration: 220, easing: cubicOut }}>
          {#if downloading(PREVIEW_ID)}
            <div class="lrow">
              <span class="mono tnum">{t("ctl.live_downloading", { pct: Math.round(pct(p) * 100) })}</span>
            </div>
            <Progress value={pct(p)} label={t("ctl.m_base")} />
          {:else if previewModel.installed}
            <span class="lok"><Icon name="check" size={13} stroke={2} />{t("ctl.live_ready")}</span>
          {:else}
            <span class="lpend"><Icon name="download" size={13} />{t("ctl.live_pending")}</span>
          {/if}
        </div>
      {/if}
    {/snippet}
  </Row>
</Section>

<Section label={t("speech.models")}>
  <div class="mhead">
    <p class="hint">{t("ctl.models_hint")}</p>
    <div class="mlang">
      <span class="mlab">{t("lm.for_lang")}</span>
      <Select label={t("lm.for_lang")} width="200px" value={modelLang} onchange={(v: string) => (pickedLang = v)} options={langOptions} />
    </div>
  </div>
  <p class="unload"><Icon name="chip" size={14} /><span>{t("lm.unload_note")}</span></p>

  {#key modelLang}
    <div class="groups">
      {#if groups.rec.length}
        <h3 class="gl rec"><Icon name="sparkle" size={13} />{modelLang === "auto" ? t("lm.rec_auto") : t("lm.rec_for", { lang: langLabel(modelLang) })}</h3>
        {@render list(groups.rec, 0, true)}
      {/if}
      {#if groups.rest.length}
        <h3 class="gl">{t("lm.others")}</h3>
        {@render list(groups.rest, groups.rec.length, false)}
      {/if}
      {#if groups.other.length}
        {#if otherShown.length}
          <h3 class="gl">{t("lm.other_langs")}</h3>
          {@render list(otherShown, 0, false, true)}
        {/if}
        {#if otherHidden > 0 || showOther}
          <button type="button" class="more" aria-expanded={showOther} onclick={() => (showOther = !showOther)}>
            <Icon name={showOther ? "chevronUp" : "chevronDown"} size={14} />
            {showOther ? t("lm.hide_other") : t("lm.show_other", { n: otherHidden })}
          </button>
        {/if}
      {/if}
    </div>
  {/key}
</Section>

{#snippet list(items: LocalModel[], offset: number, rec: boolean, foreign = false)}
  <ul class="models">
    {#each items as m, i (m.id)}
      {@const p = progress[m.id]}
      {@const inUse = s.stt.local_model === m.id && m.installed}
      {@const busy = downloading(m.id)}
      {@const only = onlyLangs(m)}
      {@const asking = confirmDel === m.id}
      <li
        class:inuse={inUse}
        class:active={inUse && s.stt.engine === "local"}
        class:installed={m.installed}
        class:foreign
        class:asking
        style="--i:{offset + i}"
      >
        <div class="mrow">
          <div class="mi">
            <div class="mn">
              <span class="ml">{nameOfModel(m)}</span>
              {#if inUse}
                <Badge tone={s.stt.engine === "local" ? "solid" : "neutral"}>{t("speech.in_use")}</Badge>
              {/if}
              {#if rec}<Badge tone="accent">{t("c.recommended")}</Badge>{/if}
              {#if only}
                <span class="only">
                  {#each only as l (l)}<Flag code={l} size={14} />{/each}
                  {t("lm.only", { lang: only.map((l) => (i18n.lang === "en" ? langLabel(l) : langLabel(l).toLocaleLowerCase(locale()))).join(", ") })}
                </span>
              {/if}
              {#if m.id === PREVIEW_ID && s.stt.live_preview}<Badge>{t("ctl.preview_badge")}</Badge>{/if}
            </div>
            <p class="md">{descOf(m)}</p>
            <div class="meters">
              {@render meter(t("ctl.speed"), m.speed)}
              {@render meter(t("ctl.accuracy"), m.quality)}
              {#if m.ram_mb >= 800}
                <span class="heavy"><Icon name="chip" size={13} />{t("ctl.strong_pc")}</span>
              {/if}
            </div>
            <div class="tech mono">
              <span class="tnum">{t("lm.disk", { size: fmtSize(m.size_mb) })}</span>
              <span aria-hidden="true">·</span>
              <span class="tnum">{t("lm.ram", { size: fmtSize(m.ram_mb) })}</span>
              <span aria-hidden="true">·</span>
              <span class="tt">{m.tech || m.file}</span>
            </div>
            {#if busy && p}
              <div class="pg" transition:slide={{ duration: 200, easing: cubicOut }}>
                <Progress value={pct(p)} label={nameOfModel(m)} />
                <span class="mono tnum">{mbOf(p)}</span>
              </div>
            {/if}
            {#if p?.error}<p class="perr mono">{p.error}</p>{/if}
          </div>
          <div class="ma">
            {#if busy}
              <span class="mono pct tnum">{t("speech.downloading", { pct: Math.round(pct(p) * 100) })}</span>
            {:else if m.installed}
              {#if !inUse && !foreign}
                <Button size="sm" onclick={() => (s.stt.local_model = m.id)}>{t("speech.use")}</Button>
              {/if}
              <Button
                size="sm"
                variant={asking ? "danger" : "ghost"}
                icon="trash"
                aria-label={t("speech.delete_model")}
                aria-expanded={asking}
                title={t("speech.delete_model")}
                onclick={() => (confirmDel = asking ? null : m.id)}
              />
            {:else}
              <Button size="sm" icon="download" onclick={() => download(m)}>
                {t("speech.download")}<span class="dsize tnum">{fmtSize(m.size_mb)}</span>
              </Button>
            {/if}
          </div>
        </div>
        {#if asking}
          {@const next = inUse ? replacementFor(m) : null}
          <div class="confirm" role="alertdialog" aria-label={t("speech.delete_model")} transition:slide={{ duration: 220, easing: cubicOut }}>
            <div class="cq">
              <Icon name="trash" size={15} />
              <div>
                <p class="cqt">{t("lm.del_q", { name: nameOfModel(m), size: fmtSize(m.size_mb) })}</p>
                {#if inUse}
                  <p class="cqs">
                    {next
                      ? t("lm.del_inuse", { name: nameOfModel(next) })
                      : s.stt.engine === "local" && cloudReady
                        ? t("lm.del_inuse_cloud")
                        : t("lm.del_inuse_none")}
                  </p>
                {/if}
                {#if m.id === PREVIEW_ID && s.stt.live_preview}<p class="cqs">{t("lm.del_preview")}</p>{/if}
              </div>
            </div>
            <div class="cb">
              <Button size="sm" variant="ghost" onclick={() => (confirmDel = null)}>{t("c.cancel")}</Button>
              <Button size="sm" variant="danger" icon="trash" onclick={() => del(m)}>{t("lm.del_yes")}</Button>
            </div>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet meter(label: string, n: number)}
  <span class="meter" aria-label="{label}: {n}/5">
    <span class="mlab">{label}</span>
    <span class="bars" aria-hidden="true">
      {#each [1, 2, 3, 4, 5] as k (k)}
        <span class="bar" class:on={k <= n} style="--k:{k}"></span>
      {/each}
    </span>
  </span>
{/snippet}

<style>
  .ph h1 {
    font-size: 26px;
  }
  .ph p {
    margin-top: 6px;
    font-size: 13.5px;
  }

  /* ---------------------------------------------------------------- engines */
  .engines {
    margin-top: 22px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .engines button {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 16px 16px 16px;
    text-align: left;
    border-radius: 12px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    overflow: hidden;
    transition:
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease),
      transform 160ms var(--spring);
  }
  /* accent wash that grows from the radio corner on select */
  .engines button::before {
    content: "";
    position: absolute;
    inset: 0;
    background: radial-gradient(120% 140% at 100% 0%, var(--accent-soft), transparent 70%);
    opacity: 0;
    transform: scale(0.6);
    transform-origin: 100% 0;
    transition:
      opacity 320ms var(--ease),
      transform 420ms var(--ease);
    pointer-events: none;
  }
  .engines button > * {
    position: relative;
  }
  .engines button:hover {
    border-color: var(--text-3);
  }
  .engines button:active {
    transform: scale(0.985);
  }
  .engines button.sel {
    border-color: var(--accent-line);
  }
  .engines button.sel::before {
    opacity: 1;
    transform: none;
  }
  .eh {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-2);
    margin-bottom: 4px;
  }
  .eicon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 9px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    transition:
      color var(--t) var(--ease),
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease),
      transform 300ms var(--spring);
  }
  .sel .eicon {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
    transform: rotate(-6deg);
  }
  .radio {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    display: grid;
    place-items: center;
    transition: border-color var(--t) var(--ease);
  }
  .radio::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-fg);
    transform: scale(0);
    transition: transform 260ms var(--spring);
  }
  .sel .radio {
    border-color: var(--accent-fg);
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
  .feats {
    list-style: none;
    margin: 6px 0 0;
    padding: 10px 0 0;
    border-top: 1px dashed var(--line-strong);
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .feats li {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--text-2);
  }
  .feats li :global(svg) {
    flex: none;
    color: var(--text-3);
    transition: color var(--t) var(--ease);
    transition-delay: calc(var(--i) * 50ms);
  }
  .sel .feats li:not(:last-child) :global(svg) {
    color: var(--accent-fg);
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

  /* ---------------------------------------------------------------- live preview status */
  .live {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 360px;
    font-size: 12px;
  }
  .lrow {
    color: var(--accent-fg);
  }
  .lok,
  .lpend {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
  }
  .lok {
    color: var(--ok);
  }

  /* ---------------------------------------------------------------- models */
  .hint {
    font-size: 12.5px;
    color: var(--text-2);
    margin-bottom: 12px;
  }
  .mhead {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px 20px;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }
  .mhead .hint {
    flex: 1 1 280px;
    margin: 4px 0 0;
    max-width: 60ch;
  }
  .mlang {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .unload {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
    padding: 8px 11px;
    border-radius: 8px;
    border: 1px dashed var(--line-strong);
    font-size: 12px;
    color: var(--text-3);
  }
  .unload :global(svg) {
    flex: none;
  }
  .groups {
    display: flex;
    flex-direction: column;
    animation: card-in 300ms var(--ease) both;
  }
  .gl {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 18px 0 9px;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .gl:first-child {
    margin-top: 2px;
  }
  .gl.rec {
    color: var(--accent-fg);
  }
  .more {
    align-self: flex-start;
    margin-top: 12px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px 6px 8px;
    border-radius: 7px;
    font-size: 12.5px;
    color: var(--text-2);
    transition:
      background-color var(--t) var(--ease),
      color var(--t) var(--ease);
  }
  .more:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .models {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .models li {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 14px 14px 14px 16px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
    animation: card-in 380ms var(--ease) both;
    animation-delay: calc(var(--i) * 45ms);
    transition:
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease);
  }
  .models li:hover {
    border-color: var(--line-strong);
  }
  .models li.active {
    border-color: var(--accent-line);
  }
  .models li.asking {
    border-color: var(--danger-line);
  }
  .models li.foreign:not(.installed) {
    background: transparent;
  }
  .models li.foreign .ml {
    color: var(--text-2);
  }
  .mrow {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .only {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .only :global(svg) {
    border-radius: 2px;
  }
  .confirm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 16px;
  }
  .confirm::before {
    content: "";
    flex-basis: 100%;
    height: 0;
    margin-top: 12px;
    border-top: 1px dashed var(--line-strong);
    margin-bottom: 2px;
  }
  .cq {
    display: flex;
    gap: 9px;
    flex: 1 1 260px;
    color: var(--danger-fg);
  }
  .cq :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .cqt {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }
  .cqs {
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-2);
  }
  .cb {
    display: flex;
    gap: 6px;
    margin-left: auto;
  }
  /* accent spine on the model in use */
  .models li::before {
    content: "";
    position: absolute;
    left: -1px;
    top: 12px;
    bottom: 12px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent-fg);
    transform: scaleY(0);
    transition: transform 320ms var(--spring);
  }
  .models li.active::before {
    transform: none;
  }
  @keyframes card-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  .mi {
    flex: 1;
    min-width: 0;
  }
  .mn {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .ml {
    font-size: 14.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .md {
    margin-top: 3px;
    font-size: 12.5px;
    color: var(--text-2);
    max-width: 56ch;
  }
  .meters {
    margin-top: 10px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 18px;
  }
  .meter {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .mlab {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .bars {
    display: inline-flex;
    gap: 2px;
  }
  .bar {
    width: 10px;
    height: 5px;
    border-radius: 1.5px;
    background: var(--surface-3);
  }
  .bar.on {
    background: var(--text);
    animation: bar-in 320ms var(--spring) both;
    animation-delay: calc(120ms + var(--i) * 45ms + var(--k) * 35ms);
  }
  .active .bar.on {
    background: var(--accent-fg);
  }
  @keyframes bar-in {
    from {
      transform: scaleX(0.2);
      opacity: 0;
    }
  }
  .heavy {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .tech {
    margin-top: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 6px;
    font-size: 10.5px;
    color: var(--text-3);
    opacity: 0.85;
  }
  .pg {
    margin-top: 10px;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 11px;
    color: var(--text-3);
  }
  .dsize {
    margin-left: 7px;
    padding-left: 7px;
    border-left: 1px solid var(--line-strong);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 400;
    color: var(--text-3);
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
    flex: none;
  }
  .pct {
    font-size: 13px;
    color: var(--accent-fg);
    min-width: 44px;
    text-align: right;
  }

  @media (max-width: 620px) {
    .engines {
      grid-template-columns: 1fr;
    }
  }
</style>
