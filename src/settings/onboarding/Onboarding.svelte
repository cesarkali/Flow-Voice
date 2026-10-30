<script lang="ts">
  import AccentPicker from "../../lib/ui/AccentPicker.svelte";
  import LangPicker from "../../lib/ui/LangPicker.svelte";
  import { onMount } from "svelte";
  import type { TransitionConfig } from "svelte/transition";
  import { api, isTauri, on } from "../../lib/api";
  import { t, type TKey } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import Logo from "../../lib/icons/Logo.svelte";
  import type { IconName } from "../../lib/icons/paths";
  import { app, completeOnboarding } from "../../lib/stores.svelte";
  import { EVENTS, type KeyProviderId, type LlmProviderId, type LocalModel, type ModelProgress, type SttProviderId } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import HotkeyInput from "../../lib/ui/HotkeyInput.svelte";
  import Keys from "../../lib/ui/Keys.svelte";
  import { keycapsOf } from "../../lib/ui/hotkeys";
  import Progress from "../../lib/ui/Progress.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import TitleBar from "../../lib/ui/TitleBar.svelte";
  import { pname, STT_PROVIDERS } from "../providers";
  import KeyStep from "./KeyStep.svelte";

  const s = $derived(app.settings!);
  const TOTAL = 5;
  const STEP_NAMES: TKey[] = ["obx.s1", "obx.s2", "obx.s3", "obx.s4", "obx.s5"];
  /** Light whisper model used for the live preview while speaking. */
  const PREVIEW_ID = "base-q5_1";

  let step = $state(1);
  let dir = $state(1);
  type Path = "groq" | "offline" | "other";
  let path = $state<Path>("groq");
  let other = $state<KeyProviderId>("gemini");
  let keyOk = $state(false);

  // local models (offline path + live preview)
  let models = $state<LocalModel[]>([]);
  let progs = $state<Record<string, ModelProgress>>({});
  const rec = $derived(models.find((m) => m.recommended) ?? models[0]);
  const modelReady = $derived(!!rec?.installed);
  const prog = $derived(rec ? progs[rec.id] : undefined);

  let live = $state(true);
  const previewModel = $derived(models.find((m) => m.id === PREVIEW_ID));
  const previewProg = $derived(progs[PREVIEW_ID]);
  const previewReady = $derived(!!previewModel?.installed);
  const previewBusy = $derived(!!previewProg && !previewProg.done && !previewProg.error);

  // test + finish
  let celebrated = $state(false);
  let testText = $state("");
  let finishing = $state(false);
  let finishError = $state("");

  const name = $derived((s.user_name ?? "").trim());

  const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  onMount(() => {
    api.listLocalModels().then((m) => (models = m));
    const offs = [
      on<ModelProgress>(EVENTS.modelProgress, async (p) => {
        progs = { ...progs, [p.id]: p };
        if (p.done) models = await api.listLocalModels();
      }),
      on(EVENTS.historyChanged, async () => {
        if (step !== TOTAL) return;
        // In the real app the text is pasted into the focused textarea by Rust; the mock can't paste.
        if (!isTauri) {
          const h = await api.getHistory();
          if (h[0] && !testText) testText = h[0].text;
        }
        celebrated = true;
      }),
    ];
    return () => offs.forEach((p) => p.then((f) => f()));
  });

  const PATHS: { id: Path; icon: IconName; title: TKey; body: TKey; meta: () => string; tag?: boolean }[] = [
    { id: "groq", icon: "cloud", title: "ob.path_groq", body: "ob.path_groq_body", meta: () => t("obx.meta_groq"), tag: true },
    {
      id: "offline",
      icon: "lock",
      title: "ob.path_offline",
      body: "ob.path_offline_body",
      meta: () => t("obx.meta_offline", { mb: rec?.size_mb ?? "—" }),
    },
    { id: "other", icon: "key", title: "ob.path_other", body: "ob.path_other_body", meta: () => t("obx.meta_other") },
  ];

  const keyProvider = $derived<KeyProviderId>(path === "groq" ? "groq" : other);

  function applyPath() {
    if (path === "offline") {
      s.stt.engine = "local";
      if (rec) s.stt.local_model = rec.id;
      return;
    }
    s.stt.engine = "cloud";
    const p = keyProvider as LlmProviderId;
    s.llm_order = [p, ...s.llm_order.filter((x) => x !== p)];
    if (STT_PROVIDERS.includes(p as SttProviderId)) {
      const sp = p as SttProviderId;
      s.stt.cloud_order = [sp, ...s.stt.cloud_order.filter((x) => x !== sp)];
    }
  }

  function applyLive() {
    s.stt.live_preview = live;
    if (!live) return;
    s.stt.preview_model = PREVIEW_ID;
    if (!previewReady && !previewBusy) startDownload(PREVIEW_ID, previewModel?.size_mb ?? 57);
  }

  function leaving(from: number) {
    // Never leave global shortcuts paused if the hotkey field was still capturing.
    if (from === 4) api.suspendHotkeys(false).catch(() => {});
  }

  function go(to: number) {
    to = Math.max(1, Math.min(TOTAL, to));
    if (to === step) return;
    leaving(step);
    finishError = "";
    dir = to > step ? 1 : -1;
    step = to;
  }
  function next() {
    if (step === 3) applyPath();
    if (step === 4) applyLive();
    go(step + 1);
  }
  const back = () => go(step - 1);

  async function finish() {
    if (finishing) return;
    leaving(step);
    if (step === 4) applyLive();
    finishing = true;
    finishError = "";
    try {
      // Saves pending edits + onboarded:true in order; App's gate flips as soon as it resolves.
      await completeOnboarding();
    } catch {
      finishError = t("obx.finish_error");
    } finally {
      finishing = false;
    }
  }

  function startDownload(id: string, mb: number) {
    progs = { ...progs, [id]: { id, downloaded: 0, total: mb * 1048576, done: false, error: null } };
    api.downloadLocalModel(id).catch((e) => {
      progs = { ...progs, [id]: { id, downloaded: 0, total: 0, done: false, error: String(e) } };
    });
  }
  const download = () => rec && startDownload(rec.id, rec.size_mb);

  const canNext = $derived(step !== 3 || (path === "offline" ? modelReady : keyOk));
  const frac = (p?: ModelProgress) => (p && p.total ? p.downloaded / p.total : 0);

  // Direction-aware step transitions (transform/opacity only).
  const outExpo = (x: number) => (x === 1 ? 1 : 1 - Math.pow(2, -10 * x));
  const inCubic = (x: number) => x * x * x;
  function stepIn(_n: Element, { d }: { d: number }): TransitionConfig {
    if (reduce) return { duration: 0 };
    return {
      delay: 90,
      duration: 460,
      easing: outExpo,
      css: (k, u) => `opacity:${k};transform:translate3d(${u * 36 * d}px,0,0)`,
    };
  }
  function stepOut(_n: Element, { d }: { d: number }): TransitionConfig {
    if (reduce) return { duration: 0 };
    return {
      duration: 170,
      easing: inCubic,
      css: (k, u) => `opacity:${k};transform:translate3d(${-u * 24 * d}px,0,0);pointer-events:none`,
    };
  }
