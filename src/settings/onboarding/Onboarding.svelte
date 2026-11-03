<script lang="ts">
  import AccentPicker from "../../lib/ui/AccentPicker.svelte";
  import LangPicker from "../../lib/ui/LangPicker.svelte";
  import { onMount } from "svelte";
  import { api, isTauri, on } from "../../lib/api";
  import { t, type TKey } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import Logo from "../../lib/icons/Logo.svelte";
  import ProviderLogo from "../../lib/icons/ProviderLogo.svelte";
  import type { IconName } from "../../lib/icons/paths";
  import { app, completeOnboarding, hasKey } from "../../lib/stores.svelte";
  import { EVENTS, type KeyProviderId, type LlmProviderId, type LocalModel, type ModelProgress, type SttProviderId } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import HotkeyInput from "../../lib/ui/HotkeyInput.svelte";
  import Keys from "../../lib/ui/Keys.svelte";
  import { keycapsOf } from "../../lib/ui/hotkeys";
  import Segmented from "../../lib/ui/Segmented.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import TitleBar from "../../lib/ui/TitleBar.svelte";
  import { pname, STT_PROVIDERS } from "../providers";
  import BehaviorStep from "./BehaviorStep.svelte";
  import KeyStep from "./KeyStep.svelte";
  import ModelPick from "./ModelPick.svelte";
  import { fmtSize, langName, pickModels } from "./models";
  import { cssLinear, reduceMotion as reduce, softSpring, spring, stepIn, stepOut } from "./motion";

  const s = $derived(app.settings!);
  const TOTAL = 6;
  const STEP_NAMES: TKey[] = ["obx.s1", "obx.s2", "obx.s3", "obx.s4", "obx.s5", "obx.s6"];
  /** Step numbers (the progress track, Back/Next and the per-step logic all key off these). */
  const S = { welcome: 1, path: 2, connect: 3, hotkey: 4, behavior: 5, test: 6 } as const;
  const OTHER: KeyProviderId[] = ["gemini", "openrouter", "mistral", "openai"];
  // Spring curves as CSS easings (progress fill, staggered content, ambient glow).
  const SPRING_CSS = cssLinear(spring);
  const SOFT_CSS = cssLinear(softSpring);

  // ?obstep=N (browser preview only): open the wizard directly on a step for visual QA.
  let step = $state(isTauri ? 1 : Math.min(6, Math.max(1, Number(new URLSearchParams(location.search).get("obstep")) || 1)));
  let dir = $state(1);
  type Path = "groq" | "offline" | "other";
  // Prefill from the current settings, so "Refazer configuração inicial" starts where the user is.
  const firstKeyed = app.settings!.llm_order.find((p) => p !== "ollama" && p !== "custom" && hasKey(p)) as KeyProviderId | undefined;
  let path = $state<Path>(
    app.settings!.stt.engine === "local" ? "offline" : firstKeyed && firstKeyed !== "groq" && OTHER.includes(firstKeyed) ? "other" : "groq",
  );
  let other = $state<KeyProviderId>(firstKeyed && OTHER.includes(firstKeyed) ? firstKeyed : "gemini");
  let keyOk = $state(false);

  // ---- local models (offline path, and the live preview when there is no cloud key)
  let models = $state<LocalModel[]>([]);
  let progs = $state<Record<string, ModelProgress>>({});
  /** Downloads the user explicitly sent to the background (kept running when leaving a step). */
  let bg = $state<Record<string, boolean>>({});
  /** Downloads that finished while the wizard was open ("pronto" instead of "você já tem"). */
  let fresh = $state<Record<string, boolean>>({});
  const cancelled = new Set<string>();
  /** The language the user speaks; "auto" falls back to the interface language. */
  const speech = $derived(s.speech_language && s.speech_language !== "auto" ? s.speech_language : s.ui_language);
  const speechName = $derived(langName(speech, s.ui_language));
  const picks = $derived(pickModels(models, speech));
  /** User's choice; until they pick, whatever is installed, else the best model for the language. */
  let chosen = $state("");
  const pick = $derived(picks.options.find((m) => m.id === chosen) ?? picks.installed ?? picks.top);
  const running = (id: string) => {
    const p = progs[id];
    return !!p && !p.done && !p.error;
  };
  const frac = (p?: ModelProgress) => (p && p.total ? p.downloaded / p.total : 0);
  /** A suitable model is on disk, or downloading in the background on the user's say-so. */
  const localOk = $derived(!!pick && (pick.installed || (running(pick.id) && !!bg[pick.id])));
  const bgDownloads = $derived(models.filter((m) => bg[m.id] && running(m.id)));

  // ---- live preview: cloud when there is a cloud STT key, otherwise a local model
  let live = $state(app.settings!.stt.live_preview);
  const cloudProv = $derived(
    s.stt.engine === "cloud" ? (s.stt.cloud_order.find((p) => hasKey(p)) ?? STT_PROVIDERS.find((p) => hasKey(p))) : undefined,
  );

  // test + finish
  let celebrated = $state(false);
  let testText = $state("");
  let finishing = $state(false);
  let leavingAll = $state(false);
  let finishError = $state("");

  const name = $derived((s.user_name ?? "").trim());

  onMount(() => {
    api.listLocalModels().then((m) => (models = m));
    const offs = [
      on<ModelProgress>(EVENTS.modelProgress, async (p) => {
        if (cancelled.has(p.id)) return;
        if (p.cancelled || (p.error && /cancel/i.test(p.error))) {
          drop(p.id);
          return;
        }
        progs = { ...progs, [p.id]: p };
        if (p.done) {
          if (!p.error) fresh = { ...fresh, [p.id]: true };
          models = await api.listLocalModels();
        }
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
    return () => {
      stopForeground();
      offs.forEach((p) => p.then((f) => f()));
    };
  });

  const PATHS: { id: Path; icon: IconName; title: TKey; body: TKey; meta: () => string; tag?: boolean }[] = [
    { id: "groq", icon: "cloud", title: "ob.path_groq", body: "ob.path_groq_body", meta: () => t("obx.meta_groq"), tag: true },
    {
      id: "offline",
      icon: "lock",
      title: "ob.path_offline",
      body: "ob.path_offline_body",
      meta: () =>
        picks.installed
          ? t("obx.meta_offline_have", { label: picks.installed.label })
          : t("obx.meta_offline", { mb: picks.top ? fmtSize(picks.top.size_mb) : "—" }),
    },
    { id: "other", icon: "key", title: "ob.path_other", body: "ob.path_other_body", meta: () => t("obx.meta_other") },
  ];

  const keyProvider = $derived<KeyProviderId>(path === "groq" ? "groq" : other);

  function applyPath() {
    if (path === "offline") {
      s.stt.engine = "local";
      if (pick) s.stt.local_model = pick.id;
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
    // With a cloud key the preview runs in the cloud: nothing local is needed.
    // Without one it uses the local model chosen here; with none on disk (and none coming in the
    // background) it stays off, so nothing is downloaded behind the user's back.
    if (live && !cloudProv) {
      if (localOk && pick) s.stt.preview_model = pick.id;
      else live = false;
    }
    s.stt.live_preview = live;
  }

  function leaving(from: number) {
    // Never leave global shortcuts paused if the hotkey field was still capturing.
    if (from === S.hotkey) api.suspendHotkeys(false).catch(() => {});
    // A download started on this step only keeps going if the user sent it to the background.
    stopForeground();
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
    if (step === S.connect) applyPath();
    if (step === S.hotkey) applyLive();
    go(step + 1);
  }
  const back = () => go(step - 1);

  async function finish() {
    if (finishing) return;
    leaving(step);
    if (step === S.hotkey) applyLive();
    finishing = true;
    finishError = "";
    leavingAll = true;
    try {
      // Saves pending edits + onboarded:true in order; App's gate flips as soon as it resolves.
      // The short wait lets the wizard's exit play first.
      await Promise.all([completeOnboarding(), new Promise((r) => setTimeout(r, reduce ? 0 : 380))]);
    } catch {
      leavingAll = false;
      finishError = t("obx.finish_error");
    } finally {
      finishing = false;
    }
  }

  // ---- downloads
  function drop(id: string) {
    const { [id]: _p, ...rest } = progs;
    progs = rest;
    const { [id]: _b, ...rb } = bg;
    bg = rb;
  }
  function startDownload(m: LocalModel) {
    cancelled.delete(m.id);
    chosen = m.id;
    progs = { ...progs, [m.id]: { id: m.id, downloaded: 0, total: m.size_mb * 1048576, done: false, error: null } };
    api.downloadLocalModel(m.id).catch((e) => {
      if (cancelled.has(m.id) || /cancel/i.test(String(e))) return;
      progs = { ...progs, [m.id]: { id: m.id, downloaded: 0, total: 0, done: true, error: String(e) } };
    });
  }
  function cancelDownload(id: string) {
    cancelled.add(id);
    drop(id);
    api.cancelLocalModelDownload(id).catch(() => {});
  }
  const toBackground = (id: string) => (bg = { ...bg, [id]: true });
  function stopForeground() {
    for (const id of Object.keys(progs)) if (running(id) && !bg[id]) cancelDownload(id);
  }

  const canNext = $derived(step !== S.connect || (path === "offline" ? localOk : keyOk || hasKey(keyProvider)));
</script>
<div
  class="ob"
  class:bye={leavingAll}
  class:won={celebrated}
  style="--spring-e: {SPRING_CSS}; --soft-e: {SOFT_CSS}; --p: {step / TOTAL}; --sx: {step}"
>
  <span class="aura" aria-hidden="true"></span>
  <TitleBar maximizable={false}>
    {#snippet left()}
      <span class="brand" data-tauri-drag-region><Logo size={18} glow={false} /><span data-tauri-drag-region>FlowVoice</span></span>
    {/snippet}
  </TitleBar>

  <div class="progress" role="progressbar" aria-label={t("ob.step", { n: step, total: TOTAL })} aria-valuemin={1} aria-valuemax={TOTAL} aria-valuenow={step}>
    <ol class="labels">
      {#each STEP_NAMES as nm, i}
        {@const n = i + 1}
        {@const done = n < step || (celebrated && n === TOTAL)}
        <li class:done class:cur={n === step}>
          <!-- finished steps are a shortcut back -->
          <button type="button" tabindex={n < step ? 0 : -1} disabled={n >= step || finishing} onclick={() => go(n)}>
            <span class="num mono">
              {#if done}<span class="ck"><Icon name="check" size={10} stroke={2.8} /></span>{:else}{String(n).padStart(2, "0")}{/if}
            </span>
            <span class="nm">{t(nm)}</span>
          </button>
        </li>
      {/each}
    </ol>
    <div class="track">
      <span class="rail"></span>
      <span class="fill"></span>
      {#each Array(TOTAL - 1) as _, i}<i style="left: {((i + 1) / TOTAL) * 100}%"></i>{/each}
      <span class="head" aria-hidden="true"><b></b>{#key step}<em></em>{/key}</span>
    </div>
  </div>

  <div class="stage">
    {#key step}
      <section class="step stagger" style="--d: {dir}" in:stepIn={{ d: dir }} out:stepOut={{ d: dir }}>
        {#if step === S.welcome}
          <div class="welcome">
            <div class="wl stagger">
              <div class="mark"><Logo size={88} animate="loop" /></div>
              <h1 class="big">{t("ob.welcome_title")}</h1>
              <p class="lead">{t("ob.welcome_body")}</p>
            </div>
            <div class="wr stagger">
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
              <div class="lang look">
                <span class="label">{t("ctl.ov_glass")}</span>
                <Segmented
                  label={t("ctl.ov_glass")}
                  size="sm"
                  bind:value={s.overlay_glass}
                  options={[
                    { value: "solid", label: t("ctl.glass_solid") },
                    { value: "glass", label: t("ctl.glass_glass") },
                    { value: "translucent", label: t("ctl.glass_translucent") },
                  ]}
                />
                <!-- the capsule over a colorful "wallpaper": glass is CSS blur here, Windows' own in the app -->
                <div class="capprev" aria-label={t("ctl.glass_preview")} role="img">
                  <span class="wall" aria-hidden="true"><i></i><i></i><i></i></span>
                  <span class="pill {s.overlay_glass}" aria-hidden="true">
                    <span class="bars">{#each [5, 9, 14, 8, 12, 6, 10, 4] as h, i (i)}<b style="height:{h}px; animation-delay:{i * 90}ms"></b>{/each}</span>
                    <span class="said">{t("obx.look_sample")}</span>
                  </span>
                </div>
                {#if s.overlay_glass !== "solid"}
                  <label class="wglass">
                    <Toggle label={t("ctl.win_glass")} bind:checked={s.window_glass} />
                    <span>{t("ctl.win_glass")}</span>
                  </label>
                {/if}
              </div>
            </div>
          </div>
        {:else if step === S.path}
          <h1>{name ? t("obx.path_named", { name }) : t("ob.path_title")}</h1>
          <p class="lead">{t("ob.path_body")}</p>
          <div class="paths stagger" role="radiogroup" aria-label={t("ob.path_title")}>
            {#each PATHS as p, i (p.id)}
              <button type="button" role="radio" aria-checked={path === p.id} class="path" class:sel={path === p.id} onclick={() => (path = p.id)}>
                <span class="bar" aria-hidden="true"></span>
                <span class="pnum mono" aria-hidden="true">{String(i + 1).padStart(2, "0")}</span>
                <span class="pi" class:logo={p.id === "groq"}>
                  {#if p.id === "groq"}<ProviderLogo id="groq" size={38} />{:else}<Icon name={p.icon} size={20} />{/if}
                </span>
                <span class="pt">
                  <span class="ptt">{t(p.title)} {#if p.tag}<Badge tone="solid">{t("c.recommended")}</Badge>{/if}</span>
                  <span class="ptb">{t(p.body)}</span>
                  <span class="pmeta mono">{p.meta()}</span>
                </span>
                <span class="radio" aria-hidden="true"><Icon name="check" size={12} stroke={2.4} /></span>
              </button>
            {/each}
          </div>
        {:else if step === S.connect}
          {#if path === "offline"}
            <h1>{t("obx.model_title")}</h1>
            <p class="lead">{t("obx.model_body", { lang: speechName })}</p>
            {#if picks.options.length}
              <div class="model">
                <ModelPick
                  {picks}
                  lang={speechName}
                  {progs}
                  {bg}
                  {fresh}
                  bind:value={chosen}
                  ondownload={startDownload}
                  oncancel={cancelDownload}
                  onbackground={toBackground}
                />
              </div>
            {/if}
          {:else}
            <h1 class="kh">
              {#key keyProvider}<span class="khl"><ProviderLogo id={keyProvider} size={30} /></span>{/key}
              <span>{t("ob.key_title", { provider: pname(keyProvider) })}</span>
            </h1>
            <p class="lead">{t("ob.key_body")}</p>
            {#if path === "other"}
              <div class="pick" role="radiogroup" aria-label={t("ob.pick_provider")}>
                {#each OTHER as id (id)}
                  <button type="button" role="radio" aria-checked={other === id} class:sel={other === id} onclick={() => (other = id)}>
                    <ProviderLogo {id} size={20} />
                    <span>{pname(id)}</span>
                  </button>
                {/each}
              </div>
            {/if}
            <KeyStep provider={keyProvider} bind:ok={keyOk} />
          {/if}
        {:else if step === S.hotkey}
          <h1>{t("ob.hotkey_title")}</h1>
          <p class="lead">{t("ob.hotkey_body")}</p>
          <div class="hk">
            <HotkeyInput
              size="lg"
              label={t("sc.dictate")}
              bind:value={s.hotkeys.dictate}
              taken={[
                { accel: s.hotkeys.translate, label: t("sc.translate") },
                { accel: s.hotkeys.ask, label: t("sc.ask") },
                { accel: s.hotkeys.command, label: t("sc.command") },
                { accel: s.hotkeys.paste_last, label: t("sc.paste_last") },
              ]}
            />
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
          <div class="livebox" class:on={live}>
            <div class="liveopt">
              <span class="lo-ic"><Icon name="wave" size={18} /></span>
              <span class="lo-t">
                <span class="lo-title">{t("obx.live_title")}</span>
                <span class="lo-body">{cloudProv ? t("obx.live_body_cloud") : t("obx.live_body_local")}</span>
                {#if live}
                  {#key `${!!cloudProv}${pick?.installed}${pick && bg[pick.id]}`}
                    {#if cloudProv}
                      <span class="lo-st ready"><Icon name="cloud" size={13} />{t("obx.live_cloud", { provider: pname(cloudProv) })}</span>
                    {:else if pick?.installed}
                      <span class="lo-st ready"><Icon name="check" size={13} stroke={2} />{t("obx.live_uses", { label: pick.label })}</span>
                    {:else if pick && bg[pick.id] && running(pick.id)}
                      <span class="lo-st">{t("obx.live_uses_dl", { label: pick.label })}</span>
                    {:else}
                      <span class="lo-st">{t("obx.live_needs")}</span>
                    {/if}
                  {/key}
                {/if}
              </span>
              <Toggle label={t("obx.live_title")} bind:checked={live} />
            </div>
            {#if live && !cloudProv && pick && !pick.installed && !bg[pick.id] && picks.options.length}
              <div class="livepick">
                <ModelPick
                  {picks}
                  lang={speechName}
                  {progs}
                  {bg}
                  {fresh}
                  bind:value={chosen}
                  ondownload={startDownload}
                  oncancel={cancelDownload}
                  onbackground={toBackground}
                />
              </div>
            {/if}
          </div>
        {:else if step === S.behavior}
          <h1>{t("obx.beh_title")}</h1>
          <p class="lead">{t("obx.beh_body")}</p>
          <BehaviorStep />
        {:else}
          <h1>{t("ob.test_title")}</h1>
          <p class="lead">
            {t("ob.test_body", { hotkey: keycapsOf(s.hotkeys.dictate).join(" + ") })}
          </p>
          <div class="testkeys"><Keys accel={s.hotkeys.dictate} size="lg" /></div>
          <textarea class="field testarea" class:won={celebrated} rows="4" placeholder={t("ob.test_placeholder")} bind:value={testText}></textarea>
          <div class="tstatus">
            {#if celebrated}
              <div class="yay" role="status">
                <span class="seal" aria-hidden="true">
                  <span class="halo"></span>
                  <span class="ripple"></span>
                  <span class="ripple r2"></span>
                  <span class="rays">{#each Array(12) as _, i}<i style="--a:{i * 30}deg;--l:{i % 2 ? 6 : 10}px"></i>{/each}</span>
                  <svg viewBox="0 0 48 48" width="48" height="48">
                    <circle class="disc" cx="24" cy="24" r="21" />
                    <circle class="ring" cx="24" cy="24" r="21" />
                    <path class="tick" d="M15 24.5l6 6 12-13" />
                  </svg>
                </span>
                <span class="yt">
                  <b>{name ? t("obx.success_named", { name }) : t("ob.test_success")}</b>
                  <span>{t("obx.success_body")}</span>
                </span>
              </div>
            {:else}
              <span class="wait"><span class="mic"><Icon name="mic" size={16} /></span>{t("ob.test_waiting")}</span>
              <Button size="sm" variant="ghost" icon="play" onclick={() => api.startRecording("dictate")}>{t("ob.test_button")}</Button>
            {/if}
          </div>
        {/if}
      </section>
    {/key}
  </div>

  <footer>
    <span class="count mono faint">
      {#key step}<span class="cnum">{String(step).padStart(2, "0")}</span>{/key}<span>/ {String(TOTAL).padStart(2, "0")}</span>
    </span>
    {#each bgDownloads as m (m.id)}
      {@const f = frac(progs[m.id])}
      <span class="bgchip mono tnum" title={t("obx.m_bg_chip", { label: m.label, pct: Math.round(f * 100) })}>
        <Icon name="download" size={12} />
        <span class="bgl">{m.label}</span>
        <span class="mini"><i style="transform: scaleX({f})"></i></span>
        <span>{Math.round(f * 100)}%</span>
        <button type="button" class="bgx" aria-label={t("obx.m_cancel")} title={t("obx.m_cancel")} onclick={() => cancelDownload(m.id)}>
          <Icon name="x" size={11} stroke={2} />
        </button>
      </span>
    {/each}
    {#if finishError}<span class="ferr" role="alert"><Icon name="alert" size={14} />{finishError}</span>{/if}
    <div class="nav">
      {#if step > 1}
        <Button variant="ghost" icon="chevronLeft" disabled={finishing} onclick={back}>{t("c.back")}</Button>
      {/if}
      {#if step === S.connect && !canNext}
        <Button variant="quiet" onclick={next}>{t("ob.skip_now")}</Button>
      {/if}
      {#if step === S.welcome}
        <Button variant="primary" iconRight="arrowRight" onclick={next}>{t("ob.start")}</Button>
      {:else if step < TOTAL}
        <Button variant="primary" iconRight="arrowRight" disabled={!canNext} onclick={next}>{t("c.next")}</Button>
      {:else if celebrated}
        <span class="fin"><Button variant="primary" iconRight="check" loading={finishing} onclick={finish}>{t("c.finish")}</Button></span>
      {:else}
        <Button variant="default" iconRight="arrowRight" loading={finishing} onclick={finish}>{t("obx.skip_test")}</Button>
      {/if}
    </div>
  </footer>
</div>

<style>
  .ob {
    position: relative;
    isolation: isolate;
    overflow: hidden;
    flex: 1;
    width: 100%;
    min-width: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--win-tint, var(--bg));
    transition: background-color var(--t-slow) var(--ease);
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    font-weight: 600;
  }

  /* ---------- ambient accent glow: drifts with the step, blooms on the welcome and on success.
     A painted-once gradient moved only by transform/opacity (its own compositor layer). */
  .aura {
    position: absolute;
    z-index: -1;
    left: 50%;
    top: 42%;
    width: 1100px;
    height: 760px;
    margin: -380px 0 0 -550px;
    border-radius: 50%;
    background:
      radial-gradient(closest-side at 50% 50%, var(--accent-soft), transparent 70%),
      radial-gradient(closest-side at 50% 50%, var(--accent-soft), transparent);
    pointer-events: none;
    will-change: transform, opacity;
    opacity: calc(0.75 * var(--aura-k, 1));
    transform: translate3d(calc((var(--sx) - 3.5) * 46px), calc((var(--sx) - 3.5) * -10px), 0) scale(0.92);
    transition:
      transform 1400ms var(--soft-e),
      opacity 900ms var(--ease);
  }
  /* light backgrounds show the tint much more: dim it */
  :global(:root[data-theme="light"]) .ob {
    --aura-k: 0.38;
  }
  @media (prefers-color-scheme: light) {
    :global(:root:not([data-theme="dark"])) .ob {
      --aura-k: 0.38;
    }
  }
  .ob:has(.welcome) .aura {
    opacity: var(--aura-k, 1);
    transform: translate3d(-160px, -40px, 0) scale(1.05);
  }
  .ob.won .aura {
    opacity: var(--aura-k, 1);
    transform: translate3d(0, 40px, 0) scale(1.18);
  }

  /* ---------- progress */
  .progress {
    width: min(820px, calc(100% - 56px));
    margin: 6px auto 0;
  }
  .labels {
    list-style: none;
    margin: 0 0 9px;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
  }
  .labels li {
    min-width: 0;
    font-size: 11.5px;
    color: var(--text-3);
    transition: color var(--t-slow) var(--ease);
  }
  .labels button {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 2px 0;
    color: inherit;
    white-space: nowrap;
    overflow: hidden;
    cursor: default;
  }
  .labels .done button:not(:disabled) {
    cursor: pointer;
  }
  .labels .done button:not(:disabled):hover {
    color: var(--text);
  }
  .labels .nm {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .labels .num {
    display: inline-grid;
    place-items: center;
    min-width: 15px;
    font-size: 10px;
    letter-spacing: 0.06em;
  }
  .ck {
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent-fg);
    animation: ob-pop 520ms var(--spring-e) both;
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
  }
  .rail,
  .fill {
    position: absolute;
    inset: 0;
    border-radius: 2px;
  }
  .rail {
    background: var(--line-strong);
  }
  .fill {
    background: var(--accent-fg);
    transform-origin: left center;
    transform: scaleX(var(--p));
    transition: transform 1100ms var(--soft-e);
  }
  .track i {
    position: absolute;
    top: 0;
    width: 3px;
    height: 100%;
    margin-left: -1.5px;
    background: var(--bg);
  }
  /* the glowing head rides the end of the fill (same curve, same time) and pings on each step */
  .head {
    position: absolute;
    inset: 0;
    pointer-events: none;
    transform: translate3d(calc(var(--p) * 100%), 0, 0);
    transition: transform 1100ms var(--soft-e);
  }
  .head b,
  .head em {
    position: absolute;
    left: -4px;
    top: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-fg);
  }
  .head b {
    box-shadow:
      0 0 0 3px var(--accent-soft),
      0 0 14px 2px var(--accent-line);
  }
  .head em {
    opacity: 0;
    animation: ob-ping 900ms 260ms var(--ease-out) both;
  }
  @keyframes ob-ping {
    from {
      opacity: 0.55;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(4.2);
    }
  }
  @keyframes ob-pop {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
  }

  /* ---------- stage */
  .stage {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    display: grid;
    /* long steps (Comportamento at the minimum window) scroll under a soft edge, not a hard cut */
    mask-image: linear-gradient(to bottom, transparent 0, #000 14px, #000 calc(100% - 28px), transparent 100%);
  }
  .step {
    grid-area: 1 / 1;
    /* "safe": when a step is taller than the stage it starts at the top instead of overflowing upward */
    align-self: safe center;
    justify-self: center;
    width: min(820px, calc(100% - 56px));
    padding: 28px 0 24px;
    will-change: transform, opacity;
    transform-origin: 50% 40%;
  }
  .step.stagger {
    --stagger: 55ms;
  }
  /* Staggered reveal: every block of the incoming step (and the cards inside nested .stagger
     groups) springs in from the travel direction, one after another — a cascade, not a slab. */
  .step > :global(*),
  .step :global(.stagger > *) {
    animation: ob-in 760ms var(--spring-e) backwards;
    animation-delay: calc(var(--base, 110ms) + var(--i, 0) * var(--stagger, 55ms));
  }
  .step :global(.stagger) {
    --base: 170ms;
  }
  /* wrappers of a nested cascade stay put; their children do the moving */
  .step > :global(.stagger),
  .step > .welcome {
    animation: none;
  }
  /* a first child must not inherit its group's own --i */
  .step > :global(:first-child),
  .step :global(.stagger > :first-child) {
    --i: 0;
  }
  @keyframes ob-in {
    from {
      opacity: 0;
      transform: translate3d(calc(var(--d, 1) * 26px), 10px, 0) scale(0.985);
    }
  }
  /* whole wizard recedes as it hands over to the app */
  .ob.bye .stage,
  .ob.bye .progress {
    opacity: 0;
    transform: scale(0.97) translate3d(0, 8px, 0);
    transition:
      opacity 340ms var(--ease),
      transform 380ms var(--ease-out);
  }
  .ob.bye .aura {
    opacity: 0;
    transform: scale(1.5);
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
  .pi.logo {
    border: 0;
    background: none;
    overflow: hidden;
  }
  .pi.logo :global(.pl) {
    border-radius: 10px;
  }
  .sel .pi.logo {
    transform: rotate(-4deg);
  }
  .sel .pi:not(.logo) {
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
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pick button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px 0 8px;
    border-radius: 18px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    font-size: 13px;
    color: var(--text-2);
    transition:
      transform var(--t) var(--spring),
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease),
      color var(--t) var(--ease);
  }
  .pick button :global(.pl) {
    border-radius: 50%;
    transition: transform var(--t-slow) var(--spring);
  }
  .pick button:hover {
    color: var(--text);
    border-color: var(--text-3);
  }
  .pick button:active {
    transform: scale(0.96);
    transition-duration: 60ms;
  }
  .pick button.sel {
    color: var(--text);
    font-weight: 600;
    border-color: var(--accent-line);
    background: linear-gradient(var(--accent-soft), var(--accent-soft)), var(--surface);
  }
  .pick button.sel :global(.pl) {
    transform: rotate(-8deg) scale(1.06);
  }
  .kh {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .khl {
    display: grid;
    flex: none;
    animation: fv-pop var(--t-slow) var(--spring) both;
  }
  .step :global(.ks) {
    margin-top: 20px;
  }
  .model {
    margin-top: 22px;
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
  .livebox {
    margin-top: 22px;
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    transition:
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease);
  }
  .livebox.on {
    border-color: var(--accent-line);
  }
  .liveopt {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
  }
  .livepick {
    padding: 2px 16px 16px;
    animation: ob-in 600ms var(--spring-e) backwards;
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
    /* pending/progress is information, not success: neutral; only "ready" turns --ok */
    color: var(--text-3);
    animation: fv-fade var(--t-slow) var(--ease) both;
  }
  .lo-st.ready {
    color: var(--ok);
  }

  /* ---------- test */
  .testkeys {
    margin-top: 18px;
  }
  .testarea {
    margin-top: 18px;
    font-size: 14px;
    resize: none;
    transition:
      border-color var(--t-slow) var(--ease),
      box-shadow var(--t-slow) var(--ease);
  }
  .testarea.won {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .tstatus {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--text-2);
    min-height: 56px;
  }
  .wait {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  /* listening mic: a slow breathing ring says "I'm waiting for you" */
  .mic {
    position: relative;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: var(--accent-fg);
    background: var(--accent-soft);
  }
  .mic::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 1px solid var(--accent-line);
    animation: ob-breathe 2.2s var(--ease) infinite;
  }
  @keyframes ob-breathe {
    0% {
      opacity: 0.9;
      transform: scale(1);
    }
    70%,
    100% {
      opacity: 0;
      transform: scale(1.7);
    }
  }

  /* ---------- success: disc fills, ring + tick draw, rays burst, two ripples, soft halo */
  .yay {
    display: flex;
    align-items: center;
    gap: 16px;
    animation: ob-in 700ms var(--spring-e) backwards;
  }
  .seal {
    position: relative;
    width: 48px;
    height: 48px;
    flex: none;
    color: var(--accent-fg);
  }
  .seal svg {
    position: relative;
    display: block;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    overflow: visible;
  }
  .disc {
    fill: var(--accent-soft);
    stroke: none;
    transform-box: fill-box;
    transform-origin: center;
    animation: ob-pop 620ms var(--spring-e) both;
  }
  .ring {
    stroke-width: 1.6;
    stroke-dasharray: 132;
    stroke-dashoffset: 132;
    transform: rotate(-90deg);
    transform-origin: center;
    animation: draw 560ms 80ms var(--ease-out) forwards;
  }
  .tick {
    stroke-width: 2.6;
    stroke-dasharray: 28;
    stroke-dashoffset: 28;
    animation: draw 340ms 420ms var(--ease-out) forwards;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  .halo,
  .ripple {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    pointer-events: none;
  }
  .halo {
    inset: -40px;
    background: radial-gradient(closest-side, var(--accent-line), transparent);
    opacity: 0;
    animation: ob-halo 1400ms 300ms var(--ease-out) both;
  }
  @keyframes ob-halo {
    0% {
      opacity: 0;
      transform: scale(0.4);
    }
    35% {
      opacity: 0.7;
    }
    100% {
      opacity: 0;
      transform: scale(1.25);
    }
  }
  .ripple {
    border: 1.5px solid var(--accent-fg);
    opacity: 0;
    animation: ob-ripple 1100ms 380ms var(--ease-out) both;
  }
  .ripple.r2 {
    animation-delay: 560ms;
  }
  @keyframes ob-ripple {
    from {
      opacity: 0.6;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(2.4);
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
    top: calc(var(--l) / -2);
    width: 2px;
    height: var(--l);
    border-radius: 2px;
    background: var(--accent-fg);
    opacity: 0;
    animation: ray 700ms 360ms var(--ease-out) both;
  }
  @keyframes ray {
    0% {
      opacity: 1;
      transform: rotate(var(--a)) translate3d(0, -24px, 0) scaleY(1);
    }
    100% {
      opacity: 0;
      transform: rotate(var(--a)) translate3d(0, -44px, 0) scaleY(0.2);
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
    font-size: 15px;
  }
  .yt span {
    font-size: 12.5px;
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
  /* background download (the user chose "continuar em segundo plano") */
  .bgchip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 26px;
    padding: 0 4px 0 9px;
    border-radius: 13px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    font-size: 10.5px;
    color: var(--text-2);
    animation: ob-in 600ms var(--spring-e) backwards;
  }
  .bgl {
    max-width: 16ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mini {
    position: relative;
    width: 44px;
    height: 3px;
    border-radius: 2px;
    background: var(--surface-3);
    overflow: hidden;
    flex: none;
  }
  .mini i {
    position: absolute;
    inset: 0;
    background: var(--accent-fg);
    transform-origin: left;
    transition: transform 260ms var(--ease);
  }
  .bgx {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--text-3);
  }
  .bgx:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  /* Finish after a successful test: one light sweep across the button */
  .fin {
    position: relative;
    display: inline-flex;
    border-radius: var(--r);
    overflow: hidden;
    animation: ob-pop 520ms var(--spring-e) backwards;
  }
  .fin::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(105deg, transparent 30%, rgba(255, 255, 255, 0.45) 50%, transparent 70%);
    transform: translate3d(-120%, 0, 0);
    animation: ob-sheen 1100ms 650ms var(--ease-out) both;
  }
  @keyframes ob-sheen {
    to {
      transform: translate3d(120%, 0, 0);
    }
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
  @media (max-width: 860px) {
    .bgl {
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

  /* ---------- capsule look (welcome): a live miniature over a colorful "wallpaper" */
  .capprev {
    position: relative;
    height: 58px;
    border-radius: var(--r);
    overflow: hidden;
    display: grid;
    place-items: center;
    background: #1b2a4a;
    isolation: isolate;
  }
  .wall i {
    position: absolute;
    border-radius: 50%;
    filter: blur(2px);
    z-index: -1;
  }
  .wall i:nth-child(1) {
    width: 120px;
    height: 120px;
    left: -10px;
    top: -40px;
    background: #ff6b4a;
  }
  .wall i:nth-child(2) {
    width: 90px;
    height: 90px;
    left: 44%;
    top: 14px;
    background: var(--accent);
  }
  .wall i:nth-child(3) {
    width: 130px;
    height: 130px;
    right: -30px;
    top: -60px;
    background: #5be3ff;
  }
  .pill {
    width: calc(100% - 28px);
    height: 36px;
    border-radius: 18px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    color: var(--cap-text);
    background: var(--cap-bg);
    border: 1px solid var(--cap-line);
    box-shadow: 0 6px 16px -6px rgba(0, 0, 0, 0.5);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    transition:
      background-color var(--t-slow) var(--ease),
      backdrop-filter var(--t-slow) var(--ease),
      border-color var(--t-slow) var(--ease);
  }
  .pill.glass,
  .pill.translucent {
    background: color-mix(in srgb, var(--cap-bg) 58%, transparent);
    border-color: rgba(255, 255, 255, 0.22);
    border-top-color: rgba(255, 255, 255, 0.42);
    backdrop-filter: blur(16px) saturate(1.3);
  }
  .pill.translucent {
    background: color-mix(in srgb, var(--cap-bg) 30%, transparent);
  }
  .bars {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .bars b {
    width: 2.5px;
    border-radius: 2px;
    background: var(--cap-wave);
    animation: capbar 1.1s var(--ease-in-out) infinite alternate;
  }
  @keyframes capbar {
    to {
      transform: scaleY(0.45);
    }
  }
  .said {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wglass {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--text-2);
  }
  @media (prefers-reduced-motion: reduce) {
    .bars b {
      animation: none;
    }
  }
</style>
