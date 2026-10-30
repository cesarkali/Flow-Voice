<script lang="ts">
  // The floating dictation overlay. ONE element (.shell) morphs between phases
  // (width/height/radius); the inner content cross-fades on top of it.
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { api, on } from "../lib/api";
  import { langName, setLanguage, t, TARGET_LANGS } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import { applyTheme } from "../lib/theme";
  import { EVENTS, type DictationResult, type OverlayEvent, type Settings } from "../lib/types";
  import { pname } from "../settings/providers";

  /** Margin between the card and the native window edge (room for the shadow). */
  const M = 22;
  const BARS = 22;
  const ENV = Array.from({ length: BARS }, (_, i) => Math.pow(Math.sin((Math.PI * (i + 0.5)) / BARS), 0.7));

  let ev = $state<OverlayEvent>({ phase: "hidden" });
  let settings = $state<Settings | null>(null);
  let partial = $state("");
  let elapsed = $state(0);
  let listenStart = 0;

  let result = $state<DictationResult | null>(null);
  let pastedText = $state<string | null>(null);
  let showOriginal = $state(false);
  let busy = $state<string | null>(null);
  let picker = $state(false);
  let hovered = $state(false);
  let cycle = $state(0);
  let copied = $state(false);
  let detailOpen = $state(false);
  let actionError = $state("");

  type View = "listening" | "processing" | "done" | "error" | "answer";
  const visible = $derived(ev.phase !== "hidden");
  let view = $state<View>("listening");
  let lastError = $state<Extract<OverlayEvent, { phase: "error" }> | null>(null);
  let listenInfo = $state({ mode: "", style: "" });
  let procLabel = $state("");

  const live = $derived(ev.phase === "listening" && ev.live);
  const hasText = $derived(live && partial.trim().length > 0);
  const width = $derived(
    view === "listening" ? (hasText ? 460 : 392) : view === "processing" ? 300 : view === "done" ? 508 : view === "error" ? 460 : 250,
  );
  let contentH = $state(54);
  const radius = $derived(view === "done" || view === "error" ? 20 : view === "listening" && hasText ? 24 : 28);
  const position = $derived(settings?.overlay_position ?? "bottom");

  // ------------------------------------------------------------ sounds (tiny synthesized blips, no assets)
  let audioCtx: AudioContext | null = null;
  function blip(kind: "start" | "done" | "error") {
    if (!settings?.sounds) return;
    try {
      audioCtx ??= new AudioContext();
      const ctx = audioCtx;
      const notes = kind === "start" ? [660, 990] : kind === "done" ? [880, 1320] : [330, 247];
      notes.forEach((f, i) => {
        const o = ctx.createOscillator();
        const g = ctx.createGain();
        const t0 = ctx.currentTime + i * 0.07;
        o.type = "sine";
        o.frequency.value = f;
        g.gain.setValueAtTime(0, t0);
        g.gain.linearRampToValueAtTime(0.07, t0 + 0.012);
        g.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.12);
        o.connect(g).connect(ctx.destination);
        o.start(t0);
        o.stop(t0 + 0.13);
      });
    } catch {
      /* audio unavailable */
    }
  }

  // ------------------------------------------------------------ events
  function onState(e: OverlayEvent) {
    if (e.phase === "listening" || e.phase === "done" || e.phase === "error") {
      blip(e.phase === "listening" ? "start" : e.phase);
    }
    if (e.phase !== "hidden") view = e.phase === "transcribing" || e.phase === "refining" ? "processing" : e.phase;
    if (e.phase === "listening") {
      partial = "";
      elapsed = 0;
      listenStart = performance.now();
      level = 0;
      const mode =
        e.mode === "translate"
          ? t("ov.translate_to", { lang: (e.target ?? settings?.translation_target ?? "en").toUpperCase() })
          : e.mode === "ask"
            ? t("mode.ask")
            : t("mode.dictate");
      listenInfo = { mode, style: e.mode === "ask" ? "" : styleName(e.style_id) };
    }
    if (e.phase === "transcribing") procLabel = t("ov.transcribing");
    if (e.phase === "refining") procLabel = t("ov.refining", { provider: pname(e.provider) });
    if (e.phase === "done") {
      result = e.result;
      pastedText = e.result.pasted ? e.result.text : null;
      showOriginal = false;
      picker = false;
      busy = null;
      copied = false;
      actionError = "";
      cycle++;
    }
    if (e.phase === "error") {
      lastError = e;
      detailOpen = false;
      copied = false;
      cycle++;
    }
    if (e.phase === "answer") {
      setTimeout(() => {
        if (ev.phase === "answer") ev = { phase: "hidden" };
      }, 1400);
    }
    if (e.phase !== "done" && e.phase !== "error" && hovered) {
      hovered = false;
      api.overlayHold(false);
    }
    ev = e;
  }

  onMount(() => {
    api.getSettings().then((s) => {
      settings = s;
      applyTheme(s.theme);
      setLanguage(s.ui_language);
    });
    const subs = [
      on<OverlayEvent>(EVENTS.overlayState, onState),
      on<number>(EVENTS.overlayLevel, (l) => (level = Math.max(level, Number(l) || 0))),
      on<{ text: string }>(EVENTS.overlayPartial, (p) => (partial = p?.text ?? "")),
      on<Settings>(EVENTS.settingsChanged, (s) => {
        settings = s;
        applyTheme(s.theme);
        setLanguage(s.ui_language);
      }),
    ];
    return () => subs.forEach((p) => p.then((f) => f()));
  });

  // ------------------------------------------------------------ waveform (rAF only while listening)
  const bars: HTMLElement[] = [];
  let level = 0;
  let smooth = 0;
  $effect(() => {
    if (!(visible && view === "listening")) return;
    let raf = 0;
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const loop = (now: number) => {
      smooth += (level - smooth) * (level > smooth ? 0.45 : 0.18);
      level *= 0.86; // decays if events stop
      const amp = Math.min(1, smooth * 1.5);
      for (let i = 0; i < BARS; i++) {
        const el = bars[i];
        if (!el) continue;
        const wob = reduce ? 0.8 : 0.55 + 0.45 * Math.sin(now / (120 + i * 11) + i * 1.9);
        el.style.transform = `scaleY(${(0.09 + amp * ENV[i] * wob * 0.91).toFixed(3)})`;
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    const timer = setInterval(() => (elapsed = Math.floor((performance.now() - listenStart) / 1000)), 250);
    return () => {
      cancelAnimationFrame(raf);
      clearInterval(timer);
    };
  });

  // ------------------------------------------------------------ native window size hints
  let sent = { w: 0, h: 0 };
  let shrinkTimer = 0;
  $effect(() => {
    if (!visible) return;
    const w = Math.ceil(width + M * 2);
    const h = Math.ceil(contentH + M * 2);
    clearTimeout(shrinkTimer);
    if (w > sent.w || h > sent.h) {
      // grow first (to the union so the morph never clips), shrink after the animation
      sent = { w: Math.max(w, sent.w), h: Math.max(h, sent.h) };
      api.overlayResize(sent.w, sent.h);
    }
    if (w < sent.w || h < sent.h) {
      shrinkTimer = window.setTimeout(() => {
        sent = { w, h };
        api.overlayResize(w, h);
      }, 480);
    }
  });

  let measureSeq = 0;
  function measure(node: HTMLElement) {
    // Only the newest content node drives the height (the old one may still be fading out).
    const id = ++measureSeq;
    const ro = new ResizeObserver(() => {
      if (id === measureSeq) contentH = node.offsetHeight;
    });
    ro.observe(node);
    contentH = node.offsetHeight;
    return { destroy: () => ro.disconnect() };
  }

  // ------------------------------------------------------------ actions
  const fmt = (s: number) => `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  const styleName = (id: string | undefined) => settings?.styles.find((s) => s.id === id)?.name ?? id ?? "";
  const displayText = $derived(result ? (showOriginal ? result.raw : result.text) : "");
  const words = $derived(partial.trim() ? partial.trim().split(/\s+/) : []);

  function dismiss() {
    if (hovered) api.overlayHold(false);
    hovered = false;
    ev = { phase: "hidden" };
    api.dismissOverlay();
  }

  async function reprocess(styleId: string, target: string | null) {
    if (!result || busy) return;
    busy = target ? `lang:${target}` : styleId;
    actionError = "";
    try {
      result = await api.reprocess(styleId, target);
      showOriginal = false;
      picker = false;
    } catch (e) {
      actionError = String(e);
    } finally {
      busy = null;
      cycle++;
    }
  }
  async function paste(replace: boolean) {
    const text = displayText;
    try {
      await api.pasteText(text, replace);
      pastedText = text;
    } catch (e) {
      actionError = String(e);
    }
    cycle++;
  }
  async function copy(text: string) {
    await api.copyText(text);
    copied = true;
    setTimeout(() => (copied = false), 1300);
    cycle++;
  }
  function hold(v: boolean) {
    if (view !== "done" && view !== "error") return;
    if (hovered === v) return;
    hovered = v;
    api.overlayHold(v);
  }
  function key(e: KeyboardEvent) {
    if (e.key !== "Escape" || !visible) return;
    if (picker) {
      picker = false;
      return;
    }
    if (ev.phase === "listening") api.cancelRecording();
    else dismiss();
  }
</script>

<svelte:window onkeydown={key} />

<div class="stage {position}" style="--m:{M}px">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="shell v-{view}"
    class:hidden={!visible}
    style="width:{width}px;height:{contentH}px;border-radius:{radius}px"
    role="region"
    aria-label="FlowVoice"
    aria-live="polite"
    onmouseenter={() => hold(true)}
    onmouseleave={() => hold(false)}
  >
    {#key view}
      <div class="content" style="width:{width}px" use:measure in:fade={{ duration: 160, delay: 70 }} out:fade={{ duration: 80 }}>
        {#if view === "listening"}
          {#if live}
            <div class="live" class:has={hasText}>
              <p>
                {#each words as w, i (i)}<span class:old={words.length - i > 7}>{w + " "}</span>{/each}
              </p>
            </div>
          {/if}
          <div class="row">
            <span class="rec" aria-hidden="true"></span>
            <div class="wave" aria-hidden="true">
              {#each ENV as _, i}
                <i bind:this={bars[i]}></i>
              {/each}
            </div>
            <div class="meta">
              <span class="mode">{listenInfo.mode}</span>
              {#if listenInfo.style}<span class="sty">{listenInfo.style}</span>{/if}
            </div>
            <span class="time tnum">{fmt(elapsed)}</span>
            <button type="button" class="stop" aria-label={t("ov.stop")} title={t("ov.stop")} onclick={() => api.stopRecording()}>
              <Icon name="stop" size={16} />
            </button>
            <button type="button" class="x" aria-label={t("ov.cancel")} title={t("ov.cancel")} onclick={() => api.cancelRecording()}>
              <Icon name="x" size={16} />
            </button>
          </div>
        {:else if view === "processing"}
          <div class="row proc">
            <div class="scan" aria-hidden="true">
              {#each ENV as e, i}
                {#if i % 2 === 0}<i style="transform:scaleY({0.18 + e * 0.5})"></i>{/if}
              {/each}
              <span class="sweep"></span>
            </div>
            <span class="plabel">{procLabel}</span>
          </div>
        {:else if view === "answer"}
          <div class="row proc">
            <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
            <span class="plabel">{t("ov.answer")}</span>
          </div>
        {:else if view === "done" && result}
          <div class="card">
            <header>
              <span class="ok"><Icon name="check" size={15} stroke={2} /></span>
              <span class="ht">{pastedText !== null ? t("ov.pasted") : t("ov.copied")}</span>
              <span class="hm">
                {#if showOriginal}
                  {t("ov.showing_original")}
                {:else if result.target}
                  {t("ov.translate_to", { lang: langName(result.target) })}
                {:else}
                  {styleName(result.style_id)}
                {/if}
                {#if result.provider && !showOriginal}<span class="via">· {result.provider}</span>{/if}
              </span>
              <button type="button" class="x" aria-label={t("ov.dismiss")} title={t("ov.dismiss")} onclick={dismiss}>
                <Icon name="x" size={16} />
              </button>
            </header>

            <div class="text" class:busy={!!busy} class:orig={showOriginal}>{displayText}</div>

            {#if actionError}
              <p class="warnline err">{actionError}</p>
            {:else if result.warnings.length}
              <p class="warnline">{result.warnings.join(" · ")}</p>
            {/if}

            <div
              class="chips"
              role="toolbar"
              aria-label={picker ? t("ov.translate_pick") : t("nav.styles")}
              onwheel={(e) => {
                if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
                  e.currentTarget.scrollLeft += e.deltaY;
                  e.preventDefault();
                }
              }}
            >
              {#if picker}
                <button type="button" class="chip back" aria-label={t("c.back")} onclick={() => (picker = false)}>
                  <Icon name="chevronLeft" size={14} />
                </button>
                {#each TARGET_LANGS.slice(0, 10) as l}
                  <button
                    type="button"
                    class="chip"
                    class:on={result.target === l}
                    class:loading={busy === `lang:${l}`}
                    disabled={!!busy}
                    onclick={() => reprocess(result!.style_id, l)}
                  >
                    <span class="code">{l.toUpperCase()}</span>{langName(l)}
                  </button>
                {/each}
              {:else}
                {#each settings?.styles ?? [] as st (st.id)}
                  <button
                    type="button"
                    class="chip"
                    class:on={!showOriginal && !result.target && result.style_id === st.id}
                    class:loading={busy === st.id}
                    disabled={!!busy}
                    onclick={() => reprocess(st.id, null)}
                  >
                    {st.name}
                  </button>
                {/each}
                <button type="button" class="chip" class:on={!!result.target && !showOriginal} disabled={!!busy} onclick={() => (picker = true)}>
                  <Icon name="translate" size={14} />{t("ov.translate")}{result.target ? ` · ${result.target.toUpperCase()}` : ""}
                </button>
                <button type="button" class="chip" class:on={showOriginal} disabled={!!busy} onclick={() => (showOriginal = !showOriginal)}>
                  {t("ov.original")}
                </button>
              {/if}
            </div>

            <div class="acts">
              <span class="esc mono">esc</span>
              {#if pastedText !== null}
                <button
                  type="button"
                  class="btn"
                  class:primary={displayText !== pastedText}
                  disabled={displayText === pastedText || !!busy}
                  title={t("ov.replace_hint")}
                  onclick={() => paste(true)}
                >
                  <Icon name="replace" size={15} />{t("ov.replace")}
                </button>
              {/if}
              <button type="button" class="btn" class:primary={pastedText === null} disabled={!!busy} onclick={() => paste(false)}>
                <Icon name="paste" size={15} />{t("ov.paste")}
              </button>
              <button type="button" class="btn" disabled={!!busy} onclick={() => copy(displayText)}>
                <Icon name={copied ? "check" : "copy"} size={15} />{copied ? t("c.copied") : t("ov.copy")}
              </button>
            </div>
          </div>
        {:else if view === "error" && lastError}
          <div class="card err">
            <header>
              <span class="bad"><Icon name="alert" size={15} /></span>
              <span class="ht">{t("ov.error")}</span>
              <span class="hm"></span>
              <button type="button" class="x" aria-label={t("ov.dismiss")} title={t("ov.dismiss")} onclick={dismiss}>
                <Icon name="x" size={16} />
              </button>
            </header>
            <p class="emsg">{lastError.message}</p>
            {#if lastError.detail}
              <button type="button" class="dtoggle" aria-expanded={detailOpen} onclick={() => (detailOpen = !detailOpen)}>
                <Icon name={detailOpen ? "chevronDown" : "chevronRight"} size={14} />
                {detailOpen ? t("ov.hide_details") : t("ov.details")}
              </button>
              {#if detailOpen}
                <pre class="detail">{lastError.detail}</pre>
              {/if}
            {/if}
            <div class="acts">
              {#if lastError.raw}
                <button type="button" class="btn primary" onclick={() => copy(lastError!.raw!)}>
                  <Icon name={copied ? "check" : "copy"} size={15} />{copied ? t("c.copied") : t("ov.copy_raw")}
                </button>
              {/if}
              <button
                type="button"
                class="btn"
                onclick={() => {
                  api.openSettings("providers");
                  dismiss();
                }}
              >
                <Icon name="key" size={15} />{t("ov.open_settings")}
              </button>
            </div>
          </div>
        {/if}

        {#if view === "done" || view === "error"}
          {#key cycle}
            <div
              class="life"
              class:paused={hovered || !!busy || picker || detailOpen}
              style="animation-duration:{view === 'error' ? 12000 : 6000}ms"
              onanimationend={() => visible && dismiss()}
            ></div>
          {/key}
        {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  :global(html),
  :global(body.overlay) {
    background: transparent !important;
  }

  .stage {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-end;
    padding: var(--m);
    pointer-events: none;
    --cap-spring: cubic-bezier(0.2, 0.9, 0.25, 1.15);
  }
  .stage.top {
    align-items: flex-start;
  }

  .shell {
    position: relative;
    flex: none;
    max-width: calc(100vw - var(--m) * 2);
    max-height: calc(100vh - var(--m) * 2);
    background: var(--cap-bg);
    color: var(--cap-text);
    border: 1px solid var(--cap-line);
    box-shadow:
      0 6px 18px rgba(0, 0, 0, 0.32),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
    overflow: hidden;
    pointer-events: auto;
    contain: layout paint style;
    transform-origin: 50% 100%;
    transition:
      width 420ms var(--cap-spring),
      height 420ms var(--cap-spring),
      border-radius 320ms var(--ease),
      background-color 200ms var(--ease),
      border-color 200ms var(--ease),
      opacity 160ms var(--ease),
      transform 320ms var(--cap-spring);
    font-size: 13px;
  }
  .top .shell {
    transform-origin: 50% 0;
  }
  .shell.hidden {
    opacity: 0;
    transform: translateY(10px) scale(0.92);
    pointer-events: none;
    transition:
      opacity 140ms var(--ease),
      transform 200ms var(--ease);
  }
  .top .shell.hidden {
    transform: translateY(-10px) scale(0.92);
  }
  .shell.v-error {
    border-color: rgba(255, 90, 78, 0.45);
    background: color-mix(in srgb, var(--cap-bg) 90%, #ff5a4e);
  }

  .content {
    position: absolute;
    left: 0;
    bottom: 0;
  }
  .top .content {
    top: 0;
    bottom: auto;
  }

  /* -------- listening */
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 54px;
    padding: 0 9px 0 18px;
  }
  .rec {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--cap-accent-line);
    box-shadow: 0 0 8px var(--cap-accent-line);
    flex: none;
    animation: rec 1.6s ease-in-out infinite;
  }
  @keyframes rec {
    50% {
      opacity: 0.45;
    }
  }
  .wave {
    display: flex;
    align-items: center;
    gap: 2.5px;
    height: 28px;
    flex: none;
  }
  .wave i {
    display: block;
    width: 3px;
    height: 28px;
    border-radius: 2px;
    background: var(--cap-wave);
    transform: scaleY(0.09);
    will-change: transform;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.25;
    padding-left: 4px;
  }
  .mode {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--cap-text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sty {
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .time {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--cap-text-2);
  }
  .stop,
  .x {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    flex: none;
    color: var(--cap-text-2);
    transition:
      background-color var(--t-fast),
      color var(--t-fast);
  }
  .stop {
    background: var(--cap-hover);
    color: var(--cap-text);
  }
  .stop:hover {
    background: var(--cap-accent);
    color: #0b0b0c;
  }
  .x:hover {
    background: var(--cap-hover);
    color: var(--cap-text);
  }
  .x {
    width: 30px;
    height: 30px;
  }

  .live {
    padding: 0 22px;
    max-height: 0;
    overflow: hidden;
  }
  .live.has {
    max-height: 70px;
    padding-top: 14px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    height: 58px;
    -webkit-mask-image: linear-gradient(to bottom, transparent 4px, #000 26px);
    mask-image: linear-gradient(to bottom, transparent 4px, #000 26px);
  }
  .live p {
    flex: none;
    font-size: 14.5px;
    line-height: 21px;
  }
  .live span {
    animation: word 260ms var(--ease) both;
    transition: color 600ms var(--ease);
  }
  .live span.old {
    color: var(--cap-text-2);
  }
  @keyframes word {
    from {
      opacity: 0;
    }
  }

  /* -------- processing */
  .proc {
    justify-content: center;
    gap: 12px;
    padding: 0 22px;
  }
  .scan {
    position: relative;
    display: flex;
    align-items: center;
    gap: 3px;
    height: 22px;
    overflow: hidden;
    flex: none;
  }
  .scan i {
    display: block;
    width: 3px;
    height: 22px;
    border-radius: 2px;
    background: var(--cap-text-2);
    opacity: 0.55;
  }
  .sweep {
    position: absolute;
    inset: 0 auto 0 0;
    width: 18px;
    background: linear-gradient(90deg, transparent, var(--cap-accent-line), transparent);
    animation: sweep 1.1s var(--ease) infinite;
  }
  @keyframes sweep {
    from {
      transform: translateX(-20px);
    }
    to {
      transform: translateX(70px);
    }
  }
  .plabel {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dots {
    display: flex;
    gap: 4px;
  }
  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--cap-accent-line);
    animation: dot 900ms ease-in-out infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 150ms;
  }
  .dots i:nth-child(3) {
    animation-delay: 300ms;
  }
  @keyframes dot {
    50% {
      transform: translateY(-3px);
      opacity: 0.4;
    }
  }

  /* -------- result / error card */
  .card {
    padding: 12px 14px 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    margin-right: -6px;
  }
  .ok,
  .bad {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--cap-accent);
    color: #0b0b0c;
    flex: none;
  }
  .bad {
    background: #ff5a4e;
    color: #fff;
  }
  .ht {
    font-weight: 600;
    font-size: 13px;
  }
  .hm {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--cap-text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .via {
    font-family: var(--mono);
    font-size: 10.5px;
  }
  .text {
    font-size: 14px;
    line-height: 20px;
    max-height: 100px;
    overflow-y: auto;
    white-space: pre-wrap;
    user-select: text;
    padding-right: 4px;
    transition: opacity var(--t) var(--ease);
  }
  .text.busy {
    opacity: 0.4;
  }
  .text.orig {
    color: var(--cap-text-2);
    font-style: italic;
  }
  .warnline {
    margin-top: -4px;
    font-size: 11px;
    color: var(--cap-text-2);
    font-family: var(--mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .warnline.err {
    color: #ff7a70;
    white-space: normal;
  }
  .chips {
    display: flex;
    gap: 5px;
    overflow-x: auto;
    scrollbar-width: none;
    margin: 0 -14px 0 -16px;
    padding: 0 14px 0 16px;
    -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 14px, #000 calc(100% - 20px), transparent);
    mask-image: linear-gradient(90deg, transparent 0, #000 14px, #000 calc(100% - 20px), transparent);
  }
  .chips::-webkit-scrollbar {
    display: none;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 27px;
    padding: 0 11px;
    border-radius: 14px;
    border: 1px solid var(--cap-line);
    font-size: 12px;
    color: var(--cap-text-2);
    white-space: nowrap;
    flex: none;
    position: relative;
    transition:
      background-color var(--t-fast),
      color var(--t-fast),
      border-color var(--t-fast);
  }
  .chip:hover:not(:disabled) {
    color: var(--cap-text);
    background: var(--cap-hover);
  }
  .chip.on {
    color: var(--cap-text);
    border-color: var(--cap-accent-line);
  }
  .chip:disabled:not(.loading) {
    opacity: 0.5;
  }
  .chip.back {
    padding: 0 7px;
  }
  .code {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--cap-text-2);
  }
  .chip.loading {
    color: var(--cap-text);
    border-color: var(--cap-accent-line);
    overflow: hidden;
  }
  .chip.loading::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--cap-accent-soft), transparent);
    animation: chip 900ms linear infinite;
  }
  @keyframes chip {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(100%);
    }
  }
  .acts {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }
  .esc {
    margin-right: auto;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--cap-text-2);
    opacity: 0.7;
    border: 1px solid var(--cap-line);
    border-radius: 4px;
    padding: 1px 5px;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 500;
    border: 1px solid var(--cap-line);
    color: var(--cap-text);
    transition:
      background-color var(--t-fast),
      border-color var(--t-fast);
  }
  .btn:hover:not(:disabled) {
    background: var(--cap-hover);
  }
  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #0b0b0c;
    font-weight: 600;
  }
  .btn.primary:hover:not(:disabled) {
    background: #e3ff74;
  }
  .btn:disabled {
    opacity: 0.4;
  }
  .emsg {
    font-size: 13.5px;
    line-height: 1.45;
  }
  .dtoggle {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    font-size: 12px;
    color: var(--cap-text-2);
    margin-top: -4px;
  }
  .dtoggle:hover {
    color: var(--cap-text);
  }
  .detail {
    margin: 0;
    max-height: 84px;
    overflow: auto;
    padding: 8px 10px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.25);
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
    color: var(--cap-text);
  }
  .life {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--cap-accent-line);
    opacity: 0.7;
    transform-origin: left;
    animation-name: life;
    animation-timing-function: linear;
    animation-fill-mode: both;
  }
  .v-error .life {
    background: #ff5a4e;
  }
  .life.paused {
    animation-play-state: paused;
  }
  @keyframes life {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .shell {
      transition-duration: 1ms;
    }
    .life {
      animation-duration: 8000ms !important;
      animation-iteration-count: 1 !important;
    }
  }
</style>
