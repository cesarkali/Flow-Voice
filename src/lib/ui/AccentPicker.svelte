<script lang="ts" module>
  type Hsv = { h: number; s: number; v: number };

  function hexToRgb(hex: string): [number, number, number] | null {
    const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return null;
    let h = m[1];
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
  }
  function rgbToHsv([r, g, b]: [number, number, number]): Hsv {
    r /= 255;
    g /= 255;
    b /= 255;
    const max = Math.max(r, g, b);
    const d = max - Math.min(r, g, b);
    let h = 0;
    if (d) {
      if (max === r) h = ((g - b) / d) % 6;
      else if (max === g) h = (b - r) / d + 2;
      else h = (r - g) / d + 4;
      h *= 60;
      if (h < 0) h += 360;
    }
    return { h, s: max ? d / max : 0, v: max };
  }
  function hsvToHex({ h, s, v }: Hsv): string {
    const f = (n: number) => {
      const k = (n + h / 60) % 6;
      return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
    };
    return "#" + [f(5), f(3), f(1)].map((x) => Math.round(x * 255).toString(16).padStart(2, "0")).join("").toUpperCase();
  }
  const clamp = (x: number, a = 0, b = 1) => Math.min(b, Math.max(a, x));

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
  const reduced = () => typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
</script>

