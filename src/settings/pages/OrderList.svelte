<script lang="ts" generics="T extends string">
  import { flip } from "svelte/animate";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import ProviderLogo from "../../lib/icons/ProviderLogo.svelte";

  let {
    items = $bindable(),
    label,
    nameOf,
    ready,
  }: { items: T[]; label: string; nameOf: (id: T) => string; ready: (id: T) => boolean } = $props();

  // Pointer-driven drag (HTML5 drag & drop doesn't fire inside the Tauri webview on Windows).
  let dragging = $state<T | null>(null);
  let dy = $state(0);
  let startY = 0;
  let pitch = 40;
  const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  function move(from: number, to: number) {
    if (to < 0 || to >= items.length || from === to) return;
    const next = [...items];
    const [x] = next.splice(from, 1);
    next.splice(to, 0, x);
    items = next;
  }

  function down(e: PointerEvent, id: T) {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
    const li = e.currentTarget as HTMLElement;
    pitch = li.offsetHeight + 4; // row + gap
    startY = e.clientY;
    dy = 0;
    dragging = id;
    try {
      li.setPointerCapture(e.pointerId);
    } catch {
      /* synthetic pointer */
    }
    e.preventDefault();
  }
  function drag(e: PointerEvent) {
    if (dragging === null) return;
    dy = e.clientY - startY;
    const from = items.indexOf(dragging);
    const to = Math.max(0, Math.min(items.length - 1, from + Math.round(dy / pitch)));
    if (to !== from) {
      move(from, to);
      // the row's resting place moved; keep it under the pointer
      startY += (to - from) * pitch;
      dy = e.clientY - startY;
    }
  }
  function up() {
    dragging = null;
    dy = 0;
  }
  function key(e: KeyboardEvent, i: number) {
    if (e.altKey && e.key === "ArrowUp") {
      e.preventDefault();
      move(i, i - 1);
    } else if (e.altKey && e.key === "ArrowDown") {
      e.preventDefault();
      move(i, i + 1);
    }
  }
</script>

<ol class="order" aria-label={label}>
  {#each items as id, i (id)}
    <li
      animate:flip={{ duration: dragging === id || reduce ? 0 : 220, easing: (x) => 1 - Math.pow(1 - x, 3) }}
      class:dragging={dragging === id}
      class:off={!ready(id)}
      title={ready(id) ? undefined : `${nameOf(id)} · ${t("prov.no_key_short")}`}
      style:transform={dragging === id ? `translate3d(0, ${dy}px, 0) scale(1.015)` : null}
      onpointerdown={(e) => down(e, id)}
      onpointermove={drag}
      onpointerup={up}
      onpointercancel={up}
      onlostpointercapture={up}
    >
      <span class="grip" title={t("c.drag")}><Icon name="grip" size={16} /></span>
      <span class="pos mono">{i + 1}</span>
      <span class="lg"><ProviderLogo {id} size={18} /></span>
      <span class="nm">{nameOf(id)}</span>
      {#if !ready(id)}<span class="faint mono tiny">{t("prov.no_key_short")}</span>{/if}
      <span class="arrows">
        <button type="button" aria-label="{t('c.move_up')}: {nameOf(id)}" disabled={i === 0} onclick={() => move(i, i - 1)} onkeydown={(e) => key(e, i)}>
          <Icon name="chevronUp" size={15} />
        </button>
        <button
          type="button"
          aria-label="{t('c.move_down')}: {nameOf(id)}"
          disabled={i === items.length - 1}
          onclick={() => move(i, i + 1)}
          onkeydown={(e) => key(e, i)}
        >
          <Icon name="chevronDown" size={15} />
        </button>
      </span>
    </li>
  {/each}
</ol>

<style>
  .order {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    container-type: inline-size;
  }
  /* narrow columns (two lists side by side): the name keeps the room; the dimmed, grayscale row
     already says "sem chave" (and its tooltip spells it out) */
  @container (max-width: 380px) {
    .tiny {
      display: none;
    }
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 4px 0 6px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--surface);
    font-size: 13px;
    cursor: grab;
    user-select: none;
    touch-action: none;
    transition:
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease),
      box-shadow var(--t) var(--ease);
  }
  li:hover {
    border-color: var(--line-strong);
  }
  li:hover .grip {
    color: var(--text-2);
  }
  li.dragging {
    position: relative;
    z-index: 2;
    cursor: grabbing;
    border-color: var(--accent-line);
    background: var(--surface-2);
    box-shadow: var(--shadow-pop);
  }
  li.off .nm {
    color: var(--text-3);
  }
  .lg {
    display: grid;
    transition:
      opacity var(--t) var(--ease),
      filter var(--t) var(--ease);
  }
  li.off .lg {
    opacity: 0.45;
    filter: grayscale(1);
  }
  .grip {
    color: var(--text-3);
    display: grid;
    transition: color var(--t-fast) var(--ease);
  }
  .pos {
    transition: color var(--t) var(--ease);
  }
  .pos {
    width: 14px;
    font-size: 11px;
    color: var(--text-3);
  }
  li:first-child .pos {
    color: var(--accent-fg);
  }
  .nm {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tiny {
    font-size: 10.5px;
  }
  .arrows {
    display: flex;
  }
  .arrows button {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-2);
  }
  .arrows button:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .arrows button:disabled {
    opacity: 0.25;
  }
</style>
