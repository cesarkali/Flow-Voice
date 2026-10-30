<script lang="ts">
  import { onMount } from "svelte";
  import { on } from "../lib/api";
  import { t, type TKey } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import Logo from "../lib/icons/Logo.svelte";
  import type { IconName } from "../lib/icons/paths";
  import { app, initApp } from "../lib/stores.svelte";
  import { EVENTS, type SettingsPage } from "../lib/types";
  import SaveIndicator from "../lib/ui/SaveIndicator.svelte";
  import TitleBar from "../lib/ui/TitleBar.svelte";
  import Onboarding from "./onboarding/Onboarding.svelte";
  import About from "./pages/About.svelte";
  import General from "./pages/General.svelte";
  import Home from "./pages/Home.svelte";
  import Providers from "./pages/Providers.svelte";
  import Shortcuts from "./pages/Shortcuts.svelte";
  import Speech from "./pages/Speech.svelte";
  import Styles from "./pages/Styles.svelte";

  const NAV: { id: SettingsPage; icon: IconName; key: TKey }[] = [
    { id: "home", icon: "home", key: "nav.home" },
    { id: "styles", icon: "styles", key: "nav.styles" },
    { id: "providers", icon: "key", key: "nav.providers" },
    { id: "speech", icon: "wave", key: "nav.speech" },
    { id: "shortcuts", icon: "keyboard", key: "nav.shortcuts" },
    { id: "general", icon: "sliders", key: "nav.general" },
    { id: "about", icon: "info", key: "nav.about" },
  ];
  const PAGES = { home: Home, styles: Styles, providers: Providers, speech: Speech, shortcuts: Shortcuts, general: General, about: About };

  const initial = new URLSearchParams(location.search).get("page") as SettingsPage | null;
  let page = $state<SettingsPage>(initial && initial in PAGES ? initial : "home");
  let content: HTMLElement | undefined = $state();
  let loadError = $state("");

  function go(p: SettingsPage) {
    if (p === page) return;
    page = p;
    content?.scrollTo({ top: 0 });
  }

  function railKey(e: KeyboardEvent) {
    const i = NAV.findIndex((n) => n.id === page);
    let j = -1;
    if (e.key === "ArrowDown") j = (i + 1) % NAV.length;
    if (e.key === "ArrowUp") j = (i - 1 + NAV.length) % NAV.length;
    if (j < 0) return;
    e.preventDefault();
    go(NAV[j].id);
    (e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("button")[j]?.focus();
  }

  onMount(() => {
    initApp().catch((e) => (loadError = String(e)));
    const un = on<{ page: SettingsPage }>(EVENTS.navigate, (p) => p?.page && go(p.page));
    return () => {
      un.then((f) => f());
    };
  });

  const Page = $derived(PAGES[page]);
</script>

<div class="shell">
  {#if app.settings && !app.settings.onboarded}
    <Onboarding />
  {:else}
    <TitleBar>
      {#snippet left()}
        <span class="brand" data-tauri-drag-region>
          <Logo size={18} glow={false} />
          <span data-tauri-drag-region>FlowVoice</span>
        </span>
      {/snippet}
      {#snippet right()}
        <SaveIndicator />
      {/snippet}
    </TitleBar>

    <div class="body">
      <nav class="rail" aria-label="FlowVoice">
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <ul role="tablist" aria-orientation="vertical" onkeydown={railKey}>
          {#each NAV as n (n.id)}
            <li>
              <button
                type="button"
                role="tab"
                aria-selected={page === n.id}
                tabindex={page === n.id ? 0 : -1}
                class:active={page === n.id}
                title={t(n.key)}
                onclick={() => go(n.id)}
              >
                <Icon name={n.icon} size={18} />
                <span>{t(n.key)}</span>
              </button>
            </li>
          {/each}
        </ul>
        <div class="rail-foot mono">v2.0</div>
      </nav>

      <main class="content" bind:this={content}>
        {#if loadError}
          <p class="err">{loadError}</p>
        {:else if app.settings}
          {#key page}
            <div class="page" role="tabpanel">
              <Page onnavigate={go} />
            </div>
          {/key}
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  .shell {
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
    letter-spacing: -0.01em;
    color: var(--text);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .rail {
    width: 188px;
    flex: none;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 10px 10px 14px;
    border-right: 1px solid var(--line);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rail button {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 11px;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--r);
    font-size: 13.5px;
    color: var(--text-2);
    text-align: left;
  }
  .rail button:hover {
    color: var(--text);
    background: var(--hover);
  }
  .rail button.active {
    color: var(--text);
    background: var(--surface-2);
  }
  .rail button.active::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 9px;
    bottom: 9px;
    width: 2px;
    border-radius: 0 2px 2px 0;
    background: var(--accent-fg);
  }
  .rail button.active :global(.icon) {
    color: var(--accent-fg);
  }
  .rail-foot {
    padding: 0 10px;
    font-size: 10.5px;
    color: var(--text-3);
    letter-spacing: 0.08em;
  }
  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
  }
  .page {
    max-width: 680px;
    margin: 0 auto;
    padding: 22px 36px 64px;
    animation: enter 200ms var(--ease) both;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  .err {
    padding: 40px;
    color: var(--danger-fg);
    font-family: var(--mono);
  }
  @media (max-width: 840px) {
    .rail {
      width: 64px;
      padding: 10px 8px 14px;
    }
    .rail button {
      justify-content: center;
      padding: 0;
    }
    .rail button span {
      display: none;
    }
    .rail button.active::before {
      left: -8px;
    }
    .rail-foot {
      display: none;
    }
    .page {
      padding: 20px 28px 56px;
    }
  }
</style>
