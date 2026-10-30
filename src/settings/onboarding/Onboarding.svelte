<script lang="ts">
  import { onMount } from "svelte";
  import { api, isTauri, on } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import Logo from "../../lib/icons/Logo.svelte";
  import type { IconName } from "../../lib/icons/paths";
  import { app, saveNow } from "../../lib/stores.svelte";
  import { EVENTS, type KeyProviderId, type LlmProviderId, type LocalModel, type ModelProgress, type SttProviderId } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import HotkeyInput from "../../lib/ui/HotkeyInput.svelte";
  import Keys from "../../lib/ui/Keys.svelte";
  import { keycapsOf } from "../../lib/ui/hotkeys";
  import Progress from "../../lib/ui/Progress.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";
  import TitleBar from "../../lib/ui/TitleBar.svelte";
  import { pname, STT_PROVIDERS } from "../providers";
  import KeyStep from "./KeyStep.svelte";

  const s = $derived(app.settings!);
  const TOTAL = 5;
  let step = $state(1);
  let dir = $state(1);
  type Path = "groq" | "offline" | "other";
  let path = $state<Path>("groq");
  let other = $state<KeyProviderId>("gemini");
  let keyOk = $state(false);

  // offline model
  let models = $state<LocalModel[]>([]);
  let prog = $state<ModelProgress | null>(null);
  const rec = $derived(models.find((m) => m.recommended) ?? models[0]);
  const modelReady = $derived(!!rec?.installed);

  // test
  let celebrated = $state(false);
  let testText = $state("");

  onMount(() => {
    api.listLocalModels().then((m) => (models = m));
    const offs = [
      on<ModelProgress>(EVENTS.modelProgress, async (p) => {
        prog = p;
        if (p.done) models = await api.listLocalModels();
      }),
      on(EVENTS.historyChanged, async () => {
        if (step !== 5) return;
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

  const PATHS: { id: Path; icon: IconName; title: () => string; body: () => string; tag?: boolean }[] = [
    { id: "groq", icon: "cloud", title: () => t("ob.path_groq"), body: () => t("ob.path_groq_body"), tag: true },
    { id: "offline", icon: "lock", title: () => t("ob.path_offline"), body: () => t("ob.path_offline_body") },
    { id: "other", icon: "key", title: () => t("ob.path_other"), body: () => t("ob.path_other_body") },
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

  function next() {
    if (step === 3) applyPath();
    dir = 1;
    step = Math.min(TOTAL, step + 1);
  }
  function back() {
    dir = -1;
    step = Math.max(1, step - 1);
  }
  async function finish() {
    s.onboarded = true;
    await saveNow();
  }
  async function download() {
    if (!rec) return;
    prog = { id: rec.id, downloaded: 0, total: rec.size_mb * 1048576, done: false, error: null };
    await api.downloadLocalModel(rec.id);
  }
  const canNext = $derived(step !== 3 || (path === "offline" ? modelReady : keyOk));
</script>

<div class="ob">
  <TitleBar maximizable={false}>
    {#snippet left()}
      <span class="brand" data-tauri-drag-region><Logo size={18} glow={false} /><span data-tauri-drag-region>FlowVoice</span></span>
    {/snippet}
  </TitleBar>

  <div class="progress" aria-label={t("ob.step", { n: step, total: TOTAL })}>
    {#each Array(TOTAL) as _, i}
      <span class:done={i < step}></span>
    {/each}
  </div>

  <div class="stage">
    {#key step}
      <section class="step" style="--dir:{dir}">
        {#if step === 1}
          <div class="welcome">
            <Logo size={76} />
            <h1>{t("ob.welcome_title")}</h1>
            <p class="lead">{t("ob.welcome_body")}</p>
            <div class="lang">
              <span class="label">{t("ob.language")}</span>
              <Segmented
                label={t("ob.language")}
                bind:value={s.ui_language}
                options={[
                  { value: "pt", label: "Português" },
                  { value: "en", label: "English" },
                  { value: "es", label: "Español" },
                ]}
              />
            </div>
          </div>
        {:else if step === 2}
          <h1>{t("ob.path_title")}</h1>
          <p class="lead">{t("ob.path_body")}</p>
          <div class="paths" role="radiogroup" aria-label={t("ob.path_title")}>
            {#each PATHS as p (p.id)}
              <button type="button" role="radio" aria-checked={path === p.id} class:sel={path === p.id} onclick={() => (path = p.id)}>
                <span class="pi"><Icon name={p.icon} size={22} /></span>
                <span class="pt">
                  <span class="ptt">{p.title()} {#if p.tag}<Badge tone="solid">{t("c.recommended")}</Badge>{/if}</span>
                  <span class="ptb">{p.body()}</span>
                </span>
                <span class="radio" aria-hidden="true"></span>
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
                  <Progress value={prog.total ? prog.downloaded / prog.total : 0} label={rec.label} />
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
          <h2 class="label sty">{t("ob.style")}</h2>
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
          <p class="sdesc muted">{s.styles.find((x) => x.id === s.active_style)?.description}</p>
        {:else}
          <h1>{t("ob.test_title")}</h1>
          <p class="lead">
            {t("ob.test_body", { hotkey: keycapsOf(s.hotkeys.dictate).join(" + ") })}
          </p>
          <div class="testkeys"><Keys accel={s.hotkeys.dictate} size="lg" /></div>
          <textarea class="field testarea" rows="4" placeholder={t("ob.test_placeholder")} bind:value={testText}></textarea>
          <div class="tstatus" class:yay={celebrated}>
            {#if celebrated}
              <span class="burst" aria-hidden="true">
                {#each Array(8) as _, i}<i style="--a:{i * 45}deg"></i>{/each}
              </span>
              <Icon name="check" size={18} stroke={2} />
              <span>{t("ob.test_success")}</span>
            {:else}
              <span class="pulse" aria-hidden="true"></span>
              <span>{t("ob.test_waiting")}</span>
              <Button size="sm" variant="ghost" icon="mic" onclick={() => api.startRecording("dictate")}>{t("ob.test_button")}</Button>
            {/if}
          </div>
        {/if}
      </section>
    {/key}
  </div>

  <footer>
    <span class="mono faint">{t("ob.step", { n: step, total: TOTAL })}</span>
    <div class="nav">
      {#if step > 1}
        <Button variant="ghost" icon="chevronLeft" onclick={back}>{t("c.back")}</Button>
      {/if}
      {#if step === 3 && !canNext}
        <Button variant="quiet" onclick={next}>{t("ob.skip_now")}</Button>
      {/if}
      {#if step === 1}
        <Button variant="primary" iconRight="arrowRight" onclick={next}>{t("ob.start")}</Button>
      {:else if step < TOTAL}
        <Button variant="primary" iconRight="arrowRight" disabled={!canNext} onclick={next}>{t("c.next")}</Button>
      {:else}
        <Button variant={celebrated ? "primary" : "default"} iconRight="check" onclick={finish}>{t("c.finish")}</Button>
      {/if}
    </div>
  </footer>
</div>

<style>
  .ob {
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
  .progress {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    width: min(520px, calc(100% - 64px));
    margin: 4px auto 0;
  }
  .progress span {
    height: 2px;
    border-radius: 2px;
    background: var(--line-strong);
    transition: background-color 240ms var(--ease);
  }
  .progress span.done {
    background: var(--accent);
  }
  .stage {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
  }
  .step {
    width: min(520px, calc(100% - 64px));
    margin: auto;
    padding: 28px 0 24px;
    animation: slide 240ms var(--ease) both;
  }
  @keyframes slide {
    from {
      opacity: 0;
      transform: translateX(calc(var(--dir) * 14px));
    }
  }
  h1 {
    font-size: 28px;
    line-height: 1.15;
    letter-spacing: -0.03em;
  }
  .lead {
    margin-top: 8px;
    color: var(--text-2);
    font-size: 14px;
    max-width: 50ch;
  }
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .welcome h1 {
    margin-top: 26px;
    font-size: 36px;
  }
  .lang {
    margin-top: 30px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .paths {
    margin-top: 22px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .paths button {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 12px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    text-align: left;
    transition: border-color var(--t-fast);
  }
  .paths button:hover {
    border-color: var(--text-3);
  }
  .paths button.sel {
    border-color: var(--accent-line);
    background: linear-gradient(var(--accent-soft), var(--accent-soft)), var(--surface);
  }
  .pi {
    color: var(--text-2);
  }
  .sel .pi {
    color: var(--accent-fg);
  }
  .pt {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .ptt {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 14.5px;
  }
  .ptb {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .radio {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    display: grid;
    place-items: center;
    flex: none;
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
  }
  .err {
    color: var(--danger-fg);
  }
  .hk {
    margin-top: 22px;
  }
  .sty {
    margin-top: 28px;
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
  }
  .chips button:hover {
    color: var(--text);
    border-color: var(--text-3);
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
  }
  .testkeys {
    margin-top: 18px;
  }
  .testarea {
    margin-top: 18px;
    font-size: 14px;
    resize: none;
  }
  .tstatus {
    position: relative;
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--text-2);
    min-height: 32px;
  }
  .tstatus.yay {
    color: var(--accent-fg);
    font-weight: 600;
  }
  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.4s var(--ease) infinite;
  }
  @keyframes pulse {
    0% {
      box-shadow: 0 0 0 0 var(--accent-line);
    }
    100% {
      box-shadow: 0 0 0 10px transparent;
    }
  }
  .burst {
    position: absolute;
    left: 9px;
    top: 50%;
    width: 0;
    height: 0;
  }
  .burst i {
    position: absolute;
    width: 2px;
    height: 7px;
    background: var(--accent);
    border-radius: 2px;
    transform: rotate(var(--a)) translateY(-6px);
    animation: burst 520ms var(--ease) both;
  }
  @keyframes burst {
    to {
      transform: rotate(var(--a)) translateY(-18px);
      opacity: 0;
    }
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: min(520px, calc(100% - 64px));
    margin: 0 auto;
    padding: 14px 0 22px;
    border-top: 1px solid var(--line);
  }
  .nav {
    display: flex;
    gap: 6px;
  }
  footer .faint {
    font-size: 11px;
  }
</style>
