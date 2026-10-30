<script lang="ts" module>
  export type SelectOption<T> = {
    value: T;
    label: string;
    disabled?: boolean;
    /** Secondary text shown on the right (e.g. native language name). Also searched. */
    hint?: string;
    /** Options sharing a group are listed under a small header (keep them contiguous). */
    group?: string;
  };

  /** Lowercase + strip accents so "portugues" finds "Português". */
  export function fold(s: string): string {
    return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
  }

  /** Moves the node to <body> so scroll containers/overflow never clip it. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  const reduced = () => typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
</script>

<script lang="ts" generics="T extends string | null">
  // Custom dropdown (combobox + listbox). Same props as the old native <select>
  // wrapper, so every page upgrades for free.
  import { tick } from "svelte";
  import { cubicOut } from "svelte/easing";
  import Icon from "../icons/Icon.svelte";
  import { t } from "../i18n.svelte";

  let {
    value = $bindable(),
    options,
    label,
    onchange,
    mono = false,
    width,
  }: {
    value: T;
    options: SelectOption<T>[];
    label: string;
    onchange?: (v: T) => void;
    mono?: boolean;
    width?: string;
  } = $props();

  const uid = $props.id();
  const listId = `${uid}-list`;
  const optId = (i: number) => `${uid}-o${i}`;

  let open = $state(false);
  let query = $state("");
  let active = $state(-1);
  let pos = $state<{ top: number; left: number | null; right: number | null; minW: number; maxW: number; maxH: number; up: boolean }>({
    top: 0,
    left: 0,
    right: null,
    minW: 200,
    maxW: 420,
    maxH: 320,
    up: false,
  });

  let trigger: HTMLButtonElement;
  let pop = $state<HTMLDivElement>();
  let list = $state<HTMLUListElement>();
  let search = $state<HTMLInputElement>();

  const searchable = $derived(options.length > 8);
  const selected = $derived(options.find((o) => o.value === value));
  const filtered = $derived.by(() => {
    const q = fold(query.trim());
    if (!q) return options;
    return options.filter((o) => fold(`${o.label} ${o.hint ?? ""} ${o.value ?? ""}`).includes(q));
  });
  const grouped = $derived(options.some((o) => o.group) && !query.trim());

  function place() {
    if (!trigger) return;
    const r = trigger.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const gap = 6;
    const margin = 10;
    const below = vh - r.bottom - gap - margin;
    const above = r.top - gap - margin;
    const want = Math.min(searchable ? 360 : 320, 44 + options.length * 34);
    const up = below < Math.min(want, 220) && above > below;
    const maxH = Math.max(120, Math.min(want, up ? above : below));
    const maxW = Math.min(420, vw - margin * 2);
    const minW = Math.min(maxW, Math.max(r.width, searchable ? 280 : 200));
    // right-aligned triggers (most settings rows) grow the popover to the left
    const anchorRight = r.left + r.width / 2 > vw / 2;
    pos = {
      top: up ? r.top - gap : r.bottom + gap,
      left: anchorRight ? null : Math.max(margin, Math.min(r.left, vw - margin - minW)),
      right: anchorRight ? Math.max(margin, vw - r.right) : null,
      minW,
      maxW,
      maxH,
      up,
    };
  }

  async function show() {
    if (open) return;
    query = "";
    lastQuery = "";
    place();
    open = true;
    active = Math.max(
      0,
      options.findIndex((o) => o.value === value),
    );
    await tick();
    // freeze the natural width so filtering doesn't make the popover jump
    if (pop) pop.style.width = `${pop.offsetWidth + 1}px`;
    (searchable ? search : list)?.focus({ preventScroll: true });
    scrollToActive(true);
  }

  function hide(refocus = true) {
    if (!open) return;
    open = false;
    typed = "";
    if (refocus) trigger?.focus({ preventScroll: true });
  }

  function choose(o: SelectOption<T> | undefined) {
    if (!o || o.disabled) return;
    const changed = o.value !== value;
    value = o.value;
    hide();
    if (changed) onchange?.(o.value);
  }

  function scrollToActive(center = false) {
    if (!list || active < 0) return;
    const el = list.querySelector<HTMLElement>(`#${CSS.escape(optId(active))}`);
    if (!el) return;
    const top = el.offsetTop;
    const bottom = top + el.offsetHeight;
    if (center) list.scrollTop = bottom + 8 > list.clientHeight ? top - (list.clientHeight - el.offsetHeight) / 2 : 0;
    else if (top < list.scrollTop + 4) list.scrollTop = top - 4;
    else if (bottom > list.scrollTop + list.clientHeight - 4) list.scrollTop = bottom - list.clientHeight + 4;
  }

  function move(to: number, dir: 1 | -1) {
    const n = filtered.length;
    if (!n) return;
    let i = Math.max(0, Math.min(n - 1, to));
    // skip disabled entries in the travel direction
    let guard = n;
    while (filtered[i]?.disabled && guard--) {
      const next = i + dir;
      if (next < 0 || next >= n) break;
      i = next;
    }
    if (filtered[i]?.disabled) return;
    active = i;
    tick().then(() => scrollToActive());
  }

  // type-ahead (used when there is no search box)
  let typed = "";
  let typedTimer: ReturnType<typeof setTimeout> | undefined;
  function typeAhead(ch: string) {
    clearTimeout(typedTimer);
    typed += fold(ch);
    typedTimer = setTimeout(() => (typed = ""), 650);
    const start = typed.length === 1 ? active + 1 : active;
    const n = filtered.length;
    for (let k = 0; k < n; k++) {
      const i = (start + k) % n;
      if (!filtered[i].disabled && fold(filtered[i].label).startsWith(typed)) {
        active = i;
        tick().then(() => scrollToActive());
        return;
      }
    }
  }

  const printable = (e: KeyboardEvent) => e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey;

  function onTriggerKey(e: KeyboardEvent) {
    if (open) return;
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
      e.preventDefault();
      show();
    } else if (printable(e)) {
      e.preventDefault();
      show().then(() => {
        if (searchable) query = e.key;
        else typeAhead(e.key);
      });
    }
  }

  function onPopKey(e: KeyboardEvent) {
    const n = filtered.length;
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        move(active + 1, 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        move(active - 1, -1);
        break;
      case "PageDown":
        e.preventDefault();
        move(active + 8, 1);
        break;
      case "PageUp":
        e.preventDefault();
        move(active - 8, -1);
        break;
      case "Home":
        if (searchable && e.target === search && query) return; // let the caret move
        e.preventDefault();
        move(0, 1);
        break;
      case "End":
        if (searchable && e.target === search && query) return;
        e.preventDefault();
        move(n - 1, -1);
        break;
      case "Enter":
        e.preventDefault();
        choose(filtered[active]);
        break;
      case " ":
        if (searchable) return; // a space is part of the query
        e.preventDefault();
        choose(filtered[active]);
        break;
      case "Escape":
        e.preventDefault();
        e.stopPropagation();
        hide();
        break;
      case "Tab":
        hide(false);
        break;
      default:
        if (!searchable && printable(e)) {
          e.preventDefault();
          typeAhead(e.key);
        }
    }
  }

  // Filtering resets the highlight to the first enabled match.
  let lastQuery = "";
  $effect(() => {
    if (!open || query === lastQuery) return;
    lastQuery = query;
    const sel = filtered.findIndex((o) => o.value === value);
    active = query.trim() ? filtered.findIndex((o) => !o.disabled) : sel;
    if (list) list.scrollTop = 0;
  });

  // Close on outside pointer, page scroll (not the list's own), resize, window blur.
  $effect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      const tgt = e.target as Node;
      if (pop?.contains(tgt) || trigger?.contains(tgt)) return;
      hide(false);
    };
    const scroll = (e: Event) => {
      if (pop && e.target instanceof Node && pop.contains(e.target)) return;
      hide(false);
    };
    const resize = () => hide(false);
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("scroll", scroll, true);
    window.addEventListener("resize", resize);
    window.addEventListener("blur", resize);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("scroll", scroll, true);
      window.removeEventListener("resize", resize);
      window.removeEventListener("blur", resize);
    };
  });

  function drop(_node: Element, { up, out = false }: { up: boolean; out?: boolean }) {
    const d = reduced() ? 0 : out ? 110 : 190;
    return {
      duration: d,
      easing: cubicOut,
      css: (k: number) =>
        `opacity:${Math.min(1, k * 1.6)};transform:translateY(${(1 - k) * (up ? 6 : -6)}px) scale(${0.97 + 0.03 * k})`,
    };
  }
</script>

<div class="sel" style:width>
  <button
    bind:this={trigger}
    type="button"
    class="trig"
    class:mono
    class:open
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={open ? listId : undefined}
    onclick={() => (open ? hide() : show())}
    onkeydown={onTriggerKey}
  >
    <span class="val">{selected?.label ?? ""}</span>
    <span class="chev" aria-hidden="true"><Icon name="chevronDown" size={16} /></span>
  </button>
</div>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={pop}
    use:portal
    class="pop"
    class:up={pos.up}
    style:left={pos.left === null ? null : `${pos.left}px`}
    style:right={pos.right === null ? null : `${pos.right}px`}
    style:top="{pos.top}px"
    style:min-width="{pos.minW}px"
    style:max-width="{pos.maxW}px"
    style:max-height="{pos.maxH}px"
    in:drop={{ up: pos.up }}
    out:drop={{ up: pos.up, out: true }}
    onkeydown={onPopKey}
  >
    {#if searchable}
      <div class="search">
        <Icon name="search" size={15} />
        <input
          bind:this={search}
          bind:value={query}
          type="text"
          role="searchbox"
          spellcheck="false"
          autocomplete="off"
          placeholder={t("ctl.search")}
          aria-label={t("ctl.search_in", { label })}
          aria-controls={listId}
          aria-activedescendant={active >= 0 ? optId(active) : undefined}
        />
        {#if query}
          <button type="button" class="clear" aria-label={t("ctl.clear")} onclick={() => ((query = ""), search?.focus())}>
            <Icon name="x" size={13} stroke={2} />
          </button>
        {/if}
      </div>
    {/if}
    <ul
      bind:this={list}
      id={listId}
      role="listbox"
      aria-label={label}
      tabindex={searchable ? undefined : -1}
      aria-activedescendant={!searchable && active >= 0 ? optId(active) : undefined}
      class:mono
    >
      {#each filtered as o, i (o.value ?? "\u0000")}
        {#if grouped && o.group && o.group !== filtered[i - 1]?.group}
          <li class="grp" role="presentation">{o.group}</li>
        {/if}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          id={optId(i)}
          role="option"
          aria-selected={o.value === value}
          aria-disabled={o.disabled || undefined}
          class:active={i === active}
          class:on={o.value === value}
          class:dis={o.disabled}
          onpointermove={() => !o.disabled && active !== i && (active = i)}
          onclick={() => choose(o)}
        >
          <span class="ck" aria-hidden="true"><Icon name="check" size={14} stroke={2} /></span>
          <span class="ol">{o.label}</span>
          {#if o.hint}<span class="oh">{o.hint}</span>{/if}
        </li>
      {:else}
        <li class="empty" role="presentation">{t("ctl.no_results")}</li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .sel {
    position: relative;
    display: inline-block;
    min-width: 210px;
    flex: none;
  }
  .trig {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 32px;
    padding: 0 8px 0 11px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    text-align: left;
    transition:
      border-color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease),
      box-shadow var(--t-fast) var(--ease),
      transform 90ms var(--ease);
  }
  .trig.mono {
    font-family: var(--mono);
    font-size: 12px;
  }
  .trig:hover {
    border-color: var(--text-3);
  }
  .trig:active {
    transform: scale(0.985);
  }
  .trig:focus-visible,
  .trig.open {
    outline: none;
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .val {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chev {
    display: grid;
    color: var(--text-3);
    transition:
      transform var(--t) var(--spring),
      color var(--t-fast);
  }
  .open .chev {
    transform: rotate(180deg);
    color: var(--text);
  }

  /* popover lives in <body> */
  .pop {
    position: fixed;
    z-index: 200;
    width: max-content;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    transform-origin: top center;
    font-family: var(--font);
    color: var(--text);
  }
  .pop.up {
    translate: 0 -100%;
    transform-origin: bottom center;
    flex-direction: column-reverse;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 11px;
    height: 40px;
    flex: none;
    color: var(--text-3);
    border-bottom: 1px solid var(--line);
  }
  .up .search {
    border-bottom: 0;
    border-top: 1px solid var(--line);
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    background: transparent;
    font-size: 13px;
    color: var(--text);
  }
  .search input:focus-visible {
    /* the popover itself is the focus context; a ring here gets clipped by its edges */
    outline: none;
  }
  .search:focus-within {
    color: var(--text-2);
  }
  .search input::placeholder {
    color: var(--text-3);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    color: var(--text-3);
  }
  .clear:hover {
    background: var(--hover);
    color: var(--text);
  }
  ul {
    position: relative;
    list-style: none;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    overscroll-behavior: contain;
    min-height: 0;
  }
  ul:focus-visible {
    outline: none;
  }
  li[role="option"] {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 6px 10px 6px 6px;
    border-radius: 6px;
    font-size: 13px;
    color: var(--text-2);
    transition:
      background-color 80ms linear,
      color 80ms linear;
  }
  ul.mono li[role="option"] {
    font-family: var(--mono);
    font-size: 12px;
  }
  li.active {
    background: var(--hover);
    color: var(--text);
  }
  li.on {
    color: var(--text);
    font-weight: 500;
  }
  li.dis {
    opacity: 0.4;
  }
  .ck {
    display: grid;
    place-items: center;
    width: 18px;
    flex: none;
    color: var(--accent-fg);
    opacity: 0;
    transform: scale(0.5);
    transition:
      opacity var(--t-fast) var(--ease),
      transform var(--t) var(--spring);
  }
  .on .ck {
    opacity: 1;
    transform: none;
  }
  .ol {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .oh {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 400;
    color: var(--text-3);
  }
  .grp {
    padding: 10px 10px 4px 30px;
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .grp:first-child {
    padding-top: 6px;
  }
  .empty {
    padding: 14px 10px;
    text-align: center;
    font-size: 12.5px;
    color: var(--text-3);
  }
</style>
