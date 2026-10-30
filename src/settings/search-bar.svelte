<script lang="ts" module>
  /** Moves the node to <body> so the title bar never clips it. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
  const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
</script>

<script lang="ts">
  // Global settings search: compact field in the title bar (Ctrl+K / Ctrl+F) with a
  // keyboard-navigable results popover. Picking a result calls `onpick`.
  import { tick } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { flip } from "svelte/animate";
  import { t } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import { searchSettings, type SearchHit } from "./search-index";

  let { onpick }: { onpick: (hit: SearchHit) => void } = $props();

  const uid = $props.id();
  const listId = `${uid}-res`;

  let q = $state("");
  let focused = $state(false);
  let active = $state(0);
  let input = $state<HTMLInputElement>();
  let box = $state<HTMLElement>();
  let pop = $state<HTMLElement>();
  let list = $state<HTMLElement>();
  let pos = $state({ top: 0, left: 0, width: 420 });
  let bar = $state({ y: 0, h: 0, on: false, snap: true });

  const hits = $derived(searchSettings(q, 8));
  const open = $derived(focused && q.trim().length > 0);

  $effect(() => {
    hits;
    active = 0;
  });

  function place() {
    if (!box) return;
    const r = box.getBoundingClientRect();
    const vw = window.innerWidth;
    const width = Math.min(Math.max(r.width, 440), vw - 20);
    pos = { top: r.bottom + 8, left: Math.max(10, Math.min(r.left, vw - 10 - width)), width };
  }

  $effect(() => {
    if (!open) return;
    place();
    addEventListener("resize", place);
    return () => removeEventListener("resize", place);
  });

  // sliding highlight behind the active result
  $effect(() => {
    active;
    hits;
    list;
    if (!open) return void (bar.on = false);
    tick().then(() => {
      const el = list?.querySelector<HTMLElement>(`[data-i="${active}"]`);
      if (!el) return void (bar.on = false);
      // first placement after opening jumps into place; later moves slide
      bar = { y: el.offsetTop, h: el.offsetHeight, on: true, snap: !bar.on };
      el.scrollIntoView({ block: "nearest" });
    });
  });

  // global shortcut
  $effect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && (e.key === "k" || e.key === "K" || e.key === "f" || e.key === "F")) {
        if (document.querySelector("[role='dialog']")) return;
        e.preventDefault();
        input?.focus();
        input?.select();
      }
    };
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  });

  function pick(h: SearchHit | undefined) {
    if (!h) return;
    q = "";
    input?.blur();
    onpick(h);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (q) q = "";
      else input?.blur();
      return;
    }
    if (!open || !hits.length) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      active = (active + 1) % hits.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = (active - 1 + hits.length) % hits.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      pick(hits[active]);
    }
  }

  function drop(_n: Element, { out = false }: { out?: boolean } = {}) {
    const d = reduced() ? 0 : out ? 120 : 220;
    return {
      duration: d,
      easing: cubicOut,
      css: (k: number) => `opacity:${Math.min(1, k * 1.5)};transform:translateY(${(1 - k) * -8}px) scale(${0.975 + 0.025 * k})`,
    };
  }
  function rise(_n: Element, { i }: { i: number }) {
    return {
      duration: reduced() ? 0 : 200,
      delay: reduced() ? 0 : Math.min(i, 6) * 18,
      easing: cubicOut,
      css: (k: number) => `opacity:${k};transform:translateY(${(1 - k) * 5}px)`,
    };
  }

  const parts = (h: SearchHit) =>
    h.mark ? [h.label.slice(0, h.mark[0]), h.label.slice(h.mark[0], h.mark[1]), h.label.slice(h.mark[1])] : [h.label, "", ""];
</script>

<div class="sb" class:focused bind:this={box}>
  <span class="ic"><Icon name="search" size={14} /></span>
  <input
    bind:this={input}
    bind:value={q}
    type="text"
    role="combobox"
    spellcheck="false"
    autocomplete="off"
    placeholder={t("sh.search")}
    aria-label={t("sh.search_label")}
    aria-expanded={open}
    aria-controls={open ? listId : undefined}
    aria-autocomplete="list"
    aria-activedescendant={open && hits.length ? `${uid}-r${active}` : undefined}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    onkeydown={onKey}
  />
  {#if q}
    <button type="button" class="clr" aria-label={t("ctl.clear")} onmousedown={(e) => e.preventDefault()} onclick={() => (q = "")}>
      <Icon name="x" size={12} stroke={2} />
    </button>
  {:else}
    <kbd class="kb mono" aria-hidden="true">Ctrl K</kbd>
  {/if}
</div>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="pop"
    bind:this={pop}
    use:portal
    style:top="{pos.top}px"
    style:left="{pos.left}px"
    style:width="{pos.width}px"
    in:drop
    out:drop={{ out: true }}
    onmousedown={(e) => e.preventDefault()}
  >
    {#if hits.length}
      <ul id={listId} role="listbox" aria-label={t("sh.search_label")} bind:this={list}>
        <li class="hl" class:on={bar.on} class:snap={bar.snap} style:transform="translate3d(0, {bar.y}px, 0)" style:height="{bar.h}px" role="presentation" aria-hidden="true"></li>
        {#each hits as h, i (h.uid)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            id="{uid}-r{i}"
            data-i={i}
            role="option"
            aria-selected={i === active}
            class:active={i === active}
            onpointermove={() => active !== i && (active = i)}
            onclick={() => pick(h)}
            in:rise={{ i }}
            animate:flip={{ duration: reduced() ? 0 : 180, easing: cubicOut }}
          >
            <span class="pi"><Icon name={h.icon} size={15} /></span>
            <span class="tx">
              <span class="lb">{parts(h)[0]}<mark>{parts(h)[1]}</mark>{parts(h)[2]}</span>
              {#if h.hint}<span class="hn">{h.hint}</span>{/if}
            </span>
            <span class="pg mono">{h.pageLabel}</span>
            <span class="go" aria-hidden="true"><Icon name="arrowRight" size={13} /></span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">{t("sh.search_empty", { q: q.trim() })}</p>
    {/if}
    <footer class="mono" aria-hidden="true">
      <span><kbd>↑</kbd><kbd>↓</kbd>{t("sh.search_nav")}</span>
      <span><kbd>Enter</kbd>{t("sh.search_open")}</span>
      <span><kbd>Esc</kbd>{t("sh.search_close")}</span>
    </footer>
  </div>
{/if}

<style>
  .sb {
    position: relative;
    display: flex;
    align-items: center;
    gap: 7px;
    width: min(300px, 38vw);
    height: 28px;
    padding: 0 5px 0 9px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--text-3);
    transition:
      border-color var(--t-fast) var(--ease),
      box-shadow var(--t) var(--ease),
      background-color var(--t-fast) var(--ease);
  }
  .sb:hover {
    border-color: var(--line-strong);
  }
  .sb.focused {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
    color: var(--text-2);
  }
  .ic {
    display: grid;
    transition: transform var(--t) var(--spring), color var(--t) var(--ease);
  }
  .focused .ic {
    color: var(--accent-fg);
    transform: scale(1.08);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    background: none;
    font-size: 12.5px;
    color: var(--text);
  }
  input::placeholder {
    color: var(--text-3);
  }
  input:focus,
  input:focus-visible {
    outline: none;
  }
  .kb {
    flex: none;
    height: 18px;
    padding: 0 5px;
    border-radius: 4px;
    border: 1px solid var(--line-strong);
    border-bottom-width: 2px;
    font-size: 10px;
    line-height: 15px;
    color: var(--text-3);
    animation: fv-fade var(--t) var(--ease) backwards;
  }
  .focused .kb {
    opacity: 0;
  }
  .clr {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-3);
    animation: fv-pop var(--t) var(--spring) backwards;
  }
  .clr:hover {
    background: var(--hover);
    color: var(--text);
  }

  /* ---- popover (in <body>) */
  .pop {
    position: fixed;
    z-index: 220;
    display: flex;
    flex-direction: column;
    max-height: min(440px, calc(100vh - 64px));
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    transform-origin: 30% 0;
    font-family: var(--font);
    color: var(--text);
    user-select: none;
  }
  ul {
    position: relative;
    list-style: none;
    margin: 0;
    padding: 5px;
    overflow-y: auto;
    overscroll-behavior: contain;
    min-height: 0;
  }
  .hl {
    position: absolute;
    min-height: 0;
    padding: 0;
    left: 5px;
    right: 5px;
    top: 0;
    border-radius: 8px;
    background: var(--hover);
    box-shadow: inset 0 0 0 1px var(--line);
    opacity: 0;
    pointer-events: none;
    transition:
      transform 220ms var(--spring),
      height var(--t-fast) var(--ease),
      opacity var(--t-fast) var(--ease);
  }
  .hl.snap {
    transition: opacity var(--t-fast) var(--ease);
  }
  .hl::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 2px;
    height: 14px;
    margin-top: -7px;
    border-radius: 0 2px 2px 0;
    background: var(--accent-fg);
  }
  .hl.on {
    opacity: 1;
  }
  li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 6px 10px 6px 9px;
    border-radius: 8px;
    color: var(--text-2);
  }
  li.active {
    color: var(--text);
  }
  .pi {
    flex: none;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--text-3);
    transition:
      color var(--t-fast) var(--ease),
      border-color var(--t-fast) var(--ease),
      transform var(--t) var(--spring);
  }
  li.active .pi {
    color: var(--accent-fg);
    border-color: var(--accent-line);
    transform: scale(1.04);
  }
  .tx {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .lb {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  mark {
    background: none;
    color: var(--accent-fg);
    box-shadow: inset 0 -1px 0 var(--accent-line);
  }
  .hn {
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pg {
    flex: none;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .go {
    flex: none;
    display: grid;
    width: 13px;
    color: var(--accent-fg);
    opacity: 0;
    transform: translate3d(-4px, 0, 0);
    transition:
      opacity var(--t-fast) var(--ease),
      transform var(--t) var(--ease);
  }
  li.active .go {
    opacity: 1;
    transform: none;
  }
  .empty {
    padding: 18px 16px;
    font-size: 12.5px;
    color: var(--text-3);
  }
  footer {
    flex: none;
    display: flex;
    gap: 14px;
    padding: 7px 12px;
    border-top: 1px solid var(--line);
    background: var(--surface-2);
    font-size: 10px;
    color: var(--text-3);
  }
  footer span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  footer kbd {
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    margin-right: 2px;
    display: inline-grid;
    place-items: center;
    border-radius: 4px;
    border: 1px solid var(--line-strong);
    font: inherit;
    color: var(--text-2);
  }
  @media (max-width: 640px) {
    .kb {
      display: none;
    }
  }
</style>