<script lang="ts">
  // Accent color: curated swatches + a custom swatch that opens a color window
  // (saturation/brightness square, hue strip, hex field). Applies live through settings.accent_color.
  import { tick } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { t } from "../i18n.svelte";
  import Icon from "../icons/Icon.svelte";
  import { ACCENT_PRESETS, isValidAccent } from "../theme";

  let {
    value = $bindable("#D7FF3F"),
    label,
    customLabel,
    large = false,
  }: { value: string; label: string; customLabel: string; large?: boolean } = $props();

  const uid = $props.id();
  const norm = (v: string) => {
    let h = v.trim().replace(/^#/, "").toUpperCase();
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    return "#" + h;
  };
  const cur = $derived(isValidAccent(value) ? norm(value) : "#D7FF3F");
  const isPreset = $derived(ACCENT_PRESETS.includes(cur));

  // ---- popover state
  let open = $state(false);
  let before = $state("#D7FF3F");
  let hsv = $state<Hsv>({ h: 72, s: 0.75, v: 1 });
  let draft = $state("");
  let trigger = $state<HTMLButtonElement>();
  let pop = $state<HTMLDivElement>();
  let svEl = $state<HTMLDivElement>();
  let hueEl = $state<HTMLDivElement>();
  let pos = $state({ top: 0, left: 0, up: false });

  const live = $derived(hsvToHex(hsv));

  // throttled push to `value` while dragging (the theme + autosave follow it)
  let pushT = 0;
  let pushPending = false;
  function push(now = false) {
    if (now) {
      clearTimeout(pushT);
      pushPending = false;
      if (value.toUpperCase() !== live) value = live;
      return;
    }
    if (pushPending) return;
    pushPending = true;
    pushT = window.setTimeout(() => {
      pushPending = false;
      if (value.toUpperCase() !== live) value = live;
    }, 60);
  }
  function setHsv(n: Partial<Hsv>, now = false) {
    hsv = { ...hsv, ...n };
    draft = hsvToHex(hsv);
    push(now);
  }

  function place() {
    if (!trigger) return;
    const r = trigger.getBoundingClientRect();
    const w = 264;
    const h = 330;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const up = vh - r.bottom < h + 16 && r.top > vh - r.bottom;
    pos = {
      top: up ? r.top - 8 : r.bottom + 8,
      left: clamp(r.left + r.width / 2 - w / 2, 10, vw - w - 10),
      up,
    };
  }

  async function show() {
    if (open) return hide(true);
    before = cur;
    const rgb = hexToRgb(cur)!;
    const n = rgbToHsv(rgb);
    // keep the hue when the color is grey (hue undefined)
    hsv = n.s === 0 ? { ...n, h: hsv.h } : n;
    draft = cur;
    place();
    open = true;
    await tick();
    svEl?.focus({ preventScroll: true });
  }
  function hide(keep: boolean) {
    if (!open) return;
    if (keep) push(true);
    else {
      clearTimeout(pushT);
      pushPending = false;
      value = before;
    }
    open = false;
    trigger?.focus({ preventScroll: true });
  }

  // outside click keeps the color; Escape reverts
  $effect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      const tg = e.target as Node;
      if (!pop?.contains(tg) && !trigger?.contains(tg)) hide(true);
    };
    const onResize = () => place();
    document.addEventListener("pointerdown", down, true);
    addEventListener("resize", onResize);
    return () => {
      document.removeEventListener("pointerdown", down, true);
      removeEventListener("resize", onResize);
    };
  });

  // ---- pointer dragging
  function drag(el: HTMLElement | undefined, e: PointerEvent, apply: (x: number, y: number) => void) {
    if (!el || e.button !== 0) return;
    e.preventDefault();
    el.focus({ preventScroll: true });
    el.setPointerCapture(e.pointerId);
    const r = el.getBoundingClientRect();
    const at = (ev: PointerEvent) => apply(clamp((ev.clientX - r.left) / r.width), clamp((ev.clientY - r.top) / r.height));
    at(e);
    const move = (ev: PointerEvent) => at(ev);
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      push(true);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  }

  function svKey(e: KeyboardEvent) {
    const st = e.shiftKey ? 0.1 : 0.01;
    const m: Record<string, Partial<Hsv>> = {
      ArrowLeft: { s: clamp(hsv.s - st) },
      ArrowRight: { s: clamp(hsv.s + st) },
      ArrowUp: { v: clamp(hsv.v + st) },
      ArrowDown: { v: clamp(hsv.v - st) },
    };
    if (m[e.key]) {
      e.preventDefault();
      setHsv(m[e.key]);
    }
  }
  function hueKey(e: KeyboardEvent) {
    const st = e.shiftKey ? 10 : 1;
    let h = hsv.h;
    if (e.key === "ArrowLeft" || e.key === "ArrowDown") h -= st;
    else if (e.key === "ArrowRight" || e.key === "ArrowUp") h += st;
    else if (e.key === "Home") h = 0;
    else if (e.key === "End") h = 359;
    else return;
    e.preventDefault();
    setHsv({ h: (h + 360) % 360 });
  }
  function popKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      hide(false);
    } else if (e.key === "Enter" && (e.target as HTMLElement).tagName !== "BUTTON") {
      e.preventDefault();
      commitDraft();
      hide(true);
    }
  }
  function commitDraft() {
    if (!isValidAccent(draft)) return;
    const n = rgbToHsv(hexToRgb(draft)!);
    hsv = n.s === 0 ? { ...n, h: hsv.h } : n;
    draft = norm(draft);
    push(true);
  }
  function onDraft() {
    if (/^#?[0-9a-f]{6}$/i.test(draft.trim())) commitDraft();
  }

  function pickPreset(c: string) {
    if (open) hide(true);
    value = c;
  }

  function drop(_n: Element, { up, out = false }: { up: boolean; out?: boolean }) {
    const d = reduced() ? 0 : out ? 130 : 240;
    return {
      duration: d,
      easing: cubicOut,
      css: (k: number) => `opacity:${Math.min(1, k * 1.5)};transform:translateY(${(1 - k) * (up ? 8 : -8)}px) scale(${0.96 + 0.04 * k})`,
    };
  }
</script>

<div class="ap" class:large role="radiogroup" aria-label={label}>
  {#each ACCENT_PRESETS as c, i (c)}
    <button
      type="button"
      role="radio"
      class="swatch"
      class:on={cur === c}
      style:--sw={c}
      style:--d="{i * 25}ms"
      aria-checked={cur === c}
      aria-label={c}
      title={c}
      onclick={() => pickPreset(c)}
    ></button>
  {/each}
  <button
    bind:this={trigger}
    type="button"
    class="swatch custom"
    class:on={!isPreset}
    class:open
    style:--sw={isPreset ? "transparent" : cur}
    style:--d="{ACCENT_PRESETS.length * 25}ms"
    aria-haspopup="dialog"
    aria-expanded={open}
    aria-label={t("sh.color_custom")}
    title={t("sh.color_custom")}
    onclick={show}
  >
    <span class="plus" aria-hidden="true"><Icon name="plus" size={large ? 15 : 13} stroke={2} /></span>
  </button>
  <span class="hexout mono" aria-live="polite">{cur}</span>
</div>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={pop}
    use:portal
    class="cp"
    class:up={pos.up}
    role="dialog"
    tabindex="-1"
    aria-label={t("sh.color_custom")}
    style:top="{pos.top}px"
    style:left="{pos.left}px"
    style:--hue="hsl({hsv.h} 100% 50%)"
    style:--live={live}
    in:drop={{ up: pos.up }}
    out:drop={{ up: pos.up, out: true }}
    onkeydown={popKey}
  >
    <div
      bind:this={svEl}
      class="sv"
      role="slider"
      tabindex="0"
      aria-label={t("sh.color_sv")}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(hsv.s * 100)}
      aria-valuetext={t("sh.color_value", { s: Math.round(hsv.s * 100), v: Math.round(hsv.v * 100) })}
      onpointerdown={(e) => drag(svEl, e, (x, y) => setHsv({ s: x, v: 1 - y }))}
      onkeydown={svKey}
    >
      <span class="thumb" style:transform="translate3d({hsv.s * 100}cqw, {(1 - hsv.v) * 100}cqh, 0)"></span>
    </div>

    <div
      bind:this={hueEl}
      class="hue"
      role="slider"
      tabindex="0"
      aria-label={t("sh.color_hue")}
      aria-valuemin={0}
      aria-valuemax={359}
      aria-valuenow={Math.round(hsv.h)}
      onpointerdown={(e) => drag(hueEl, e, (x) => setHsv({ h: Math.min(359.9, x * 360) }))}
      onkeydown={hueKey}
    >
      <span class="hthumb" style:transform="translate3d({(hsv.h / 360) * 100}cqw, 0, 0)"></span>
    </div>

    <div class="row">
      <span class="cmp" aria-hidden="true">
        <span class="old" style:--c={before} title={t("sh.color_before")}></span>
        <span class="new"></span>
      </span>
      <label class="hex" class:bad={draft !== "" && !isValidAccent(draft)}>
        <span class="hash mono">#</span>
        <input
          type="text"
          maxlength="7"
          spellcheck="false"
          aria-label={customLabel}
          value={draft.replace(/^#/, "")}
          oninput={(e) => {
            draft = "#" + (e.currentTarget as HTMLInputElement).value.replace(/^#/, "");
            onDraft();
          }}
          onchange={commitDraft}
        />
      </label>
      <button type="button" class="apply" onclick={() => (commitDraft(), hide(true))}>
        <Icon name="check" size={14} stroke={2} />
        {t("sh.color_apply")}
      </button>
    </div>
  </div>
{/if}

<style>
  .ap {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .swatch {
    --s: 22px;
    position: relative;
    width: var(--s);
    height: var(--s);
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--sw);
    box-shadow: 0 0 0 1px var(--line-strong);
    cursor: pointer;
    animation: sw-in 360ms var(--spring) both;
    animation-delay: var(--d);
    transition:
      transform var(--t-fast) var(--spring),
      box-shadow var(--t) var(--ease);
  }
  .large .swatch {
    --s: 28px;
  }
  .swatch:hover {
    transform: scale(1.14);
  }
  .swatch:active {
    transform: scale(0.9);
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--sw);
    transform: scale(1.06);
  }
  .swatch:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 4px;
  }
  /* custom swatch: dashed ring + plus when empty, the chosen color when set */
  .custom {
    display: grid;
    place-items: center;
    color: var(--text-2);
    box-shadow: inset 0 0 0 1px var(--line-strong);
    background:
      conic-gradient(from 0deg, #ff5f5f, #ffd84a, #7dff6a, #4ae0ff, #6f7bff, #ff5fd2, #ff5f5f) border-box;
  }
  .custom::before {
    /* inner disc that leaves a 2px rainbow rim */
    content: "";
    position: absolute;
    inset: 2px;
    border-radius: 50%;
    background: var(--surface);
    transition: background-color var(--t) var(--ease);
  }
  .custom.on::before {
    background: var(--sw);
  }
  .custom.on {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--sw);
  }
  .plus {
    position: relative;
    display: grid;
    transition:
      transform var(--t) var(--spring),
      opacity var(--t-fast) var(--ease);
  }
  .custom.on .plus {
    opacity: 0;
    transform: scale(0.5);
  }
  .custom.on:hover .plus,
  .custom.open .plus {
    opacity: 1;
    transform: none;
    color: var(--on-accent);
  }
  .custom.open .plus {
    transform: rotate(45deg);
  }
  .hexout {
    margin-left: 4px;
    font-size: 11.5px;
    color: var(--text-3);
    min-width: 7ch;
  }

  /* ---- color window (in <body>) */
  .cp {
    position: fixed;
    z-index: 230;
    width: 264px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    box-shadow: var(--shadow-pop);
    transform-origin: 50% 0;
    font-family: var(--font);
    color: var(--text);
  }
  .cp.up {
    translate: 0 -100%;
    transform-origin: 50% 100%;
  }
  .cp:focus {
    outline: none;
  }
  .sv {
    position: relative;
    height: 150px;
    border-radius: 9px;
    background:
      linear-gradient(to top, #000, transparent),
      linear-gradient(to right, #fff, var(--hue));
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
    container-type: size;
    touch-action: none;
    cursor: crosshair;
  }
  .hue {
    position: relative;
    height: 14px;
    border-radius: 7px;
    background: linear-gradient(to right, #f00, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
    container-type: size;
    touch-action: none;
  }
  .sv:focus-visible,
  .hue:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
  .thumb,
  .hthumb {
    position: absolute;
    left: 0;
    top: 0;
    width: 16px;
    height: 16px;
    margin: -8px 0 0 -8px;
    border-radius: 50%;
    background: var(--live);
    box-shadow:
      0 0 0 2px #fff,
      0 0 0 3px rgba(0, 0, 0, 0.35),
      0 2px 6px rgba(0, 0, 0, 0.35);
    pointer-events: none;
  }
  .hthumb {
    top: 50%;
    background: var(--hue);
  }
  .sv:active .thumb,
  .hue:active .hthumb {
    box-shadow:
      0 0 0 3px #fff,
      0 0 0 4px rgba(0, 0, 0, 0.35),
      0 3px 10px rgba(0, 0, 0, 0.4);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cmp {
    flex: none;
    display: flex;
    width: 40px;
    height: 30px;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .cmp span {
    flex: 1;
  }
  .old {
    background: var(--c);
  }
  .new {
    background: var(--live);
  }
  .hex {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 30px;
    padding: 0 8px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    background: var(--bg);
    transition: border-color var(--t-fast) var(--ease);
  }
  .hex:focus-within {
    border-color: var(--accent-line);
  }
  .hex.bad {
    border-color: var(--danger-line);
  }
  .hash {
    color: var(--text-3);
    font-size: 12px;
  }
  .hex input {
    width: 100%;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: 12px var(--mono);
    text-transform: uppercase;
  }
  .hex input:focus {
    outline: none;
  }
  .apply {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 11px;
    border-radius: var(--r);
    background: var(--live);
    color: var(--on-accent);
    font-size: 12.5px;
    font-weight: 600;
    transition: transform var(--t) var(--spring);
  }
  .apply:hover {
    transform: translateY(-1px);
  }
  .apply:active {
    transform: scale(0.96);
    transition-duration: 60ms;
  }
  @keyframes sw-in {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .swatch {
      animation: none;
      transition: none;
    }
  }
</style>
