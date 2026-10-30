<script lang="ts">
  // The floating dictation overlay. ONE element (.shell) morphs between phases: its size and
  // radius are animated imperatively (Web Animations, FLIP from the current rendered size) while
  // the content layers cross-fade on top of it, absolutely positioned so nothing reflows.
  // The waveform lives outside the layers so the same bars carry over from "listening" into
  // the processing wave.
  import { onMount, tick, untrack } from "svelte";
  import { api, on } from "../lib/api";
  import { langName, setLanguage, t, TARGET_LANGS } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import { applyAccent, applyTheme } from "../lib/theme";
  import { EVENTS, type DictationResult, type OverlayEvent, type Settings } from "../lib/types";
  import { pname } from "../settings/providers";

  /** Margin between the card and the native window edge (room for the shadow). */
  const M = 22;
  /** Native window limits enforced by Rust (overlay_resize clamps to these). */
  const MAX_W = 760;
  const MAX_H = 560;
  const ROW_H = 54;
  const BARS = 20;
  const BAR_W = 3;
  const BAR_GAP = 2.5;
  const WAVE_W = BARS * BAR_W + (BARS - 1) * BAR_GAP;
  const PAD_L = 20;
  const EASE = "cubic-bezier(0.22, 1, 0.36, 1)";
  const MORPH_MS = 460;
  /** Bar envelope (center taller) — edges still move so the whole wave feels alive. */
  const ENV = Array.from({ length: BARS }, (_, i) => 0.42 + 0.58 * Math.pow(Math.sin((Math.PI * (i + 0.5)) / BARS), 1.3));
  /** History delay (frames) per bar: speech ripples outward from the center. */
  const LAG = Array.from({ length: BARS }, (_, i) => Math.round(Math.abs(i - (BARS - 1) / 2) * 1.15));
  const SIZES = {
    compact: { card: 440, cap: 368, live: 440, err: 440 },
    normal: { card: 600, cap: 388, live: 500, err: 520 },
    wide: { card: 700, cap: 400, live: 560, err: 560 },
  } as const;

  const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  let ev = $state<OverlayEvent>({ phase: "hidden" });
  let settings = $state<Settings | null>(null);
  let partial = $state("");
  let elapsed = $state(0);
  let listenStart = 0;

  let result = $state<DictationResult | null>(null);
  /** Text currently sitting in the target app (null = nothing was pasted). */
  let pastedText = $state<string | null>(null);
  let showOriginal = $state(false);
  let pending = $state<{ key: string; id: string; seq: number } | null>(null);
  let pseq = 0;
  let picker = $state(false);
  let hovered = $state(false);
  let cycle = $state(0);
  let copied = $state(false);
  let detailOpen = $state(false);
  let actionError = $state("");
  let errSeq = $state(0);

  type View = "listening" | "processing" | "done" | "error" | "answer";
  const visible = $derived(ev.phase !== "hidden");
  let view = $state<View>("listening");
  let lastError = $state<Extract<OverlayEvent, { phase: "error" }> | null>(null);
  let listenInfo = $state({ mode: "", style: "" });
  let procLabel = $state("");

  const live = $derived(ev.phase === "listening" && ev.live);
  const hasText = $derived(live && partial.trim().length > 0);
  const position = $derived(settings?.overlay_position ?? "bottom");
  const sz = $derived(SIZES[settings?.overlay_width ?? "normal"] ?? SIZES.normal);
  const fit = $derived(view === "processing" || view === "answer");
  const layerKey = $derived(view === "done" ? `done:${result?.id ?? ""}` : view === "error" ? `error:${errSeq}` : view);
  /** Size of the newest content layer, tagged with the layer it came from. */
  let measured = $state({ key: "", w: 0, h: 0 });
  const fixedW = $derived(
    view === "listening" ? (hasText ? sz.live : sz.cap) : view === "done" ? sz.card : view === "error" ? sz.err : 300,
  );
  const targetW = $derived(Math.min(MAX_W - M * 2, fit && measured.key === layerKey ? measured.w : fixedW));
  const targetR = $derived(view === "done" || view === "error" ? 18 : view === "listening" && hasText ? 22 : ROW_H / 2 + 1);
  const waveMode = $derived(!visible ? "off" : view === "listening" ? "listen" : fit ? "proc" : "off");

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
  function applyUpdate(r: DictationResult) {
    // A reprocessed version of the card's result: swap in place, keep everything else.
    pending = null;
    result = r;
    showOriginal = false;
    picker = false;
    actionError = "";
    cycle++;
  }

  function onState(e: OverlayEvent) {
    // While a reprocess runs the card stays put; stray pipeline phases are ignored.
    if (pending && (e.phase === "refining" || e.phase === "transcribing")) return;
    if (e.phase === "done" && result && e.result.id === result.id && ev.phase === "done") {
      applyUpdate(e.result);
      ev = e;
      return;
    }
    if (e.phase === "listening" || e.phase === "done" || e.phase === "error") {
      blip(e.phase === "listening" ? "start" : e.phase);
    }
    if (e.phase === "hidden") {
      snapNext = true;
    } else {
      if (ev.phase === "hidden") snapNext = true;
      view = e.phase === "transcribing" || e.phase === "refining" ? "processing" : e.phase;
    }
    if (e.phase === "listening") {
      partial = "";
      elapsed = 0;
      listenStart = performance.now();
      level = 0;
      amp = 0;
      peak = Math.max(0.28, peak * 0.8);
      pending = null;
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
    if (e.phase === "answer") procLabel = t("ov.answer");
    if (e.phase === "done") {
      result = e.result;
      pastedText = e.result.pasted ? e.result.text : null;
      showOriginal = false;
      picker = false;
      pending = null;
      copied = false;
      actionError = "";
      cycle++;
    }
    if (e.phase === "error") {
      lastError = e;
      errSeq++;
      detailOpen = false;
      copied = false;
      pending = null;
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
      applyAccent(s.accent_color);
      setLanguage(s.ui_language);
    });
    const subs = [
      on<OverlayEvent>(EVENTS.overlayState, onState),
      on<number>(EVENTS.overlayLevel, (l) => {
        const v = Math.min(1, Math.max(0, Number(l) || 0));
        level = Math.max(level, v);
      }),
      on<{ text: string }>(EVENTS.overlayPartial, (p) => {
        if (ev.phase === "listening") partial = p?.text ?? "";
      }),
      on<Settings>(EVENTS.settingsChanged, (s) => {
        settings = s;
        applyTheme(s.theme);
        applyAccent(s.accent_color);
        setLanguage(s.ui_language);
      }),
    ];
    return () => subs.forEach((p) => p.then((f) => f()));
  });

  // ------------------------------------------------------------ shell morph + native window size
  let shellEl = $state<HTMLDivElement>();
  let morphAnim: Animation | null = null;
  let cur = { w: 0, h: 0, r: 0 };
  let sent = { w: 0, h: 0 };
  let snapNext = $state(true);
  let settleTimer = 0;

  const winSize = (w: number, h: number) => ({
    w: Math.min(MAX_W, Math.ceil(w + M * 2)),
    h: Math.min(MAX_H, Math.ceil(h + M * 2)),
  });
  function growWindow(w: number, h: number) {
    const n = winSize(w, h);
    if (n.w > sent.w || n.h > sent.h) {
      // Grow to the union first so the morph never clips; shrink once it has ended.
      sent = { w: Math.max(n.w, sent.w), h: Math.max(n.h, sent.h) };
      api.overlayResize(sent.w, sent.h);
    }
  }
  function settleWindow() {
    clearTimeout(settleTimer);
    settleTimer = window.setTimeout(() => {
      if (morphAnim) return;
      const n = winSize(cur.w, cur.h);
      if (n.w !== sent.w || n.h !== sent.h) {
        sent = n;
        api.overlayResize(n.w, n.h);
      }
    }, 60);
  }

  function morph(w: number, h: number, r: number) {
    const el = shellEl;
    if (!el || !w || !h) return;
    if (w === cur.w && h === cur.h && r === cur.r) return;
    const instant = snapNext || reduce || !cur.w;
    const from = { w: el.offsetWidth, h: el.offsetHeight, r: parseFloat(getComputedStyle(el).borderTopLeftRadius) || r };
    morphAnim?.cancel();
    morphAnim = null;
    cur = { w, h, r };
    el.style.width = `${w}px`;
    el.style.height = `${h}px`;
    el.style.borderRadius = `${r}px`;
    growWindow(w, h);
    if (instant) {
      settleWindow();
      return;
    }
    const a = el.animate(
      [
        { width: `${from.w}px`, height: `${from.h}px`, borderRadius: `${from.r}px` },
        { width: `${w}px`, height: `${h}px`, borderRadius: `${r}px` },
      ],
      { duration: MORPH_MS, easing: EASE },
    );
    morphAnim = a;
    a.onfinish = () => {
      if (morphAnim === a) morphAnim = null;
      settleWindow();
    };
  }

  $effect(() => {
    if (!visible || !shellEl) return;
    if (measured.key !== layerKey) return; // wait until the new layer has been laid out
    const w = Math.round(targetW);
    const h = Math.round(measured.h) + 2; // + the 1px border on each side
    const r = targetR;
    untrack(() => morph(w, h, r));
  });
  // Snap stays armed only for the first layout after appearing.
  $effect(() => {
    if (!visible) return;
    const id = requestAnimationFrame(() => requestAnimationFrame(() => (snapNext = false)));
    return () => cancelAnimationFrame(id);
  });

  function measure(node: HTMLElement, key: string) {
    const read = () => {
      if (key !== layerKey) return; // an outgoing layer never drives the size
      const w = node.offsetWidth;
      const h = node.offsetHeight;
      if (measured.key !== key || measured.w !== w || measured.h !== h) measured = { key, w, h };
    };
    const ro = new ResizeObserver(read);
    ro.observe(node);
    read();
    return { destroy: () => ro.disconnect() };
  }

  // ------------------------------------------------------------ layer transitions (transform/opacity only)
  const dir = $derived(position === "top" ? -1 : 1);
  function layerIn(_: Element) {
    const d = dir;
    return {
      delay: 90,
      duration: 300,
      easing: (x: number) => 1 - Math.pow(1 - x, 3),
      css: (k: number) => `opacity:${k};transform:translateY(${(1 - k) * 8 * d}px) scale(${0.985 + 0.015 * k})`,
    };
  }
  function layerOut(_: Element) {
    // Appearing from hidden: whatever was there last time must not ghost-fade.
    if (snapNext) return { duration: 0 };
    return {
      duration: 130,
      css: (k: number) => `opacity:${k};transform:scale(${0.985 + 0.015 * k})`,
    };
  }
  function rise(_: Element, { delay = 0, y = 6, duration = 260 } = {}) {
    return {
      delay,
      duration,
      easing: (x: number) => 1 - Math.pow(1 - x, 3),
      css: (k: number) => `opacity:${k};transform:translateY(${(1 - k) * y}px)`,
    };
  }
  function fadeOut(_: Element, { duration = 120 } = {}) {
    return { duration, css: (k: number) => `opacity:${k}` };
  }
  /** Width-growing reveal for a button that appears inside a row (neighbors slide instead of jumping). */
  function grow(node: Element) {
    const w = (node as HTMLElement).offsetWidth;
    return {
      duration: 280,
      easing: (x: number) => 1 - Math.pow(1 - x, 3),
      css: (k: number) => `max-width:${w * k}px;opacity:${k};transform:scale(${0.9 + 0.1 * k});overflow:hidden`,
    };
  }

  // ------------------------------------------------------------ waveform
  const bars: HTMLElement[] = [];
  const scales = new Array<number>(BARS).fill(0.1);
  function bar(node: HTMLElement, i: number) {
    bars[i] = node;
  }
  let level = 0;
  let amp = 0;
  let peak = 0.3;
  let floor = -1;
  const HIST = 24;
  const hist = new Float32Array(HIST);
  let hpos = 0;

  function runListening() {
    let raf = 0;
    let last = performance.now();
    floor = -1;
    hist.fill(0);
    const loop = (now: number) => {
      const dt = Math.min(4, (now - last) / 16.67);
      last = now;
      const raw = level;
      level *= Math.pow(0.8, dt); // decays if events stop
      // --- auto gain: track a noise floor (falls fast, rises very slowly) and a decaying peak
      if (floor < 0) floor = raw * 0.7;
      floor += (raw - floor) * (raw < floor ? 1 - Math.pow(0.75, dt) : 1 - Math.pow(0.9994, dt));
      peak = raw > peak ? peak + (raw - peak) * 0.5 : peak * Math.pow(0.9982, dt);
      const span = Math.max(0.2, peak - floor);
      let n = Math.min(1, Math.max(0, (raw - floor - 0.015) / span));
      n = Math.pow(n, 0.65);
      // --- snappy attack, smooth release
      amp += (n - amp) * (n > amp ? 1 - Math.pow(0.3, dt) : 1 - Math.pow(0.88, dt));
      hpos = (hpos + 1) % HIST;
      hist[hpos] = amp;
      for (let i = 0; i < BARS; i++) {
        const el = bars[i];
        if (!el) continue;
        let s: number;
        if (reduce) {
          s = 0.1 + 0.9 * amp * ENV[i];
        } else {
          const v = hist[(hpos - LAG[i] + HIST) % HIST];
          const wob = 0.62 + 0.38 * Math.sin(now / (150 + i * 19) + i * 2.3) * Math.sin(now / (410 - i * 7) + i);
          s = 0.1 + 0.9 * Math.min(1, v * ENV[i] * (0.55 + wob * 0.65));
        }
        scales[i] = s;
        el.style.transform = `scaleY(${s.toFixed(3)})`;
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    const timer = setInterval(() => (elapsed = Math.floor((performance.now() - listenStart) / 1000)), 250);
    return () => {
      cancelAnimationFrame(raf);
      clearInterval(timer);
    };
  }

  /** Processing: the same bars settle low, then a slow sine travels through them (compositor-only). */
  function runProcessing() {
    const LO = 0.16;
    const anims: Animation[] = [];
    let alive = true;
    bars.forEach((el, i) => {
      if (!el) return;
      if (reduce) {
        el.style.transform = `scaleY(${(0.2 + 0.25 * ENV[i]).toFixed(3)})`;
        return;
      }
      const hi = 0.28 + 0.42 * ENV[i];
      const a1 = el.animate([{ transform: `scaleY(${scales[i]})` }, { transform: `scaleY(${LO})` }], {
        duration: 320,
        easing: EASE,
        fill: "forwards",
      });
      anims.push(a1);
      a1.onfinish = () => {
        if (!alive) return;
        anims.push(
          el.animate([{ transform: `scaleY(${LO})` }, { transform: `scaleY(${hi.toFixed(3)})` }, { transform: `scaleY(${LO})` }], {
            duration: 1500,
            delay: i * 75,
            iterations: Infinity,
            easing: "cubic-bezier(0.45, 0, 0.55, 1)",
            fill: "backwards",
          }),
        );
      };
    });
    return () => {
      alive = false;
      bars.forEach((el, i) => {
        if (!el) return;
        el.style.transform = `scaleY(${LO})`;
        scales[i] = LO;
      });
      anims.forEach((a) => a.cancel());
    };
  }

  $effect(() => {
    const m = waveMode;
    if (m === "listen") return untrack(runListening);
    if (m === "proc") return untrack(runProcessing);
  });

  // ------------------------------------------------------------ live transcript
  const words = $derived(partial.trim() ? partial.trim().split(/\s+/) : []);
  const liveWords = $derived.by(() => {
    const start = Math.max(0, words.length - 40);
    return words.slice(start).map((w, k) => ({ w, i: start + k }));
  });
  function lineFlip(node: HTMLElement) {
    // When the paragraph gains a line, slide the old lines up instead of jumping.
    let h = node.offsetHeight;
    const ro = new ResizeObserver(() => {
      const nh = node.offsetHeight;
      const d = nh - h;
      h = nh;
      if (d > 0 && !reduce) {
        node.animate([{ transform: `translateY(${d}px)` }, { transform: "none" }], { duration: 260, easing: EASE });
      }
    });
    ro.observe(node);
    return { destroy: () => ro.disconnect() };
  }

  // ------------------------------------------------------------ result text (animated swap)
  const targetText = $derived(result ? (showOriginal ? result.raw : result.text) : "");
  let shownText = $state("");
  let shownFor = "";
  let textEl = $state<HTMLElement>();
  let swapSeq = 0;

  function replay(el: HTMLElement, cls: string) {
    el.classList.remove(cls);
    void el.offsetWidth;
    el.classList.add(cls);
  }
  function revealNow(node: HTMLElement) {
    if (!reduce) node.classList.add("reveal");
  }
  async function swapText(txt: string) {
    const seq = ++swapSeq;
    const el = textEl;
    if (!el || reduce) {
      shownText = txt;
      return;
    }
    el.classList.remove("reveal");
    const out = el.animate([{ opacity: 1 }, { opacity: 0, transform: "translateY(-3px)" }], {
      duration: 120,
      easing: "ease-in",
      fill: "forwards",
    });
    await out.finished.catch(() => {});
    if (seq !== swapSeq) return;
    shownText = txt;
    await tick();
    el.scrollTop = 0;
    replay(el, "reveal");
    out.cancel();
  }
  $effect(() => {
    const txt = targetText;
    const id = result?.id ?? "";
    untrack(() => {
      if (id !== shownFor) {
        shownFor = id;
        shownText = txt;
        return;
      }
      if (txt !== shownText) swapText(txt);
    });
  });

  // ------------------------------------------------------------ actions
  const fmt = (s: number) => `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  const styleName = (id: string | undefined) => settings?.styles.find((s) => s.id === id)?.name ?? id ?? "";
  const busy = $derived(!!pending);
  /** Paste rules: never pasted → Colar; pasted and unchanged → nothing; pasted and changed → Substituir. */
  const pasteAction = $derived(pastedText === null ? "paste" : targetText !== pastedText ? "replace" : null);
  const headTitle = $derived(pastedText === null ? t("ov.copied") : targetText === pastedText ? t("ov.pasted") : t("ovx.pasted_prev"));

  function dismiss() {
    if (hovered) api.overlayHold(false);
    hovered = false;
    ev = { phase: "hidden" };
    snapNext = true;
    api.dismissOverlay();
  }

  async function reprocess(styleId: string, target: string | null) {
    if (!result || pending) return;
    const p = { key: target ? `lang:${target}` : styleId, id: result.id, seq: ++pseq };
    pending = p;
    actionError = "";
    try {
      const r = await api.reprocess(styleId, target);
      // The `done` event may already have applied it.
      if (pending?.seq === p.seq && r) applyUpdate(r);
    } catch (e) {
      if (pending?.seq === p.seq) {
        pending = null;
        actionError = String(e);
        cycle++;
      }
    }
  }
  async function paste(replace: boolean) {
    const text = targetText;
    try {
      await api.pasteText(text, replace);
      pastedText = text;
    } catch (e) {
      actionError = String(e);
    }
    cycle++;
  }
  let copiedTimer = 0;
  async function copy(text: string) {
    await api.copyText(text);
    copied = true;
    clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => (copied = false), 1400);
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
    bind:this={shellEl}
    class="shell v-{view} w-{settings?.overlay_width ?? 'normal'}"
    class:hidden={!visible}
    role="region"
    aria-label="FlowVoice"
    aria-live="polite"
    onmouseenter={() => hold(true)}
    onmouseleave={() => hold(false)}
  >
    <!-- Persistent waveform: listening bars → processing wave, one set of elements. -->
    <div
      class="wave m-{waveMode}"
      class:snap={snapNext}
      aria-hidden="true"
      style="width:{WAVE_W}px;transform:translateX({-targetW / 2 + PAD_L}px)"
    >
      {#each ENV as _, i}
        <i use:bar={i} style="width:{BAR_W}px"></i>
      {/each}
    </div>

    {#key layerKey}
      <div
        class="layer"
        class:fit
        style:width={fit ? null : `${targetW}px`}
        use:measure={layerKey}
        in:layerIn
        out:layerOut
      >
        {#if view === "listening"}
          <div class="listen">
            <div class="row" style="padding-left:{PAD_L}px">
              <span class="wslot" style="width:{WAVE_W}px"></span>
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
            {#if hasText}
              <div class="live" in:rise={{ delay: 120, y: 10 * dir }} out:fadeOut>
                <div class="lwin">
                  <p use:lineFlip>
                    {#each liveWords as w (w.i)}<span class:old={words.length - w.i > 9}>{w.w}</span>{" "}{/each}
                  </p>
                </div>
              </div>
            {/if}
          </div>
        {:else if fit}
          <div class="row proc" style="padding-left:{PAD_L}px">
            <span class="wslot" style="width:{WAVE_W}px"></span>
            <span class="plabel">
              {#key procLabel}
                <span class="pl" in:rise={{ delay: 60, y: 6 * dir }} out:fadeOut={{ duration: 100 }}>{procLabel}</span>
              {/key}
            </span>
          </div>
        {:else if view === "done" && result}
          <div class="card">
            <header>
              <span class="badge ok"><Icon name="check" size={14} stroke={2.2} /></span>
              <span class="hstack ht">
                {#key headTitle}<span in:rise={{ y: 5 }} out:fadeOut={{ duration: 90 }}>{headTitle}</span>{/key}
              </span>
              <span class="hm">
                {#if busy}
                  <span class="rw">{t("ovx.rewriting")}</span>
                {:else if showOriginal}
                  {t("ov.showing_original")}
                {:else if result.target}
                  {t("ov.translate_to", { lang: langName(result.target) })}
                {:else}
                  {styleName(result.style_id)}
                {/if}
                {#if result.provider && !showOriginal && !busy}<span class="via">· {result.provider}</span>{/if}
              </span>
              <button type="button" class="x" aria-label={t("ov.dismiss")} title={t("ov.dismiss")} onclick={dismiss}>
                <Icon name="x" size={16} />
              </button>
            </header>

            <div class="tbox" class:busy>
              <div class="text" use:revealNow class:orig={showOriginal} bind:this={textEl} onanimationend={(e) => e.currentTarget.classList.remove("reveal")}>
                {shownText}
              </div>
            </div>

            {#if actionError}
              <p class="warnline err" in:rise={{ y: 4 }}>{actionError}</p>
            {:else if result.warnings.length}
              <p class="warnline">{result.warnings.join(" · ")}</p>
            {/if}

            <div class="chips">
              {#key picker}
                <div
                  class="chipset"
                  role="toolbar"
                  aria-label={picker ? t("ov.translate_pick") : t("nav.styles")}
                  in:rise={{ delay: 60, y: 4 }}
                  out:fadeOut={{ duration: 100 }}
                >
                  {#if picker}
                    <button type="button" class="chip back" aria-label={t("ovx.back_tip")} title={t("ovx.back_tip")} onclick={() => (picker = false)}>
                      <Icon name="chevronLeft" size={14} />
                    </button>
                    {#each TARGET_LANGS.slice(0, 10) as l, i}
                      <button
                        type="button"
                        class="chip"
                        style="--i:{i}"
                        class:on={result.target === l && !showOriginal}
                        class:loading={pending?.key === `lang:${l}`}
                        disabled={busy}
                        title={t("ovx.lang_tip", { lang: langName(l) })}
                        onclick={() => reprocess(result!.style_id, l)}
                      >
                        <span class="code">{l.toUpperCase()}</span>{langName(l)}
                      </button>
                    {/each}
                  {:else}
                    {#each settings?.styles ?? [] as st, i (st.id)}
                      <button
                        type="button"
                        class="chip"
                        style="--i:{i}"
                        class:on={!showOriginal && !result.target && result.style_id === st.id}
                        class:loading={pending?.key === st.id}
                        disabled={busy}
                        title={st.description || t("ovx.style_tip", { name: st.name })}
                        onclick={() => reprocess(st.id, null)}
                      >
                        {st.name}
                      </button>
                    {/each}
                    <button
                      type="button"
                      class="chip"
                      class:on={!!result.target && !showOriginal}
                      disabled={busy}
                      title={t("ovx.translate_tip")}
                      onclick={() => (picker = true)}
                    >
                      <Icon name="translate" size={14} />{t("ov.translate")}{result.target ? ` · ${result.target.toUpperCase()}` : ""}
                    </button>
                    <button
                      type="button"
                      class="chip"
                      class:on={showOriginal}
                      aria-pressed={showOriginal}
                      disabled={busy}
                      title={showOriginal ? t("ovx.refined_tip") : t("ovx.original_tip")}
                      onclick={() => (showOriginal = !showOriginal)}
                    >
                      {t("ov.original")}
                    </button>
                  {/if}
                </div>
              {/key}
            </div>

            <div class="acts">
              <span class="esc mono" title={t("ovx.esc_tip")}>esc</span>
              <button type="button" class="btn" disabled={busy} title={t("ovx.copy_tip")} onclick={() => copy(targetText)}>
                <span class="ico">
                  {#key copied}<span class="ic" in:rise={{ y: 4, duration: 220 }}><Icon name={copied ? "check" : "copy"} size={15} /></span>{/key}
                </span>
                {copied ? t("c.copied") : t("ov.copy")}
              </button>
              {#if pasteAction === "paste"}
                <button type="button" class="btn primary" disabled={busy} title={t("ovx.paste_tip")} in:grow onclick={() => paste(false)}>
                  <Icon name="paste" size={15} />{t("ov.paste")}
                </button>
              {:else if pasteAction === "replace"}
                <button type="button" class="btn primary" disabled={busy} title={t("ovx.replace_tip")} in:grow onclick={() => paste(true)}>
                  <Icon name="replace" size={15} />{t("ov.replace")}
                </button>
              {/if}
            </div>
            {@render life(6000)}
          </div>
        {:else if view === "error" && lastError}
          <div class="card err">
            <header>
              <span class="badge bad"><Icon name="alert" size={14} stroke={1.8} /></span>
              <span class="ht">{t("ov.error")}</span>
              <span class="hm"></span>
              <button type="button" class="x" aria-label={t("ov.dismiss")} title={t("ov.dismiss")} onclick={dismiss}>
                <Icon name="x" size={16} />
              </button>
            </header>
            <p class="emsg">{lastError.message}</p>
            {#if lastError.detail}
              <button type="button" class="dtoggle" aria-expanded={detailOpen} onclick={() => (detailOpen = !detailOpen)}>
                <span class="chev" class:open={detailOpen}><Icon name="chevronRight" size={14} /></span>
                {detailOpen ? t("ov.hide_details") : t("ov.details")}
              </button>
              {#if detailOpen}
                <pre class="detail" in:rise={{ y: -4, duration: 220 }}>{lastError.detail}</pre>
              {/if}
            {/if}
            <div class="acts">
              <button
                type="button"
                class="btn"
                title={t("ovx.settings_tip")}
                onclick={() => {
                  api.openSettings("providers");
                  dismiss();
                }}
              >
                <Icon name="key" size={15} />{t("ov.open_settings")}
              </button>
              {#if lastError.raw}
                <button type="button" class="btn primary" title={t("ovx.copy_raw_tip")} onclick={() => copy(lastError!.raw!)}>
                  <Icon name={copied ? "check" : "copy"} size={15} />{copied ? t("c.copied") : t("ov.copy_raw")}
                </button>
              {/if}
            </div>
            {@render life(12000)}
          </div>
        {/if}
      </div>
    {/key}
  </div>
</div>

{#snippet life(ms: number)}
  {#key cycle}
    <div
      class="life"
      class:paused={hovered || busy || picker || detailOpen}
      style="animation-duration:{ms}ms"
      onanimationend={() => visible && dismiss()}
    ></div>
  {/key}
{/snippet}

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
    --morph: cubic-bezier(0.22, 1, 0.36, 1);
  }
  .stage.top {
    align-items: flex-start;
  }

  /* ---------------------------------------------------------------- shell */
  .shell {
    position: relative;
    flex: none;
    width: 388px;
    height: 56px;
    border-radius: 28px;
    background: var(--cap-bg);
    color: var(--cap-text);
    border: 1px solid var(--cap-line);
    box-shadow:
      0 8px 20px -6px rgba(0, 0, 0, 0.45),
      0 2px 6px rgba(0, 0, 0, 0.18),
      inset 0 1px 0 rgba(255, 255, 255, 0.05);
    overflow: hidden;
    pointer-events: auto;
    contain: layout paint style;
    transform-origin: 50% 100%;
    transition:
      background-color 260ms var(--ease),
      border-color 260ms var(--ease),
      opacity 220ms var(--ease),
      transform 420ms var(--spring-soft, var(--spring));
    font-size: 13px;
  }
  .top .shell {
    transform-origin: 50% 0;
  }
  .shell.hidden {
    opacity: 0;
    transform: translateY(10px) scale(0.94);
    pointer-events: none;
    transition:
      opacity 170ms var(--ease-in, ease-in),
      transform 200ms var(--ease-in, ease-in);
  }
  .top .shell.hidden {
    transform: translateY(-10px) scale(0.94);
  }
  .shell.v-error {
    border-color: rgba(255, 90, 78, 0.42);
    background: color-mix(in srgb, var(--cap-bg) 92%, #ff5a4e);
  }

  /* ---------------------------------------------------------------- layers */
  .layer {
    position: absolute;
    left: 50%;
    translate: -50% 0;
    bottom: 0;
  }
  .layer.fit {
    width: max-content;
  }
  .top .layer {
    top: 0;
    bottom: auto;
  }

  /* ---------------------------------------------------------------- waveform (persistent) */
  .wave {
    position: absolute;
    left: 50%;
    bottom: 0;
    height: 54px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    pointer-events: none;
    z-index: 1;
    transition:
      transform 460ms var(--morph),
      opacity 200ms var(--ease);
  }
  .top .wave {
    top: 0;
    bottom: auto;
  }
  .wave.snap {
    transition: none;
  }
  .wave.m-off {
    opacity: 0;
    transition:
      transform 460ms var(--morph),
      opacity 140ms var(--ease);
  }
  .wave i {
    display: block;
    height: 30px;
    border-radius: 2px;
    background: var(--cap-wave);
    transform: scaleY(0.1);
    will-change: transform;
    transition: background-color 360ms var(--ease);
  }
  .wave.m-proc i {
    background: var(--cap-accent-line);
  }
  .wslot {
    flex: none;
    height: 30px;
  }

  /* ---------------------------------------------------------------- listening */
  .listen {
    display: flex;
    flex-direction: column-reverse;
  }
  .top .listen {
    flex-direction: column;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 54px;
    padding-right: 9px;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.25;
    padding-left: 2px;
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
    margin-right: 2px;
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
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease),
      transform 260ms var(--spring);
  }
  .stop {
    background: var(--cap-hover);
    color: var(--cap-text);
  }
  .stop:hover {
    background: var(--cap-accent);
    color: var(--on-accent, #0b0b0c);
    transform: scale(1.06);
  }
  .x:hover {
    background: var(--cap-hover);
    color: var(--cap-text);
  }
  .x:hover :global(svg) {
    transform: rotate(90deg);
  }
  .x :global(svg) {
    transition: transform 280ms var(--spring);
  }
  .stop:active,
  .x:active {
    transform: scale(0.9);
    transition-duration: 60ms;
  }
  .x {
    width: 30px;
    height: 30px;
  }

  .live {
    padding: 14px 22px 0;
  }
  .top .live {
    padding: 0 22px 14px;
  }
  .lwin {
    height: 44px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, rgba(0, 0, 0, 0.35) 8px, #000 24px);
    mask-image: linear-gradient(to bottom, transparent 0, rgba(0, 0, 0, 0.35) 8px, #000 24px);
  }
  .live p {
    flex: none;
    margin: 0;
    font-size: 14.5px;
    line-height: 22px;
    letter-spacing: -0.003em;
  }
  .w-compact .live p {
    font-size: 13.5px;
  }
  .live span {
    display: inline-block;
    animation: word 340ms var(--morph) both;
    transition: color 700ms var(--ease);
  }
  .live span.old {
    color: var(--cap-text-2);
  }
  @keyframes word {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }

  /* ---------------------------------------------------------------- processing */
  .proc {
    padding-right: 24px;
  }
  .plabel {
    display: grid;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
  }
  .pl {
    grid-area: 1 / 1;
  }

  /* ---------------------------------------------------------------- result / error card */
  .card {
    position: relative;
    padding: 12px 14px 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    --fs: 14px;
    --lh: 21px;
  }
  .w-compact .card {
    --fs: 13px;
    --lh: 19px;
    padding: 11px 12px 12px 14px;
    gap: 9px;
  }
  .w-wide .card {
    --fs: 14.5px;
    --lh: 22px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    margin-right: -6px;
  }
  .badge {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    flex: none;
    animation: badge 520ms var(--spring) 80ms both;
  }
  .ok {
    background: var(--cap-accent);
    color: var(--on-accent, #0b0b0c);
  }
  .ok :global(path) {
    stroke-dasharray: 22;
    stroke-dashoffset: 22;
    animation: draw 380ms var(--morph) 260ms forwards;
  }
  .bad {
    background: #ff5a4e;
    color: #fff;
    animation-name: badge, nudge;
    animation-duration: 520ms, 420ms;
    animation-delay: 80ms, 520ms;
  }
  @keyframes badge {
    from {
      transform: scale(0.3) rotate(-30deg);
      opacity: 0;
    }
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes nudge {
    25% {
      transform: translateX(-2px);
    }
    50% {
      transform: translateX(2px);
    }
    75% {
      transform: translateX(-1px);
    }
  }
  .ht {
    font-weight: 600;
    font-size: 13px;
    white-space: nowrap;
  }
  .hstack {
    display: grid;
  }
  .hstack > :global(span) {
    grid-area: 1 / 1;
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
  .rw {
    color: var(--cap-text);
  }
  .via {
    font-family: var(--mono);
    font-size: 10.5px;
  }

  .tbox {
    position: relative;
    overflow: hidden;
    margin-right: -4px;
  }
  .text {
    font-size: var(--fs);
    line-height: var(--lh);
    max-height: calc(var(--lh) * 6);
    overflow-y: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
    padding-right: 6px;
    transition: opacity 240ms var(--ease);
    scrollbar-width: thin;
  }
  .w-compact .text {
    max-height: calc(var(--lh) * 5);
  }
  .text.reveal {
    -webkit-mask-image: linear-gradient(100deg, #000 42%, transparent 58%);
    mask-image: linear-gradient(100deg, #000 42%, transparent 58%);
    -webkit-mask-size: 280% 100%;
    mask-size: 280% 100%;
    animation: reveal 620ms var(--morph) both;
  }
  @keyframes reveal {
    from {
      -webkit-mask-position: 100% 0;
      mask-position: 100% 0;
      transform: translateY(3px);
    }
    to {
      -webkit-mask-position: 0 0;
      mask-position: 0 0;
      transform: none;
    }
  }
  .tbox.busy .text {
    opacity: 0.45;
  }
  .tbox::after {
    content: "";
    position: absolute;
    inset: -2px 0;
    pointer-events: none;
    opacity: 0;
    background: linear-gradient(100deg, transparent 30%, var(--cap-accent-soft) 50%, transparent 70%);
    transform: translateX(-100%);
    transition: opacity 200ms var(--ease);
  }
  .tbox.busy::after {
    opacity: 1;
    animation: shimmer 1300ms cubic-bezier(0.45, 0, 0.55, 1) infinite;
  }
  @keyframes shimmer {
    to {
      transform: translateX(100%);
    }
  }
  .text.orig {
    color: var(--cap-text-2);
    font-style: italic;
  }
  .warnline {
    margin: -4px 0 0;
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
  :global([data-theme="light"]) .warnline.err {
    color: #c8372d;
  }

  /* chips: wrap, max two rows, never horizontal scroll */
  .chips {
    display: grid;
  }
  .chipset {
    grid-area: 1 / 1;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-height: calc(28px * 2 + 6px);
    overflow-y: auto;
    scrollbar-width: none;
    padding: 1px;
    margin: -1px;
  }
  .chipset::-webkit-scrollbar {
    display: none;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 12px;
    border-radius: 14px;
    border: 1px solid var(--cap-line);
    font-size: 12px;
    color: var(--cap-text-2);
    white-space: nowrap;
    flex: none;
    position: relative;
    overflow: hidden;
    animation: chipin 360ms var(--morph) both;
    animation-delay: calc(140ms + var(--i, 8) * 22ms);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease),
      border-color var(--t) var(--ease),
      transform 240ms var(--spring);
  }
  @keyframes chipin {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.96);
    }
  }
  .w-compact .chipset {
    gap: 5px;
    max-height: calc(25px * 2 + 5px);
  }
  .w-compact .chip {
    height: 25px;
    padding: 0 10px;
    font-size: 11.5px;
  }
  .chip:hover:not(:disabled) {
    color: var(--cap-text);
    background: var(--cap-hover);
    transform: translateY(-1px);
  }
  .chip:active:not(:disabled) {
    transform: scale(0.95);
    transition-duration: 60ms;
  }
  .chip.on {
    color: var(--cap-text);
    border-color: var(--cap-accent-line);
    background: var(--cap-accent-soft);
  }
  .chip:disabled:not(.loading) {
    opacity: 0.45;
  }
  .chip.back {
    padding: 0 8px;
  }
  .code {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--cap-text-2);
  }
  .chip.loading {
    color: var(--cap-text);
    border-color: var(--cap-accent-line);
  }
  .chip.loading::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--cap-accent-line), transparent);
    animation: chipbar 900ms cubic-bezier(0.45, 0, 0.55, 1) infinite;
  }
  @keyframes chipbar {
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
    cursor: default;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: 9px;
    font-size: 12.5px;
    font-weight: 500;
    border: 1px solid var(--cap-line);
    color: var(--cap-text);
    white-space: nowrap;
    transition:
      background-color var(--t-fast) var(--ease),
      border-color var(--t-fast) var(--ease),
      transform 240ms var(--spring);
  }
  .btn:hover:not(:disabled) {
    background: var(--cap-hover);
    transform: translateY(-1px);
  }
  .btn:active:not(:disabled) {
    transform: scale(0.96);
    transition-duration: 60ms;
  }
  .btn.primary {
    background: var(--cap-accent);
    border-color: var(--cap-accent);
    color: var(--on-accent, #0b0b0c);
    font-weight: 600;
  }
  .btn.primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--cap-accent) 82%, #fff);
  }
  .btn:disabled {
    opacity: 0.4;
  }
  .ico {
    display: grid;
    width: 15px;
    height: 15px;
  }
  .ic {
    grid-area: 1 / 1;
  }

  .emsg {
    margin: 0;
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
    transition: color var(--t-fast) var(--ease);
  }
  .dtoggle:hover {
    color: var(--cap-text);
  }
  .chev {
    display: grid;
    transition: transform 260ms var(--spring);
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .detail {
    margin: 0;
    max-height: 96px;
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
  :global([data-theme="light"]) .detail {
    background: rgba(0, 0, 0, 0.05);
  }

  /* countdown */
  .life {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: 0;
    height: 2px;
    border-radius: 2px 2px 0 0;
    background: var(--cap-accent-line);
    opacity: 0.75;
    transform-origin: 50% 50%;
    animation-name: life;
    animation-timing-function: linear;
    animation-fill-mode: both;
    transition: opacity 240ms var(--ease);
  }
  .err .life {
    background: #ff5a4e;
  }
  .life.paused {
    animation-play-state: paused;
    opacity: 0.25;
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
    .shell,
    .wave {
      transition-duration: 1ms;
    }
    .badge,
    .ok :global(path),
    .chip,
    .live span,
    .text.reveal {
      animation: none !important;
      stroke-dashoffset: 0;
    }
    .text.reveal {
      -webkit-mask-image: none;
      mask-image: none;
    }
    .tbox.busy::after {
      animation: none;
      transform: none;
    }
    .life {
      animation-duration: 8000ms !important;
      animation-iteration-count: 1 !important;
    }
  }
</style>
