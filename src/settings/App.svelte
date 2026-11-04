<script lang="ts">
  import { onMount, tick } from "svelte";
  import { fade } from "svelte/transition";
  import { api, on } from "../lib/api";
  import { t, type TKey } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import Logo from "../lib/icons/Logo.svelte";
  import ProviderLogo from "../lib/icons/ProviderLogo.svelte";
  import type { IconName } from "../lib/icons/paths";
  import { app, hasKey, initApp, usableLlm } from "../lib/stores.svelte";
  import { EVENTS, type SettingsPage } from "../lib/types";
  import SaveIndicator from "../lib/ui/SaveIndicator.svelte";
  import TitleBar from "../lib/ui/TitleBar.svelte";
  import { rememberAppearance } from "../lib/ui/appearance-cache";
  import ModelNotice from "./ModelNotice.svelte";
  import WhatsNew from "./WhatsNew.svelte";
  import Onboarding from "./onboarding/Onboarding.svelte";
  import About from "./pages/About.svelte";
  import General from "./pages/General.svelte";
  import Home from "./pages/Home.svelte";
  import Providers from "./pages/Providers.svelte";
  import Shortcuts from "./pages/Shortcuts.svelte";
  import Speech from "./pages/Speech.svelte";
  import Styles from "./pages/Styles.svelte";
  import { pname } from "./providers";
  import SearchBar from "./search-bar.svelte";
  import { findTarget, reveal } from "./search-hit";
  import type { SearchHit } from "./search-index";
  import { donateOptions, listenUpdates, upd } from "./shell-update.svelte";

  type NavItem = { id: SettingsPage; icon: IconName; key: TKey };
  const GROUPS: { key: TKey | null; items: NavItem[] }[] = [
    {
      key: "sh.group_use",
      items: [
        { id: "home", icon: "home", key: "nav.home" },
        { id: "styles", icon: "styles", key: "nav.styles" },
      ],
    },
    {
      key: "sh.group_setup",
      items: [
        { id: "providers", icon: "key", key: "nav.providers" },
        { id: "speech", icon: "wave", key: "nav.speech" },
        { id: "shortcuts", icon: "keyboard", key: "nav.shortcuts" },
        { id: "general", icon: "sliders", key: "nav.general" },
      ],
    },
    { key: null, items: [{ id: "about", icon: "info", key: "nav.about" }] },
  ];
  const NAV = GROUPS.flatMap((g) => g.items);
  const ORDER = Object.fromEntries(NAV.map((n, i) => [n.id, i])) as Record<SettingsPage, number>;
  const PAGES = { home: Home, styles: Styles, providers: Providers, speech: Speech, shortcuts: Shortcuts, general: General, about: About };


  const initial = new URLSearchParams(location.search).get("page") as SettingsPage | null;
  let page = $state<SettingsPage>(initial && initial in PAGES ? initial : "home");
  /** +1 when moving down the nav, -1 when moving up: the new page rises from that side. */
  let dir = $state(1);
  let content: HTMLElement | undefined = $state();
  let navEl: HTMLElement | undefined = $state();
  let loadError = $state("");
  let version = $state("");
  let scrolled = $state(false);
  let ind = $state({ y: 0, h: 0, ready: false });
  let donOpen = $state(false);
  let donEl: HTMLElement | undefined = $state();

  function donate(url: string) {
    donOpen = false;
    api.openUrl(url);
  }

  // keep the first-frame appearance cache fresh (chat window + next launch paint with it)
  $effect(() => {
    const st = app.settings;
    if (st) rememberAppearance(st.theme, st.accent_color);
  });

  // close the compact-mode donation popover on outside click / Escape
  $effect(() => {
    if (!donOpen) return;
    const down = (e: PointerEvent) => !donEl?.contains(e.target as Node) && (donOpen = false);
    const key = (e: KeyboardEvent) => e.key === "Escape" && (donOpen = false);
    document.addEventListener("pointerdown", down);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", down);
      document.removeEventListener("keydown", key);
    };
  });

  const updAvail = $derived(!!upd.info?.available && !!upd.info.version);

  /** Active providers shown on the "Provedores" nav item: current voice (cloud) and text provider. */
  const provMarks = $derived.by(() => {
    const s = app.settings;
    if (!s) return { ids: [] as string[], tip: "" };
    const stt = s.stt.engine === "local" ? "local" : s.stt.cloud_order.find((p) => hasKey(p));
    const llm = usableLlm()[0];
    const ids = [...new Set([stt, llm].filter(Boolean) as string[])];
    const nm = (id?: string) => (!id ? t("home.none") : id === "local" ? t("speech.local_title") : pname(id));
    return { ids, tip: t("sh.prov_active", { stt: nm(stt), llm: nm(llm) }) };
  });

  /** Search result picked: open its page, then scroll to + pulse the matching row. */
  async function onPick(h: SearchHit) {
    const changed = h.page !== page;
    go(h.page);
    if (h.pageOnly) return;
    await tick();
    const t0 = performance.now();
    const find = () => {
      const pages = content?.querySelectorAll<HTMLElement>(":scope > .stage > .page");
      const pg = pages?.[pages.length - 1];
      const el = pg && findTarget(pg, h.label, h.id);
      if (el && content) return reveal(content, el);
      if (performance.now() - t0 < (changed ? 900 : 300)) requestAnimationFrame(find);
    };
    requestAnimationFrame(find);
  }

  function go(p: SettingsPage) {
    if (p === page || !(p in PAGES)) return;
    dir = ORDER[p] >= ORDER[page] ? 1 : -1;
    page = p;
    content?.scrollTo({ top: 0 });
    scrolled = false;
  }

  function navKey(e: KeyboardEvent) {
    const btns = [...(navEl?.querySelectorAll<HTMLButtonElement>("button[data-id]") ?? [])];
    const i = btns.indexOf(document.activeElement as HTMLButtonElement);
    let j = -1;
    if (e.key === "ArrowDown") j = (i + 1) % btns.length;
    else if (e.key === "ArrowUp") j = (i - 1 + btns.length) % btns.length;
    else if (e.key === "Home") j = 0;
    else if (e.key === "End") j = btns.length - 1;
    if (j < 0) return;
    e.preventDefault();
    btns[j].focus();
    go(btns[j].dataset.id as SettingsPage);
  }

  /** Moves the sliding active pill under the current nav button. */
  function placeIndicator() {
    const b = navEl?.querySelector<HTMLElement>(`button[data-id="${page}"]`);
    if (!b || !navEl) return;
    const first = !ind.ready && ind.h === 0;
    // rect delta, not offsetTop: while the <li> enter animation runs it is transformed and
    // becomes the button's offsetParent (offsetTop would read 0). The slide is horizontal only.
    const y = b.getBoundingClientRect().top - navEl.getBoundingClientRect().top;
    ind = { y: Math.round(y), h: b.offsetHeight, ready: ind.ready };
    // enable the transition only after the first placement, so it doesn't fly in from the top
    if (first) requestAnimationFrame(() => requestAnimationFrame(() => (ind.ready = true)));
  }

  $effect(() => {
    page;
    if (!navEl) return;
    tick().then(placeIndicator);
  });

  $effect(() => {
    if (!navEl) return;
    const ro = new ResizeObserver(() => placeIndicator());
    ro.observe(navEl);
    return () => ro.disconnect();
  });

  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  /** Replays the page entrance (used when the hidden window is shown again). */
  function replayEnter() {
    if (reduced.matches) return;
    const pg = content?.querySelector(".page");
    if (!pg) return;
    [...pg.children].forEach((el, i) =>
      (el as HTMLElement).animate(
        [
          { opacity: 0, transform: "translate3d(0, 12px, 0)" },
          { opacity: 1, transform: "none" },
        ],
        { duration: 440, delay: Math.min(i, 9) * 36, easing: "cubic-bezier(0.16, 1, 0.3, 1)", fill: "backwards" },
      ),
    );
  }

  onMount(() => {
    initApp().catch((e) => (loadError = String(e)));
    api.appInfo().then((i) => (version = i.version)).catch(() => {});
    const un = on<{ page: SettingsPage }>(EVENTS.navigate, (p) => p?.page && go(p.page));
    const unUpd = listenUpdates();
    let hiddenAt = 0;
    const vis = () => {
      if (document.visibilityState === "hidden") hiddenAt = Date.now();
      else if (hiddenAt && Date.now() - hiddenAt > 400) replayEnter();
    };
    document.addEventListener("visibilitychange", vis);
    return () => {
      un.then((f) => f());
      unUpd();
      document.removeEventListener("visibilitychange", vis);
    };
  });

  const Page = $derived(PAGES[page]);
