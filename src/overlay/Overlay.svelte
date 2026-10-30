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
  /** Rust: { phase: "out" | "in" } around a follow-mouse monitor switch (not in EVENTS yet). */
  const OVERLAY_MOVE = "overlay-move";
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
  /** Live transcript as keyed chunks (stable prefix + newer pieces), see setLive(). */
  type Chunk = { id: number; text: string; kind: "add" | "fix" };
  let chunks = $state<Chunk[]>([]);
  let liveFull = "";
  let chunkSeq = 0;
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
  const hasText = $derived(live && chunks.length > 0);
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
    hideTip();
    if (e.phase === "hidden") {
      snapNext = true;
    } else {
      if (ev.phase === "hidden") {
        snapNext = true;
        // The native window may have been resized/moved while hidden: re-send the size.
        cur = { w: 0, h: 0, r: 0 };
        sent = { w: 0, h: 0 };
      }
      view = e.phase === "transcribing" || e.phase === "refining" ? "processing" : e.phase;
    }
    if (e.phase === "listening") {
      liveFull = "";
      chunks = [];
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
      expanded = false;
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
        if (ev.phase === "listening") setLive(p?.text ?? "");
      }),
      // Follow-mouse monitor switch: Rust emits "out", moves the window ~140ms later, then "in".
      on<{ phase: "out" | "in" }>(OVERLAY_MOVE, (p) => moveCard(p?.phase)),
      on<Settings>(EVENTS.settingsChanged, (s) => {
        settings = s;
        applyTheme(s.theme);
        applyAccent(s.accent_color);
        setLanguage(s.ui_language);
      }),
    ];
    window.addEventListener("resize", healWindow);
    return () => {
      window.removeEventListener("resize", healWindow);
      subs.forEach((p) => p.then((f) => f()));
    };
  });

  // ------------------------------------------------------------ shell morph + native window size
  let shellEl = $state<HTMLDivElement>();
  let morphAnim: Animation | null = null;
  let cur = { w: 0, h: 0, r: 0 };
  let sent = { w: 0, h: 0 };
  let snapNext = $state(true);
  let settleTimer = 0;
  let healCount = 0;

  /** Native size for a card size: integers, even width (Rust centers it → no half-pixel shift). */
  const winSize = (w: number, h: number) => {
    let nw = Math.min(MAX_W, Math.ceil(w + M * 2));
    if (nw % 2) nw = Math.min(MAX_W, nw + 1);
    return { w: nw, h: Math.min(MAX_H, Math.ceil(h + M * 2)) };
  };
  // Resize requests are coalesced per task: several layout passes in one frame → one IPC call.
  let want: { w: number; h: number } | null = null;
  function requestWin(n: { w: number; h: number }) {
    const first = !want;
    want = n;
    if (!first) return;
    queueMicrotask(() => {
      const n2 = want!;
      want = null;
      if (n2.w === sent.w && n2.h === sent.h) return;
      sent = n2;
      healCount = 0;
      api.overlayResize(n2.w, n2.h);
    });
  }
  function growWindow(w: number, h: number) {
    const n = winSize(w, h);
    const base = want ?? sent;
    // Grow to the union first so the morph never clips; shrink only once it has ended.
    if (n.w > base.w || n.h > base.h) requestWin({ w: Math.max(n.w, base.w), h: Math.max(n.h, base.h) });
  }
  function settleWindow(delay = 90) {
    clearTimeout(settleTimer);
    settleTimer = window.setTimeout(() => {
      if (morphAnim || !visible) return;
      requestWin(winSize(cur.w, cur.h));
    }, delay);
  }
  /** Self-heal: if the native window ends up smaller than the card (lost/clamped request), re-send. */
  function healWindow() {
    if (!visible || morphAnim || !cur.w || healCount >= 2) return;
    const n = winSize(cur.w, cur.h);
    if (innerWidth + 1 < n.w || innerHeight + 1 < n.h) {
      healCount++;
      sent = { w: 0, h: 0 };
      requestWin(n);
    }
  }

  function morph(w: number, h: number, r: number) {
    const el = shellEl;
    if (!el || !w || !h) return;
    if (w === cur.w && h === cur.h && r === cur.r) return;
    const instant = snapNext || reduce || !cur.w;
    // Read the in-flight size before touching anything (one style read, no forced reflow loop).
    const from = morphAnim
      ? { w: el.offsetWidth, h: el.offsetHeight, r: parseFloat(getComputedStyle(el).borderTopLeftRadius) || r }
      : { ...cur };
    morphAnim?.cancel();
    morphAnim = null;
    cur = { w, h, r };
    el.style.width = `${w}px`;
    el.style.height = `${h}px`;
    el.style.borderRadius = `${r}px`;
    growWindow(Math.max(w, from.w), Math.max(h, from.h));
    if (instant) {
      settleWindow(0);
      return;
    }
    clearTimeout(settleTimer);
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

  // In-card FLIP: when a block inside the card changes height (expand text, language picker,
  // error details), the shell height morphs and every [data-flip] block glides from its old
  // place (transform `translate`, composited) instead of jumping; a block that itself resized is
  // clipped from its old box to its new one while its content travels with its top edge.
  type Box = { t: number; b: number };
  function snapshot(layer: HTMLElement) {
    const m = new Map<HTMLElement, Box>();
    const H = layer.offsetHeight;
    const fromBottom = position !== "top"; // the anchored edge stays put on screen
    layer.querySelectorAll<HTMLElement>("[data-flip]").forEach((el) => {
      let t = 0;
      for (let p: HTMLElement | null = el; p && p !== layer; p = p.offsetParent as HTMLElement | null) t += p.offsetTop;
      const b = t + el.offsetHeight;
      m.set(el, fromBottom ? { t: t - H, b: b - H } : { t, b });
    });
    return m;
  }
  const flipAnims = new Map<HTMLElement, Animation>();
  function flip(prev: Map<HTMLElement, Box>, next: Map<HTMLElement, Box>) {
    const opt = { duration: MORPH_MS, easing: EASE };
    next.forEach((n, el) => {
      const o = prev.get(el);
      if (!o) return;
      const dT = o.t - n.t;
      const dB = o.b - n.b;
      if (Math.abs(dT) < 0.5 && Math.abs(dB) < 0.5) return;
      if (Math.abs(dT - dB) < 0.5) {
        el.animate([{ translate: `0 ${dT}px` }, { translate: "0 0" }], { ...opt, composite: "add" });
        return;
      }
      flipAnims.get(el)?.cancel();
      flipAnims.set(el, el.animate([{ clipPath: `inset(${dT}px -8px ${-dB}px -8px)` }, { clipPath: "inset(0px -8px 0px -8px)" }], opt));
      if (Math.abs(dT) >= 0.5) {
        for (const c of Array.from(el.children)) {
          (c as HTMLElement).animate([{ translate: `0 ${dT}px` }, { translate: "0 0" }], { ...opt, composite: "add" });
        }
      }
    });
  }

  function measure(node: HTMLElement, key: string) {
    let boxes = new Map<HTMLElement, Box>();
    // The first layout of a new layer is driven by the layer cross-fade, not by FLIP.
    let morphReady = false;
    const read = () => {
      if (key !== layerKey) return; // an outgoing layer never drives the size
      const w = node.offsetWidth;
      const h = node.offsetHeight;
      const next = snapshot(node);
      next.forEach((_, el) => ro.observe(el));
      if (!snapNext && !reduce && morphReady) flip(boxes, next);
      boxes = next;
      if (measured.key !== key || measured.w !== w || measured.h !== h) measured = { key, w, h };
    };
    const ro = new ResizeObserver(read);
    ro.observe(node);
    read();
    const id = requestAnimationFrame(() => (morphReady = true));
    return {
      destroy: () => {
        cancelAnimationFrame(id);
        ro.disconnect();
      },
    };
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
  /** Outgoing chip set: leaves the flow at once so the card sizes to the incoming set only
   *  (otherwise the card would grow to the union for 100ms and then shrink back). */
  function chipOut(node: Element) {
    const el = node as HTMLElement;
    el.style.position = "absolute";
    el.style.inset = "0 0 auto 0";
    el.style.pointerEvents = "none";
    return { duration: 100, css: (k: number) => `opacity:${k}` };
  }
  /** Svelte may revive an outroing chip set (quick toggle back): put it back into the flow. */
  function chipIn(node: Element) {
    const el = node as HTMLElement;
    el.style.position = el.style.inset = el.style.pointerEvents = "";
    return rise(node, { delay: 60, y: 4 });
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
  // Rust sends one line: finished segments + current tail, and finished segments are later
  // replaced in place by a better transcript. So the text is diffed by common prefix: the
  // unchanged prefix keeps its DOM (no re-animation, no reordering), an appended suffix fades
  // in, and a rewrite in the middle cross-fades softly from that word on. Flowing inline text,
  // never per-word boxes, so nothing can look shuffled.
  function normLive(s: string) {
    let x = s.replace(/\s+/g, " ").trim();
    x = x.replace(/\s+([,.;:!?…%)\]])/gu, "$1"); // no space before punctuation
    x = x.replace(/([(\[])\s+/gu, "$1");
    x = x.replace(/([,;])(?=\p{L})/gu, "$1 "); // "a,b" → "a, b"
    x = x.replace(/([.!?…])(?=\p{Lu})/gu, "$1 "); // "fim.Outra" → "fim. Outra"
    x = x.replace(/([.!?…]\s+)(\p{Ll})/gu, (_, a: string, b: string) => a + b.toUpperCase());
    return x.charAt(0).toUpperCase() + x.slice(1);
  }
  function setLive(raw: string) {
    const next = normLive(raw);
    const prev = liveFull;
    if (next === prev) return;
    liveFull = next;
    if (!next) {
      chunks = [];
      return;
    }
    let c = 0;
    const L = Math.min(prev.length, next.length);
    while (c < L && prev.charCodeAt(c) === next.charCodeAt(c)) c++;
    const appended = c === prev.length;
    if (!appended) while (c > 0 && next[c - 1] !== " ") c--; // rewrite from the start of that word
    const out: Chunk[] = [];
    let pos = 0;
    for (const ch of chunks) {
      if (pos >= c) break;
      const end = pos + ch.text.length;
      out.push(end <= c ? ch : { ...ch, text: ch.text.slice(0, c - pos) });
      pos = end;
    }
    const rest = next.slice(c);
    if (rest) out.push({ id: ++chunkSeq, text: rest, kind: appended ? "add" : "fix" });
    // Fold the oldest (long settled) chunks together; the survivor keeps its key → no re-render.
    while (out.length > 6) out.splice(0, 2, { ...out[0], text: out[0].text + out[1].text });
    chunks = out;
  }
  function lineFlip(node: HTMLElement) {
    // When the paragraph gains a line, glide the old lines up instead of jumping (coalesced:
    // composite "add" lets a second line arrive mid-glide without a restart).
    let h = node.offsetHeight;
    const ro = new ResizeObserver(() => {
      const nh = node.offsetHeight;
      const d = nh - h;
      h = nh;
      if (d > 0 && !reduce) {
        node.animate([{ translate: `0 ${d}px` }, { translate: "0 0" }], { duration: 300, easing: EASE, composite: "add" });
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

  // ------------------------------------------------------------ result text clamp ("ver tudo")
  // No scrollbars: the text is clamped to a few lines with a fade; "ver tudo" grows the card
  // (through the normal morph + window resize) up to what the 760×560 native window allows.
  let expanded = $state(false);
  let overflowing = $state(false);
  let expandLines = $state(0);
  const baseLines = $derived(picker ? 2 : settings?.overlay_width === "compact" ? 5 : 6);
  const textLines = $derived(expanded && !picker ? Math.max(baseLines, expandLines) : baseLines);

  function checkOverflow() {
    const el = textEl;
    overflowing = !!el && el.scrollHeight > el.clientHeight + 2;
  }
  $effect(() => {
    void shownText;
    void textLines;
    const el = textEl;
    if (!el) return;
    tick().then(checkOverflow);
    const ro = new ResizeObserver(checkOverflow);
    ro.observe(el);
    return () => ro.disconnect();
  });
  $effect(() => {
    void result?.id;
    void picker;
    expanded = false;
  });
  function toggleExpand() {
    hideTip();
    if (expanded) {
      expanded = false;
      textEl?.scrollTo({ top: 0 });
      return;
    }
    const el = textEl;
    if (!el) return;
    const lh = parseFloat(getComputedStyle(el).lineHeight) || 21;
    // Everything in the card except the text box stays; the text gets the rest of the max window.
    const chrome = measured.h + 2 - el.clientHeight;
    expandLines = Math.max(baseLines, Math.floor((MAX_H - M * 2 - chrome - 24) / lh));
    expanded = true;
  }
  /** Long text still over the max window: wheel scroll is allowed, but no scrollbar is drawn. */
  const wheelScroll = $derived(expanded && overflowing);

  // ------------------------------------------------------------ in-card tooltip (no native `title`)
  // Rendered inside the card, clamped to its bounds (never clipped, never resizes the window).
  let tip = $state<{ text: string; x: number; y: number; on: boolean }>({ text: "", x: 0, y: 0, on: false });
  let tipEl = $state<HTMLElement>();
  let tipTimer = 0;
  let tipFor: HTMLElement | null = null;
  function hideTip() {
    clearTimeout(tipTimer);
    tipFor = null;
    if (tip.on) tip = { ...tip, on: false };
  }
  function tipOver(e: Event) {
    const t = (e.target as HTMLElement | null)?.closest<HTMLElement>("[data-tip]");
    if (t === tipFor) return;
    hideTip();
    if (!t || !t.dataset.tip) return;
    tipFor = t;
    tipTimer = window.setTimeout(() => placeTip(t), 520);
  }
  async function placeTip(t: HTMLElement) {
    const card = t.closest<HTMLElement>(".card");
    if (!card || tipFor !== t) return;
    tip = { text: t.dataset.tip ?? "", x: 0, y: 0, on: false };
    await tick();
    if (!tipEl || tipFor !== t) return;
    const c = card.getBoundingClientRect();
    const r = t.getBoundingClientRect();
    const tw = tipEl.offsetWidth;
    const th = tipEl.offsetHeight;
    const x = Math.round(Math.min(Math.max(6, r.left - c.left + r.width / 2 - tw / 2), c.width - tw - 6));
    const above = r.top - c.top - th - 6;
    const y = Math.round(above >= 4 ? above : Math.min(r.bottom - c.top + 6, c.height - th - 4));
    tip = { text: tip.text, x, y, on: true };
  }

  // ------------------------------------------------------------ monitor switch (overlay-move)
  let moverEl = $state<HTMLDivElement>();
  let moveAnim: Animation | null = null;
  let moveSafety = 0;
  function moveCard(phase: "out" | "in" | undefined) {
    const el = moverEl;
    if (!el) return;
    clearTimeout(moveSafety);
    hideTip();
    const d = dir;
    const outFrame = { opacity: 0, transform: `translateY(${6 * d}px) scale(0.96)` };
    moveAnim?.cancel();
    if (phase === "out") {
      moveAnim = reduce
        ? el.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 80, fill: "forwards" })
        : el.animate([{ opacity: 1, transform: "none" }, outFrame], { duration: 120, easing: "cubic-bezier(0.4, 0, 1, 1)", fill: "forwards" });
      // Never stay invisible if the "in" event gets lost.
      moveSafety = window.setTimeout(() => moveCard("in"), 900);
    } else {
      moveAnim = reduce
        ? el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 120 })
        : el.animate([{ ...outFrame, transform: `translateY(${10 * d}px) scale(0.97)` }, { opacity: 1, transform: "none" }], {
            duration: 220,
            easing: "cubic-bezier(0.34, 1.36, 0.64, 1)",
          });
      const a = moveAnim;
      a.onfinish = () => moveAnim === a && (moveAnim = null);
    }
  }

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
  <div class="mover" bind:this={moverEl}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={shellEl}
      class="shell v-{view} w-{settings?.overlay_width ?? 'normal'}"
      class:hidden={!visible}
      role="region"
      aria-label="FlowVoice"
      aria-live="polite"
      onmouseenter={() => hold(true)}
      onmouseleave={() => {
        hold(false);
        hideTip();
      }}
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
                <button type="button" class="stop" aria-label={t("ov.stop")} onclick={() => api.stopRecording()}>
                  <Icon name="stop" size={16} />
                </button>
                <button type="button" class="x" aria-label={t("ov.cancel")} onclick={() => api.cancelRecording()}>
                  <Icon name="x" size={16} />
                </button>
              </div>
              {#if hasText}
                <div class="live" in:rise={{ delay: 120, y: 10 * dir }} out:fadeOut>
                  <div class="lwin">
                    <p use:lineFlip>{#each chunks as c, i (c.id)}<span class="ck {c.kind}" class:old={i < chunks.length - 4}>{c.text}</span>{/each}</p>
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
            <!-- svelte-ignore a11y_mouse_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="card" onmouseover={tipOver} onfocusin={tipOver} onfocusout={hideTip} onpointerdown={hideTip}>
              <header data-flip>
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
                <button type="button" class="x" aria-label={t("ov.dismiss")} onclick={dismiss}>
                  <Icon name="x" size={16} />
                </button>
              </header>

              <div class="tbox" class:busy data-flip>
                <div
                  class="text"
                  class:orig={showOriginal}
                  class:wheel={wheelScroll}
                  style="--lines:{textLines}"
                  use:revealNow
                  bind:this={textEl}
                  onanimationend={(e) => e.currentTarget.classList.remove("reveal")}
                >
                  {shownText}
                </div>
                {#if overflowing && !wheelScroll}<div class="fadeb" aria-hidden="true" out:fadeOut></div>{/if}
              </div>
              {#if (overflowing || expanded) && !picker}
                <div class="morebar" data-flip>
                  <button type="button" class="more" aria-expanded={expanded} onclick={toggleExpand}>
                    {expanded ? t("ovx.less") : t("ovx.more")}
                    <span class="mchev" class:up={expanded}><Icon name="chevronRight" size={12} /></span>
                  </button>
                </div>
              {/if}

              {#if actionError}
                <p class="warnline err" data-flip in:rise={{ y: 4 }}>{actionError}</p>
              {:else if result.warnings.length}
                <p class="warnline" data-flip>{result.warnings.join(" · ")}</p>
              {/if}

              <div class="chips" data-flip>
                {#key picker}
                  <div
                    class="chipset"
                    class:langs={picker}
                    role="toolbar"
                    aria-label={picker ? t("ov.translate_pick") : t("nav.styles")}
                    in:chipIn
                    out:chipOut
                  >
                    {#if picker}
                      <button type="button" class="chip back" style="--i:0" aria-label={t("ovx.back_tip")} onclick={() => (picker = false)}>
                        <Icon name="chevronLeft" size={14} />{t("ovx.back")}
                      </button>
                      {#each TARGET_LANGS as l, i}
                        <button
                          type="button"
                          class="chip lang"
                          style="--i:{Math.min(i + 1, 14)}"
                          class:on={result.target === l && !showOriginal}
                          class:loading={pending?.key === `lang:${l}`}
                          disabled={busy}
                          aria-label={t("ovx.lang_tip", { lang: langName(l) })}
                          onclick={() => reprocess(result!.style_id, l)}
                        >
                          <span class="code">{l.toUpperCase()}</span><span class="ln">{langName(l)}</span>
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
                          data-tip={st.description || null}
                          onclick={() => reprocess(st.id, null)}
                        >
                          {st.name}
                        </button>
                      {/each}
                      <button
                        type="button"
                        class="chip"
                        data-k="translate"
                        class:on={!!result.target && !showOriginal}
                        disabled={busy}
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
                        data-tip={showOriginal ? t("ovx.refined_tip") : t("ovx.original_tip")}
                        onclick={() => (showOriginal = !showOriginal)}
                      >
                        {t("ov.original")}
                      </button>
                    {/if}
                  </div>
                {/key}
              </div>

              <div class="acts" data-flip>
                <span class="esc mono" aria-label={t("ovx.esc_tip")}>esc</span>
                <button type="button" class="btn" disabled={busy} onclick={() => copy(targetText)}>
                  <span class="ico">
                    {#key copied}<span class="ic" in:rise={{ y: 4, duration: 220 }}><Icon name={copied ? "check" : "copy"} size={15} /></span>{/key}
                  </span>
                  {copied ? t("c.copied") : t("ov.copy")}
                </button>
                {#if pasteAction === "paste"}
                  <button type="button" class="btn primary" disabled={busy} data-tip={t("ovx.paste_tip")} in:grow onclick={() => paste(false)}>
                    <Icon name="paste" size={15} />{t("ov.paste")}
                  </button>
                {:else if pasteAction === "replace"}
                  <button type="button" class="btn primary" disabled={busy} data-tip={t("ovx.replace_tip")} in:grow onclick={() => paste(true)}>
                    <Icon name="replace" size={15} />{t("ov.replace")}
                  </button>
                {/if}
              </div>
              {@render tipBox()}
              {@render life(6000)}
            </div>
          {:else if view === "error" && lastError}
            <!-- svelte-ignore a11y_mouse_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="card err" onmouseover={tipOver} onfocusin={tipOver} onfocusout={hideTip} onpointerdown={hideTip}>
              <header data-flip>
                <span class="badge bad"><Icon name="alert" size={14} stroke={1.8} /></span>
                <span class="ht">{t("ov.error")}</span>
                <span class="hm"></span>
                <button type="button" class="x" aria-label={t("ov.dismiss")} onclick={dismiss}>
                  <Icon name="x" size={16} />
                </button>
              </header>
              <p class="emsg" data-flip>{lastError.message}</p>
              {#if lastError.detail}
                <button type="button" class="dtoggle" data-flip aria-expanded={detailOpen} onclick={() => (detailOpen = !detailOpen)}>
                  <span class="chev" class:open={detailOpen}><Icon name="chevronRight" size={14} /></span>
                  {detailOpen ? t("ov.hide_details") : t("ov.details")}
                </button>
                {#if detailOpen}
                  <pre class="detail" data-flip in:rise={{ y: -4, duration: 220 }}>{lastError.detail}</pre>
                {/if}
              {/if}
              <div class="acts" data-flip>
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
                {#if lastError.raw}
                  <button type="button" class="btn primary" data-tip={t("ovx.copy_raw_tip")} onclick={() => copy(lastError!.raw!)}>
                    <Icon name={copied ? "check" : "copy"} size={15} />{copied ? t("c.copied") : t("ov.copy_raw")}
                  </button>
                {/if}
              </div>
              {@render tipBox()}
              {@render life(12000)}
            </div>
          {/if}
        </div>
      {/key}
    </div>
  </div>
</div>

{#snippet tipBox()}
  <div class="tip" class:on={tip.on} bind:this={tipEl} role="tooltip" aria-hidden="true" style="transform:translate({tip.x}px, {tip.y}px)">
    {tip.text}
  </div>
{/snippet}

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
  .mover {
    flex: none;
    display: flex;
    transform-origin: 50% 100%;
  }
  .top .mover {
    transform-origin: 50% 0;
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
  /* flowing inline text: only opacity/color animate, so wrapping never changes mid-animation */
  .ck {
    transition: color 900ms var(--ease);
  }
  .ck.add {
    animation: ckadd 360ms var(--ease) both;
  }
  .ck.fix {
    animation: ckfix 560ms var(--ease) both;
  }
  .ck.old {
    color: var(--cap-text-2);
  }
  @keyframes ckadd {
    from {
      opacity: 0;
    }
  }
  @keyframes ckfix {
    from {
      opacity: 0.2;
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
    clip-path: inset(0);
    margin-right: -4px;
  }
  .text {
    font-size: var(--fs);
    line-height: var(--lh);
    max-height: calc(var(--lh) * var(--lines, 6));
    overflow: hidden;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
    padding-right: 6px;
    transition: opacity 240ms var(--ease);
  }
  /* still longer than the max window when expanded: wheel works, no scrollbar is ever drawn */
  .text.wheel {
    overflow-y: auto;
    scrollbar-width: none;
  }
  .text.wheel::-webkit-scrollbar {
    display: none;
  }
  .fadeb {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: calc(var(--lh) * 1.5);
    background: linear-gradient(to bottom, transparent, var(--cap-bg) 92%);
    pointer-events: none;
  }
  .morebar {
    display: flex;
    justify-content: flex-end;
    height: 18px;
    margin: -8px -4px -2px 0;
  }
  .more {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px 0 6px;
    border-radius: 6px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--cap-text-2);
    transition:
      color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease);
  }
  .more:hover {
    color: var(--cap-text);
    background: var(--cap-hover);
  }
  .mchev {
    display: grid;
    transform: rotate(90deg);
    transition: transform 320ms var(--spring);
  }
  .mchev.up {
    transform: rotate(-90deg);
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
    position: relative;
  }
  .chipset {
    grid-area: 1 / 1;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 1px;
    margin: -1px;
  }
  /* language picker: a dense grid that wraps; the card grows, it never scrolls */
  .chipset.langs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(98px, 1fr));
    gap: 5px;
  }
  .w-compact .chipset.langs {
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 4px;
  }
  .chip.lang,
  .chipset.langs .back {
    height: 26px;
    padding: 0 9px;
    min-width: 0;
  }
  .ln {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
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
    padding: 0 10px 0 6px;
    color: var(--cap-text);
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
    max-height: 132px;
    overflow-y: auto;
    scrollbar-width: none;
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

  .detail::-webkit-scrollbar {
    display: none;
  }

  /* in-card tooltip (replaces native title tooltips) */
  .tip {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 5;
    max-width: 250px;
    width: max-content;
    padding: 5px 9px;
    border-radius: 7px;
    background: var(--cap-text);
    color: var(--cap-bg);
    font-size: 11.5px;
    line-height: 1.35;
    font-weight: 500;
    pointer-events: none;
    opacity: 0;
    scale: 0.96;
    transition:
      opacity 120ms var(--ease),
      scale 200ms var(--ease);
  }
  .tip.on {
    opacity: 1;
    scale: 1;
    transition-duration: 160ms, 260ms;
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
    .ck,
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