</script>

<div class="ob">
  <TitleBar maximizable={false}>
    {#snippet left()}
      <span class="brand" data-tauri-drag-region><Logo size={18} glow={false} /><span data-tauri-drag-region>FlowVoice</span></span>
    {/snippet}
  </TitleBar>

  <div class="progress" role="progressbar" aria-label={t("ob.step", { n: step, total: TOTAL })} aria-valuemin={1} aria-valuemax={TOTAL} aria-valuenow={step}>
    <ol class="labels">
      {#each STEP_NAMES as name, i}
        <li class:done={i + 1 < step} class:cur={i + 1 === step}>
          <span class="num mono">{String(i + 1).padStart(2, "0")}</span>
          <span class="nm">{t(name)}</span>
        </li>
      {/each}
    </ol>
    <div class="track">
      <span class="fill" style="transform: scaleX({step / TOTAL})"></span>
      {#each Array(TOTAL - 1) as _, i}<i style="left: {((i + 1) / TOTAL) * 100}%"></i>{/each}
    </div>
  </div>

  <div class="stage">
    {#key step}
      <section class="step stagger" in:stepIn={{ d: dir }} out:stepOut={{ d: dir }}>
        {#if step === 1}
          <div class="welcome">
          <div class="wl">
          <div class="mark"><Logo size={88} animate="loop" /></div>
          <h1 class="big">{t("ob.welcome_title")}</h1>
          <p class="lead">{t("ob.welcome_body")}</p>
          </div>
          <div class="wr">
          <label class="namef">
            <span class="label">{t("obx.name_label")}</span>
            <input class="field" type="text" autocomplete="given-name" maxlength="40" placeholder={t("obx.name_placeholder")} bind:value={s.user_name} />
          </label>
          <div class="lang">
            <span class="label">{t("ob.language")}</span>
            <LangPicker bind:value={s.ui_language} label={t("ob.language")} />
          </div>
          <div class="lang accent">
            <span class="label">{t("gen.accent")}</span>
            <AccentPicker bind:value={s.accent_color} label={t("gen.accent")} customLabel={t("gen.accent_custom")} large />
          </div>
          </div>
          </div>
        {:else if step === 2}
          <h1>{name ? t("obx.path_named", { name }) : t("ob.path_title")}</h1>
          <p class="lead">{t("ob.path_body")}</p>
          <div class="paths stagger" role="radiogroup" aria-label={t("ob.path_title")}>
            {#each PATHS as p, i (p.id)}
              <button type="button" role="radio" aria-checked={path === p.id} class="path" class:sel={path === p.id} onclick={() => (path = p.id)}>
                <span class="bar" aria-hidden="true"></span>
                <span class="pnum mono" aria-hidden="true">{String(i + 1).padStart(2, "0")}</span>
                <span class="pi"><Icon name={p.icon} size={20} /></span>
                <span class="pt">
                  <span class="ptt">{t(p.title)} {#if p.tag}<Badge tone="solid">{t("c.recommended")}</Badge>{/if}</span>
                  <span class="ptb">{t(p.body)}</span>
                  <span class="pmeta mono">{p.meta()}</span>
                </span>
                <span class="radio" aria-hidden="true"><Icon name="check" size={12} stroke={2.4} /></span>
              </button>
            {/each}
          </div>
        {:else if step === 3}
          {#if path === "offline"}
            <h1>{t("ob.model_title")}</h1>
            {#if rec}
              <p class="lead">{t("ob.model_body", { label: rec.label, mb: rec.size_mb })}</p>
              <div class="model">
                {#if modelReady}
                  <p class="okline"><Icon name="check" size={18} stroke={2} />{t("ob.model_done")}</p>
                {:else if prog && !prog.done && !prog.error}
                  <Progress value={frac(prog)} label={rec.label} />
                  <span class="mono faint tnum">
                    {Math.round(prog.downloaded / 1048576)} / {Math.round(prog.total / 1048576)} MB
                  </span>
                {:else}
                  <Button variant="primary" size="lg" icon="download" onclick={download}>{t("ob.model_download")}</Button>
                  {#if prog?.error}<code class="mono err">{prog.error}</code>{/if}
                {/if}
              </div>
            {/if}
          {:else}
            <h1>{t("ob.key_title", { provider: pname(keyProvider) })}</h1>
            <p class="lead">{t("ob.key_body")}</p>
            {#if path === "other"}
              <div class="pick">
                <Segmented
                  size="sm"
                  label={t("ob.pick_provider")}
                  bind:value={other}
                  options={(["gemini", "openrouter", "mistral", "openai"] as const).map((id) => ({ value: id, label: pname(id) }))}
                />
              </div>
            {/if}
            <KeyStep provider={keyProvider} bind:ok={keyOk} />
          {/if}
        {:else if step === 4}
          <h1>{t("ob.hotkey_title")}</h1>
          <p class="lead">{t("ob.hotkey_body")}</p>
          <div class="hk">
            <HotkeyInput size="lg" label={t("sc.dictate")} bind:value={s.hotkeys.dictate} />
          </div>
          <div class="sty">
            <h2 class="label">{t("ob.style")}</h2>
            <div class="chips" role="radiogroup" aria-label={t("ob.style")}>
              {#each s.styles as st (st.id)}
                <button
                  type="button"
                  role="radio"
                  aria-checked={s.active_style === st.id}
                  class:sel={s.active_style === st.id}
                  title={st.description}
                  onclick={() => (s.active_style = st.id)}
                >
                  {st.name}
                </button>
              {/each}
            </div>
            {#key s.active_style}
              <p class="sdesc muted">{s.styles.find((x) => x.id === s.active_style)?.description}</p>
            {/key}
          </div>
          <div class="liveopt" class:on={live}>
            <span class="lo-ic"><Icon name="wave" size={18} /></span>
            <span class="lo-t">
              <span class="lo-title">{t("obx.live_title")}</span>
              <span class="lo-body">{t("obx.live_body")}</span>
              {#if live}
                <span class="lo-st mono">
                  {#if previewReady}
                    <Icon name="check" size={13} stroke={2} />{t("obx.live_ready")}
                  {:else if previewBusy}
                    {t("obx.live_downloading")} · {Math.round(frac(previewProg) * 100)}%
                  {:else}
                    {t("obx.live_later")}
                  {/if}
                </span>
              {/if}
            </span>
            <Toggle label={t("obx.live_title")} bind:checked={live} />
          </div>
        {:else}
          <h1>{t("ob.test_title")}</h1>
          <p class="lead">
            {t("ob.test_body", { hotkey: keycapsOf(s.hotkeys.dictate).join(" + ") })}
          </p>
          <div class="testkeys"><Keys accel={s.hotkeys.dictate} size="lg" /></div>
          <textarea class="field testarea" rows="4" placeholder={t("ob.test_placeholder")} bind:value={testText}></textarea>
          <div class="tstatus">
            {#if celebrated}
              <div class="yay" role="status">
                <span class="seal" aria-hidden="true">
                  <span class="rays">{#each Array(10) as _, i}<i style="--a:{i * 36}deg"></i>{/each}</span>
                  <svg viewBox="0 0 40 40" width="40" height="40">
                    <circle class="ring" cx="20" cy="20" r="18" />
                    <path class="tick" d="M12.5 20.5l5 5 10-11" />
                  </svg>
                </span>
                <span class="yt">
                  <b>{name ? t("obx.success_named", { name }) : t("ob.test_success")}</b>
                  <span>{t("obx.success_body")}</span>
                </span>
              </div>
            {:else}
              <span class="wait"><Icon name="mic" size={16} />{t("ob.test_waiting")}</span>
              <Button size="sm" variant="ghost" icon="play" onclick={() => api.startRecording("dictate")}>{t("ob.test_button")}</Button>
            {/if}
          </div>
          {#if s.stt.live_preview && previewProg}
            <div class="dl">
              {#if previewReady}
                <span class="okmini mono"><Icon name="check" size={13} stroke={2} />{t("obx.live_ready")}</span>
              {:else if previewProg.error}
                <span class="err">{t("obx.live_error")}</span>
              {:else}
                <span class="mono faint">{t("obx.live_downloading")} · {Math.round(frac(previewProg) * 100)}%</span>
                <Progress value={frac(previewProg)} label={t("obx.live_downloading")} />
              {/if}
            </div>
          {/if}
        {/if}
      </section>
    {/key}
  </div>

  <footer>
    <span class="count mono faint">
      {#key step}<span class="cnum">{String(step).padStart(2, "0")}</span>{/key}<span>/ {String(TOTAL).padStart(2, "0")}</span>
    </span>
    {#if finishError}<span class="ferr" role="alert"><Icon name="alert" size={14} />{finishError}</span>{/if}
    <div class="nav">
      {#if step > 1}
        <Button variant="ghost" icon="chevronLeft" disabled={finishing} onclick={back}>{t("c.back")}</Button>
      {/if}
      {#if step === 3 && !canNext}
        <Button variant="quiet" onclick={next}>{t("ob.skip_now")}</Button>
      {/if}
      {#if step === 1}
        <Button variant="primary" iconRight="arrowRight" onclick={next}>{t("ob.start")}</Button>
      {:else if step < TOTAL}
        <Button variant="primary" iconRight="arrowRight" disabled={!canNext} onclick={next}>{t("c.next")}</Button>
      {:else if celebrated}
        <Button variant="primary" iconRight="check" loading={finishing} onclick={finish}>{t("c.finish")}</Button>
      {:else}
        <Button variant="default" iconRight="arrowRight" loading={finishing} onclick={finish}>{t("obx.skip_test")}</Button>
      {/if}
    </div>
  </footer>
</div>

<style>
  .ob {
    flex: 1;
    width: 100%;
    min-width: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    font-weight: 600;
  }

  /* ---------- progress */
  .progress {
    width: min(820px, calc(100% - 56px));
    margin: 6px auto 0;
  }
  .labels {
    list-style: none;
    margin: 0 0 8px;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
  }
  .labels li {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 11.5px;
    color: var(--text-3);
    transition: color var(--t-slow) var(--ease);
    white-space: nowrap;
    overflow: hidden;
  }
  .labels .num {
    font-size: 10px;
    letter-spacing: 0.06em;
  }
  .labels li.done {
    color: var(--text-2);
  }
  .labels li.cur {
    color: var(--text);
  }
  .labels li.cur .num {
    color: var(--accent-fg);
  }
  .track {
    position: relative;
    height: 2px;
    border-radius: 2px;
    background: var(--line-strong);
    overflow: hidden;
  }
  .fill {
    position: absolute;
    inset: 0;
    background: var(--accent-fg);
    transform-origin: left center;
    transition: transform 620ms var(--ease-out);
  }
  .track i {
    position: absolute;
    top: 0;
    width: 3px;
    height: 100%;
    margin-left: -1.5px;
    background: var(--bg);
  }

  /* ---------- stage */
  .stage {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    display: grid;
  }
  .step {
    grid-area: 1 / 1;
    align-self: center;
    justify-self: center;
    width: min(820px, calc(100% - 56px));
    padding: 28px 0 24px;
    will-change: transform, opacity;
  }
  .step.stagger {
    --stagger: 45ms;
  }
  h1 {
    font-size: 28px;
    line-height: 1.15;
    letter-spacing: -0.03em;
  }
  h1.big {
    margin-top: 26px;
    font-size: 40px;
    max-width: 22ch;
  }
  .lead {
    margin-top: 8px;
    color: var(--text-2);
    font-size: 14.5px;
    max-width: 68ch;
  }
  .mark {
    display: inline-flex;
  }
  .namef {
    margin-top: 28px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(420px, 100%);
  }
  .namef .field {
    height: 38px;
  }
  .lang {
    margin-top: 20px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  /* ---------- path cards */
  .paths {
    margin-top: 22px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    --stagger: 60ms;
  }
  .path {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px 14px 14px;
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    text-align: left;
    overflow: hidden;
    transition:
      transform var(--t) var(--ease),
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease);
  }
  .path:hover {
    transform: translate3d(0, -1px, 0);
    border-color: var(--text-3);
  }
  .path:active {
    transform: scale(0.99);
    transition-duration: 60ms;
  }
  .path.sel {
    border-color: var(--accent-line);
    background: linear-gradient(var(--accent-soft), var(--accent-soft)), var(--surface);
  }
  .bar {
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent-fg);
    transform: scaleY(0);
    transition: transform var(--t-slow) var(--spring);
  }
  .sel .bar {
    transform: scaleY(1);
  }
  .pnum {
    align-self: flex-start;
    margin-top: 2px;
    font-size: 10.5px;
    color: var(--text-3);
    letter-spacing: 0.06em;
  }
  .sel .pnum {
    color: var(--accent-fg);
  }
  .pi {
    width: 38px;
    height: 38px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    color: var(--text-2);
    transition:
      background-color var(--t) var(--ease),
      color var(--t) var(--ease),
      transform var(--t-slow) var(--spring);
  }
  .sel .pi {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    transform: rotate(-4deg);
  }
  .pt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .ptt {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-weight: 600;
    font-size: 14.5px;
  }
  .ptb {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .pmeta {
    margin-top: 3px;
    font-size: 10.5px;
    color: var(--text-3);
    letter-spacing: 0.04em;
  }
  .radio {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    display: grid;
    place-items: center;
    flex: none;
    color: var(--on-accent);
    transition:
      background-color var(--t) var(--ease),
      border-color var(--t) var(--ease);
  }
  .radio :global(.icon) {
    transform: scale(0);
    transition: transform var(--t-slow) var(--spring);
  }
  .sel .radio {
    border-color: var(--accent);
    background: var(--accent);
  }
  .sel .radio :global(.icon) {
    transform: scale(1);
  }

  /* ---------- key / model */
  .pick {
    margin: 18px 0 4px;
  }
  .step :global(.ks) {
    margin-top: 20px;
  }
  .model {
    margin-top: 24px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
  }
  .okline {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent-fg);
    font-size: 14px;
    animation: fv-pop var(--t-slow) var(--spring) both;
  }
  .err {
    color: var(--danger-fg);
  }

  /* ---------- hotkey + style + live */
  .hk {
    margin-top: 22px;
  }
  .sty {
    margin-top: 26px;
  }
  .sty .label {
    font-family: var(--mono);
    letter-spacing: 0.12em;
  }
  .chips {
    margin-top: 10px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips button {
    height: 32px;
    padding: 0 14px;
    border-radius: 16px;
    border: 1px solid var(--line-strong);
    font-size: 13px;
    color: var(--text-2);
    transition:
      transform var(--t) var(--spring),
      background-color var(--t) var(--ease),
      border-color var(--t) var(--ease),
      color var(--t) var(--ease);
  }
  .chips button:hover {
    color: var(--text);
    border-color: var(--text-3);
  }
  .chips button:active {
    transform: scale(0.95);
    transition-duration: 60ms;
  }
  .chips button.sel {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .sdesc {
    margin-top: 10px;
    font-size: 12.5px;
    min-height: 1.5em;
    animation: fv-fade var(--t-slow) var(--ease) both;
  }
  .liveopt {
    margin-top: 22px;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    transition:
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease);
  }
  .liveopt.on {
    border-color: var(--accent-line);
  }
  .lo-ic {
    width: 34px;
    height: 34px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--surface-2);
    color: var(--text-2);
    transition:
      background-color var(--t) var(--ease),
      color var(--t) var(--ease);
  }
  .on .lo-ic {
    background: var(--accent-soft);
    color: var(--accent-fg);
  }
  .lo-t {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .lo-title {
    font-weight: 600;
    font-size: 14px;
  }
  .lo-body {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .lo-st {
    margin-top: 4px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--accent-fg);
    animation: fv-fade var(--t-slow) var(--ease) both;
  }

  /* ---------- test */
  .testkeys {
    margin-top: 18px;
  }
  .testarea {
    margin-top: 18px;
    font-size: 14px;
    resize: none;
  }
  .tstatus {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--text-2);
    min-height: 48px;
  }
  .wait {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .yay {
    display: flex;
    align-items: center;
    gap: 14px;
    animation: fv-rise var(--t-page) var(--ease-out) both;
  }
  .seal {
    position: relative;
    width: 40px;
    height: 40px;
    flex: none;
    color: var(--accent-fg);
  }
  .seal svg {
    display: block;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .ring {
    stroke-width: 1.5;
    stroke-dasharray: 114;
    stroke-dashoffset: 114;
    transform: rotate(-90deg);
    transform-origin: center;
    animation: draw 520ms var(--ease-out) forwards;
  }
  .tick {
    stroke-width: 2.4;
    stroke-dasharray: 24;
    stroke-dashoffset: 24;
    animation: draw 320ms 380ms var(--ease-out) forwards;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  .rays {
    position: absolute;
    left: 50%;
    top: 50%;
  }
  .rays i {
    position: absolute;
    left: -1px;
    top: -4px;
    width: 2px;
    height: 8px;
    border-radius: 2px;
    background: var(--accent-fg);
    opacity: 0;
    transform: rotate(var(--a)) translate3d(0, -18px, 0);
    animation: ray 640ms 300ms var(--ease-out) both;
  }
  @keyframes ray {
    0% {
      opacity: 1;
      transform: rotate(var(--a)) translate3d(0, -18px, 0) scaleY(1);
    }
    100% {
      opacity: 0;
      transform: rotate(var(--a)) translate3d(0, -32px, 0) scaleY(0.3);
    }
  }
  .yt {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .yt b {
    color: var(--accent-fg);
    font-weight: 600;
    font-size: 14px;
  }
  .yt span {
    font-size: 12.5px;
  }
  .dl {
    margin-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 11px;
    animation: fv-fade var(--t-slow) var(--ease) both;
  }

  .okmini {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--accent-fg);
    animation: fv-pop var(--t-slow) var(--spring) both;
  }

  /* ---------- footer */
  footer {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: min(820px, calc(100% - 56px));
    margin: 0 auto;
    padding: 14px 0 22px;
    border-top: 1px solid var(--line);
    background: var(--bg);
  }
  .nav {
    display: flex;
    gap: 6px;
    margin-left: auto;
  }
  .count {
    font-size: 11px;
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    overflow: hidden;
  }
  .cnum {
    display: inline-block;
    color: var(--text);
    animation: fv-rise var(--t-slow) var(--ease-out) both;
  }
  .ferr {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--danger-fg);
    animation: fv-fade var(--t) var(--ease) both;
  }
  @media (max-width: 620px) {
    .labels .nm {
      display: none;
    }
  }
  /* welcome: copy on the left, name + language card on the right when there is room */
  .welcome {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(280px, 1fr);
    gap: 40px;
    align-items: center;
  }
  .wr {
    padding: 22px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  .wr .namef {
    margin-top: 0;
    width: 100%;
  }
  @media (max-width: 760px) {
    .welcome {
      grid-template-columns: 1fr;
      gap: 8px;
    }
    .wr {
      padding: 0;
      border: 0;
      background: none;
    }
    .wr .namef {
      margin-top: 28px;
    }
  }
  .wr .lang {
    align-items: stretch;
  }
</style>