</script>

<div class="shell">
  <ModelNotice />
  {#if app.settings && !app.settings.onboarded}
    <Onboarding />
  {:else}
    <WhatsNew />
    <aside class="rail">
      <div class="brand" data-tauri-drag-region>
        <span class="logo"><Logo size={24} glow={false} /></span>
        <span class="bname" data-tauri-drag-region>FlowVoice</span>
        <span class="bver mono" data-tauri-drag-region>{(version || "2.1").split(".").slice(0, 2).join(".")}</span>
      </div>

      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <nav class="nav" aria-label="FlowVoice" bind:this={navEl} onkeydown={navKey}>
        <span class="ind" class:ready={ind.ready} style:transform="translate3d(0, {ind.y}px, 0)" style:height="{ind.h}px" aria-hidden="true"></span>
        {#each GROUPS as g, gi}
          {#if g.key}
            <div class="grp" style:--i={gi * 3}><span class="label">{t(g.key)}</span></div>
          {:else}
            <div class="sep" aria-hidden="true"></div>
          {/if}
          <ul>
            {#each g.items as n (n.id)}
              <li style:--i={ORDER[n.id] + gi + 1}>
                <button
                  type="button"
                  data-id={n.id}
                  data-tip={t(n.key)}
                  aria-current={page === n.id ? "page" : undefined}
                  class:active={page === n.id}
                  onclick={() => go(n.id)}
                >
                  <span class="ico"><Icon name={n.icon} size={18} /></span>
                  <span class="lbl">{t(n.key)}</span>
                  {#if n.id === "providers" && provMarks.ids.length}
                    <span class="pmarks" title={provMarks.tip}>
                      {#each provMarks.ids as pid, k (pid)}
                        <span class="pm" style:--k={k}><ProviderLogo id={pid} size={14} /></span>
                      {/each}
                    </span>
                  {/if}
                  {#if n.id === "about" && updAvail}
                    <span class="nbadge" title={t("sh.update_tip", { v: upd.info?.version ?? "" })}></span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/each}
      </nav>

      <div class="me">
        <div class="donate" bind:this={donEl} role="group" aria-label={t("sh.donate")}>
          <span class="dhead">
            <span class="heart"><Icon name="heart" size={14} /></span>
            <span>{t("sh.donate")}</span>
          </span>
          <button
            type="button"
            class="dtoggle"
            data-tip={t("sh.donate")}
            aria-label={t("sh.donate_menu")}
            aria-expanded={donOpen}
            onclick={() => (donOpen = !donOpen)}
          >
            <span class="heart"><Icon name="heart" size={16} /></span>
          </button>
          <div class="dopts" class:open={donOpen}>
            {#each donateOptions() as d (d.id)}
              <button type="button" class="dopt" title={t(d.key)} onclick={() => donate(d.url)}>
                {#if d.id === "sponsors"}<Icon name="github" size={13} />{/if}
                <span class="short">{d.id === "pix" ? "Pix" : "Sponsors"}</span>
                <span class="full">{t(d.key)}</span>
              </button>
            {/each}
          </div>
        </div>
        <div class="verline">
          <span class="mono">v{version || "2.0"}</span>
          {#if updAvail}
            <button type="button" class="upd" onclick={() => go("about")} title={t("sh.update_tip", { v: upd.info?.version ?? "" })}>
              <Icon name="download" size={12} stroke={1.8} />
              <span>{t("sh.update_badge", { v: upd.info?.version ?? "" })}</span>
            </button>
          {/if}
        </div>
      </div>
    </aside>

    <div class="main" class:scrolled>
      <TitleBar>
        {#snippet left()}
          <SearchBar onpick={onPick} />
        {/snippet}
        {#snippet right()}
          <SaveIndicator />
        {/snippet}
      </TitleBar>

      <main class="content" bind:this={content} onscroll={(e) => (scrolled = (e.currentTarget as HTMLElement).scrollTop > 4)}>
        {#if loadError}
          <p class="err">{loadError}</p>
        {:else if app.settings}
          <div class="stage">
            {#key page}
              <div class="page stagger" style:--rise="{dir * 14}px" out:fade={{ duration: 110 }}>
                <Page onnavigate={go} />
              </div>
            {/key}
          </div>
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  .shell {
    height: 100%;
    display: flex;
    background: var(--win-tint, var(--bg));
    transition: background-color var(--t-slow) var(--ease);
  }

  /* ------------------------------------------------------------------ rail */
  .rail {
    width: 224px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--win-rail, var(--rail));
    border-right: 1px solid var(--line);
    padding: 0 12px 12px;
    position: relative;
    z-index: 6;
    animation: fv-slide-l var(--t-page) var(--ease-out) backwards;
  }
  .brand {
    height: 52px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 6px;
    margin-bottom: 6px;
  }
  .logo {
    display: block;
    border-radius: 7px;
    box-shadow: 0 0 0 1px var(--line), 0 4px 10px rgba(0, 0, 0, 0.25);
    transition: transform var(--t-slow) var(--spring);
  }
  .brand:hover .logo {
    transform: rotate(-6deg) scale(1.04);
  }
  .bname {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.025em;
  }
  .bver {
    font-size: 10px;
    color: var(--accent-fg);
    border: 1px solid var(--accent-line);
    border-radius: 4px;
    padding: 0 4px;
    line-height: 15px;
    transform: translateY(1px);
  }

  .nav {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .ind {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    border-radius: var(--r);
    background: var(--rail-active);
    box-shadow:
      inset 0 0 0 1px var(--line),
      0 1px 2px rgba(0, 0, 0, 0.18);
    pointer-events: none;
  }
  .ind::before {
    content: "";
    position: absolute;
    left: -12px;
    top: 50%;
    width: 3px;
    height: 16px;
    margin-top: -8px;
    border-radius: 0 3px 3px 0;
    background: var(--accent-fg);
  }
  .ind.ready {
    transition:
      transform 420ms var(--spring),
      height var(--t) var(--ease);
  }
  .grp {
    height: 30px;
    display: flex;
    align-items: flex-end;
    padding: 0 10px 7px;
    animation: fv-fade var(--t-page) var(--ease) backwards;
    animation-delay: calc(120ms + var(--i) * 30ms);
  }
  .grp:first-of-type {
    height: 22px;
  }
  .grp .label {
    font-size: 9.5px;
    letter-spacing: 0.16em;
  }
  .sep {
    height: 1px;
    margin: 12px 10px;
    background: var(--line);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    animation: fv-slide-l var(--t-page) var(--ease-out) backwards;
    animation-delay: calc(80ms + var(--i) * 28ms);
  }
  .nav button {
    position: relative;
    width: 100%;
    height: 36px;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 0 10px;
    border-radius: var(--r);
    font-size: 13.5px;
    color: var(--text-2);
    text-align: left;
    transition:
      color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease);
  }
  .nav button:hover:not(.active) {
    color: var(--text);
    background: var(--hover);
  }
  .nav button.active {
    color: var(--text);
    font-weight: 550;
  }
  .ico {
    display: grid;
    place-items: center;
    transition:
      transform var(--t) var(--spring),
      color var(--t) var(--ease);
  }
  .active .ico {
    color: var(--accent-fg);
  }
  .nav button:active .ico {
    transform: scale(0.86);
    transition-duration: 60ms;
  }
  .lbl {
    transition: transform var(--t) var(--ease);
    white-space: nowrap;
  }
  .nav button:hover:not(.active) .lbl {
    transform: translateX(2px);
  }
  .pmarks {
    margin-left: auto;
    display: flex;
    align-items: center;
    padding-right: 1px;
  }
  .pm {
    display: grid;
    border-radius: 5px;
    box-shadow: 0 0 0 1.5px var(--rail);
    animation: fv-pop var(--t-slow) var(--spring) backwards;
    animation-delay: calc(360ms + var(--k) * 60ms);
    transition: transform var(--t) var(--spring);
  }
  .pm + .pm {
    margin-left: -4px;
  }
  .nav button.active .pm {
    box-shadow: 0 0 0 1.5px var(--rail-active);
  }
  .nav button:hover .pm + .pm {
    transform: translate3d(3px, 0, 0);
  }
  .pm :global(svg) {
    border-radius: 4px;
  }
  .nbadge {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent-fg);
    box-shadow: 0 0 0 3px var(--accent-soft);
    animation: fv-pop var(--t-slow) var(--spring) backwards;
  }

  /* footer: support + version */
  .me {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
    animation: fv-rise var(--t-page) var(--ease-out) 260ms backwards;
  }
  .donate {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 9px 9px 9px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--surface);
    transition: border-color var(--t) var(--ease);
  }
  .donate:hover {
    border-color: var(--accent-line);
  }
  .dhead {
    display: flex;
    align-items: center;
    gap: 7px;
    padding-left: 1px;
    font-size: 12px;
    font-weight: 550;
  }
  .heart {
    display: grid;
    color: var(--accent-fg);
    transition: transform var(--t-slow) var(--spring);
  }
  .donate:hover .dhead .heart {
    transform: scale(1.2);
  }
  .dtoggle {
    display: none;
  }
  .dopts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 5px;
  }
  .dopt {
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 0 6px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    font-size: 11.5px;
    font-weight: 550;
    color: var(--text-2);
    white-space: nowrap;
    transition:
      color var(--t-fast) var(--ease),
      border-color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease),
      transform var(--t) var(--spring);
  }
  .dopt:first-child {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .dopt:hover {
    color: var(--text);
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .dopt:first-child:hover {
    color: var(--on-accent);
    background: var(--accent);
    transform: translateY(-1px);
  }
  .dopt:active {
    transform: scale(0.96);
    transition-duration: 60ms;
  }
  .dopt .full {
    display: none;
  }
  .verline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 20px;
    padding: 0 4px;
    font-size: 10.5px;
    color: var(--text-3);
  }
  .upd {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
    animation: fv-pop var(--t-slow) var(--spring) backwards;
    transition: transform var(--t) var(--spring);
  }
  .upd:hover {
    transform: translateY(-1px);
  }

  /* ------------------------------------------------------------------ main */
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .main::after {
    /* hairline under the title bar once the page scrolls */
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 40px;
    height: 1px;
    background: var(--line);
    opacity: 0;
    transition: opacity var(--t) var(--ease);
    pointer-events: none;
    z-index: 4;
  }
  .main.scrolled::after {
    opacity: 1;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
    container: content / inline-size;
  }
  .stage {
    position: relative;
    display: grid;
  }
  .page {
    grid-area: 1 / 1;
    width: 100%;
    /* settings read as a column: past ~1000px rows turn into label … far-away control */
    max-width: 1000px;
    margin: 0 auto;
    padding: 12px clamp(20px, 3.2cqi, 40px) 72px;
    min-width: 0;
  }
  .err {
    padding: 40px;
    color: var(--danger-fg);
    font-family: var(--mono);
  }

  /* ------------------------------------------------- compact (icon-only) rail */
  @media (max-width: 840px) {
    .rail {
      width: 64px;
      padding: 0 10px 12px;
    }
    .brand {
      justify-content: center;
      padding: 0;
    }
    .bname,
    .bver,
    .lbl,
    .grp .label,
    .pmarks,
    .verline {
      display: none;
    }
    .grp,
    .grp:first-of-type {
      height: 13px;
      padding: 0;
    }
    .grp:first-of-type {
      height: 4px;
    }
    .grp + ul {
      position: relative;
    }
    .grp:not(:first-of-type)::before {
      content: "";
      display: block;
      width: 18px;
      height: 1px;
      margin: 6px auto 0;
      background: var(--line-strong);
    }
    .sep {
      margin: 6px 12px;
    }
    .nav button {
      justify-content: center;
      padding: 0;
    }
    .ind::before {
      left: -10px;
    }
    .nbadge {
      position: absolute;
      top: 7px;
      right: 8px;
      width: 6px;
      height: 6px;
      box-shadow: 0 0 0 2px var(--rail);
    }
    .donate {
      padding: 0;
      border: 0;
      background: none;
    }
    .dhead {
      display: none;
    }
    .dtoggle {
      position: relative;
      height: 34px;
      display: grid;
      place-items: center;
      border-radius: var(--r);
      border: 1px solid var(--line-strong);
      transition:
        border-color var(--t-fast) var(--ease),
        background-color var(--t-fast) var(--ease);
    }
    .dtoggle:hover,
    .dtoggle[aria-expanded="true"] {
      border-color: var(--accent-line);
      background: var(--accent-soft);
    }
    .dtoggle:hover .heart {
      transform: scale(1.18);
    }
    .dopts {
      display: none;
      position: absolute;
      left: calc(100% + 12px);
      bottom: 0;
      z-index: 60;
      width: 190px;
      grid-template-columns: 1fr;
      padding: 6px;
      border-radius: var(--r-lg);
      background: var(--surface);
      box-shadow: var(--shadow-pop);
      transform-origin: 0 100%;
    }
    .dopts.open {
      display: grid;
      animation: fv-pop var(--t) var(--spring) backwards;
    }
    .dopt {
      height: 32px;
      justify-content: flex-start;
      padding: 0 10px;
      font-size: 12.5px;
    }
    .dopt .short {
      display: none;
    }
    .dopt .full {
      display: inline;
    }
    /* tooltips for the icon-only rail */
    .nav button::after,
    .dtoggle::after {
      content: attr(data-tip);
      position: absolute;
      left: calc(100% + 12px);
      top: 50%;
      z-index: 50;
      padding: 5px 9px;
      border-radius: var(--r-sm);
      background: var(--text);
      color: var(--bg);
      font-size: 12px;
      font-weight: 500;
      line-height: 1.2;
      white-space: nowrap;
      pointer-events: none;
      opacity: 0;
      transform: translate3d(-4px, -50%, 0);
      transition:
        opacity var(--t-fast) var(--ease),
        transform var(--t-fast) var(--ease);
    }
    .nav button:hover::after,
    .nav button:focus-visible::after,
    .dtoggle:not([aria-expanded="true"]):hover::after,
    .dtoggle:not([aria-expanded="true"]):focus-visible::after {
      opacity: 1;
      transform: translate3d(0, -50%, 0);
      transition-delay: 220ms;
    }
    .nav button:active::after {
      opacity: 0;
      transition-delay: 0ms;
    }
  }
</style>
