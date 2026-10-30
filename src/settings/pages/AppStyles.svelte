<script lang="ts">
  // "Por aplicativo": per-app style rules (Settings → Estilos). A rule overrides the active style
  // for dictations into that app only; Rust matches them (src-tauri/src/apps.rs).
  import { onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "svelte/motion";
  import { fly } from "svelte/transition";
  import { api, on } from "../../lib/api";
  import { appHue, appMonogram, appName, normalizeApp, recentApps, suggestions } from "../../lib/apps";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import { EVENTS, type DictationResult } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import Modal from "../../lib/ui/Modal.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";

  const s = $derived(app.settings!);

  let history = $state<DictationResult[]>([]);
  onMount(() => {
    const load = () => api.getHistory().then((h) => (history = h));
    load();
    const un = on(EVENTS.historyChanged, load);
    return () => un.then((f) => f());
  });

  const recent = $derived(recentApps(history));
  const sugg = $derived(suggestions(s, recent.map((r) => r.app), 6));
  const styleOptions = $derived(s.styles.map((st) => ({ value: st.id, label: st.name })));
  const styleName = (id: string) => s.styles.find((x) => x.id === id)?.name ?? id;
  /** Shown under the name only when it says something the name doesn't ("code" under "VS Code"). */
  const idHint = (id: string) => (appName(id).toLowerCase() === id ? "" : id);

  function upsert(id: string, style_id: string) {
    const existing = s.app_styles.find((r) => r.app === id);
    if (existing) existing.style_id = style_id;
    else s.app_styles.push({ app: id, style_id });
  }
  function remove(id: string) {
    s.app_styles = s.app_styles.filter((r) => r.app !== id);
  }

  // ------------------------------------------------------------ add modal
  let addOpen = $state(false);
  let manual = $state("");
  let picked = $state("");
  let draftStyle = $state("");
  const draftApp = $derived(picked || normalizeApp(manual));
  const exists = $derived(!!draftApp && s.app_styles.some((r) => r.app === draftApp));
  const recentFree = $derived(recent.filter((r) => !s.app_styles.some((x) => x.app === r.app)).slice(0, 8));

  function defaultStyleFor(id: string) {
    return suggestions({ ...s, app_styles: [] }, [], 99).find((x) => x.app === id)?.style_id ?? s.active_style;
  }
  function openAdd() {
    manual = "";
    picked = "";
    draftStyle = s.active_style;
    addOpen = true;
  }
  function pick(id: string) {
    picked = picked === id ? "" : id;
    manual = "";
    if (picked) draftStyle = defaultStyleFor(picked);
  }
  function onManual() {
    picked = "";
    const id = normalizeApp(manual);
    if (id) draftStyle = s.app_styles.find((r) => r.app === id)?.style_id ?? defaultStyleFor(id);
  }
  function confirmAdd() {
    if (!draftApp) return;
    upsert(draftApp, draftStyle || s.active_style);
    addOpen = false;
  }

  const dur = () => (prefersReducedMotion.current ? 0 : 220);
  function collapse(node: HTMLElement) {
    const h = node.offsetHeight;
    return {
      duration: prefersReducedMotion.current ? 0 : 200,
      easing: cubicOut,
      css: (k: number) => `opacity:${k};height:${k * h}px;overflow:hidden;transform:translateX(${(1 - k) * 10}px)`,
    };
  }
</script>

{#snippet tile(id: string, small = false)}
  <span class="tile" class:small style="--h:{appHue(id)}" aria-hidden="true">{appMonogram(id)}</span>
{/snippet}

<Section label={t("apps.sec")}>
  {#snippet aside()}
    <Button size="sm" icon="plus" onclick={openAdd}>{t("apps.add")}</Button>
  {/snippet}
  <Row id="set-sty-apps" label={t("apps.enable")} hint={t("apps.enable_hint")}>
    <Toggle label={t("apps.enable")} bind:checked={s.app_styles_enabled} />
  </Row>

  <div class="body" class:off={!s.app_styles_enabled}>
    {#if !s.app_styles_enabled}
      <p class="offnote faint" in:fly={{ y: -4, duration: dur() }}><Icon name="info" size={13} />{t("apps.off_note")}</p>
    {/if}
    {#if s.app_styles.length}
      <ul class="rules">
        {#each s.app_styles as r, i (r.app)}
          <li in:fly={{ y: 8, duration: dur(), delay: prefersReducedMotion.current ? 0 : Math.min(i, 6) * 30, easing: cubicOut }} out:collapse>
            {@render tile(r.app)}
            <span class="an">
              <span class="nm">{appName(r.app)}</span>
              {#if idHint(r.app)}<span class="aid mono">{idHint(r.app)}</span>{/if}
            </span>
            <span class="arrow" aria-hidden="true"><Icon name="arrowRight" size={14} /></span>
            <span class="sel">
              <Select label={t("apps.style_for", { app: appName(r.app) })} bind:value={r.style_id} options={styleOptions} />
            </span>
            <span class="rm">
              <Button size="sm" variant="ghost" icon="x" aria-label={t("apps.remove", { app: appName(r.app) })} title={t("apps.remove", { app: appName(r.app) })} onclick={() => remove(r.app)} />
            </span>
          </li>
        {/each}
      </ul>
    {:else}
      <button type="button" class="empty" onclick={openAdd}>
        <span class="plus-box" aria-hidden="true"><Icon name="plus" size={18} /></span>
        <span>
          <span class="empty-t">{t("apps.empty_t")}</span>
          <span class="empty-d">{t("apps.empty_d")}</span>
        </span>
      </button>
    {/if}

    {#if sugg.length}
      <div class="sugg">
        <span class="slabel">{t("apps.suggested")}</span>
        <div class="schips">
          {#each sugg as x (x.app)}
            <button
              type="button"
              class="schip"
              title={t("apps.suggest_tip", { style: styleName(x.style_id), app: appName(x.app) })}
              onclick={() => upsert(x.app, x.style_id)}
              out:fly={{ y: -4, duration: prefersReducedMotion.current ? 0 : 140 }}
            >
              <Icon name="plus" size={12} />
              <span class="sn">{appName(x.app)}</span>
              <span class="sa" aria-hidden="true">→</span>
              <span class="ss">{styleName(x.style_id)}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</Section>

<Modal bind:open={addOpen} title={t("apps.add_title")} width="500px">
  <form
    class="form"
    id="app-style-form"
    onsubmit={(e) => {
      e.preventDefault();
      confirmAdd();
    }}
  >
    <div class="group">
      <span class="label">{t("apps.recent")}</span>
      {#if recentFree.length}
        <div class="pchips" role="listbox" aria-label={t("apps.recent")}>
          {#each recentFree as a (a.app)}
            <button type="button" class="pchip" role="option" aria-selected={picked === a.app} class:sel={picked === a.app} onclick={() => pick(a.app)}>
              {@render tile(a.app, true)}
              <span>{a.name}</span>
              <span class="cnt mono">{t("apps.times", { n: a.count })}</span>
            </button>
          {/each}
        </div>
      {:else}
        <p class="none faint">{t("apps.recent_none")}</p>
      {/if}
    </div>

    <label>
      <span class="label">{t("apps.manual")}</span>
      <input class="field" bind:value={manual} oninput={onManual} placeholder={t("apps.manual_ph")} spellcheck="false" autocomplete="off" maxlength="80" />
      {#if manual.trim() && draftApp}
        <span class="preview">
          {@render tile(draftApp, true)}
          <span class="pn">{appName(draftApp)}</span>
          {#if idHint(draftApp)}<span class="aid mono">{draftApp}</span>{/if}
        </span>
      {:else}
        <span class="fhint">{t("apps.manual_hint")}</span>
      {/if}
    </label>

    <div class="group">
      <span class="label">{t("apps.style")}</span>
      <Select label={t("apps.style")} bind:value={draftStyle} options={styleOptions} />
    </div>
    {#if exists}
      <p class="exists"><Icon name="info" size={14} />{t("apps.exists", { app: appName(draftApp) })}</p>
    {/if}
  </form>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (addOpen = false)}>{t("c.cancel")}</Button>
    <Button variant="primary" type="submit" form="app-style-form" icon="check" disabled={!draftApp}>{exists ? t("apps.replace_btn") : t("apps.add_btn")}</Button>
  {/snippet}
</Modal>

<style>
  .body {
    transition: opacity var(--t) var(--ease);
  }
  .body.off .rules,
  .body.off .sugg,
  .body.off .empty {
    opacity: 0.5;
  }
  .offnote {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 2px 0 10px;
    font-size: 12px;
  }

  /* ------------------------------------------------------------ rules */
  .rules {
    list-style: none;
    margin: 0 -10px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto minmax(150px, 200px) auto;
    align-items: center;
    column-gap: 12px;
    padding: 7px 6px 7px 10px;
    border-radius: var(--r);
    transition: background var(--t) var(--ease);
  }
  li:hover,
  li:focus-within {
    background: var(--hover);
  }
  .tile {
    --tint: oklch(0.72 0.13 var(--h));
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 8px;
    font: 600 11px var(--mono);
    letter-spacing: 0.02em;
    color: color-mix(in oklab, var(--tint) 75%, var(--text));
    background: color-mix(in oklab, var(--tint) 14%, var(--surface-2));
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--tint) 22%, transparent);
  }
  .tile.small {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    font-size: 8.5px;
  }
  .an {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .nm {
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .aid {
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .arrow {
    color: var(--text-3);
    display: grid;
    transition:
      color var(--t-fast),
      transform var(--t) var(--ease);
  }
  li:hover .arrow {
    color: var(--accent-fg);
    transform: translateX(2px);
  }
  .sel {
    min-width: 0;
    display: grid;
  }
  .sel :global(button) {
    width: 100%;
  }
  .rm {
    opacity: 0.55;
    transition: opacity var(--t-fast);
  }
  li:hover .rm,
  li:focus-within .rm {
    opacity: 1;
  }

  /* ------------------------------------------------------------ empty */
  .empty {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px;
    text-align: left;
    border: 1px dashed var(--line-strong);
    border-radius: var(--r);
    transition:
      border-color var(--t-fast),
      background var(--t-fast);
  }
  .empty:hover {
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .plus-box {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--r);
    background: var(--surface-2);
    color: var(--accent-fg);
    transition: transform var(--t) var(--spring);
  }
  .empty:hover .plus-box {
    transform: rotate(90deg);
  }
  .empty-t {
    display: block;
    font-size: 13.5px;
    font-weight: 500;
  }
  .empty-d {
    display: block;
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  /* ------------------------------------------------------------ suggestions */
  .sugg {
    display: flex;
    align-items: baseline;
    gap: 8px 12px;
    margin-top: 12px;
    flex-wrap: wrap;
  }
  .slabel {
    font: 500 10.5px var(--mono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-3);
    flex: none;
  }
  .schips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }
  .schip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px 0 9px;
    border-radius: 14px;
    border: 1px dashed var(--line-strong);
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
    transition:
      color var(--t-fast),
      border-color var(--t-fast),
      background var(--t-fast),
      transform var(--t-fast) var(--ease);
  }
  .schip :global(.icon) {
    color: var(--text-3);
    transition:
      color var(--t-fast),
      transform var(--t) var(--spring);
  }
  .schip:hover {
    color: var(--text);
    border-style: solid;
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .schip:hover :global(.icon) {
    color: var(--accent-fg);
    transform: rotate(90deg);
  }
  .schip:active {
    transform: scale(0.96);
  }
  .sn {
    color: var(--text);
    font-weight: 500;
  }
  .sa {
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ modal */
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .group,
  label {
    display: flex;
    flex-direction: column;
    gap: 7px;
    min-width: 0;
  }
  .pchips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pchip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 10px 0 6px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    font-size: 12.5px;
    color: var(--text);
    transition:
      border-color var(--t-fast),
      background var(--t-fast),
      transform var(--t-fast) var(--ease);
  }
  .pchip:hover {
    border-color: var(--text-3);
  }
  .pchip:active {
    transform: scale(0.97);
  }
  .pchip.sel {
    border-color: var(--accent-line);
    background: var(--accent-soft);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .cnt {
    font-size: 10.5px;
    color: var(--text-3);
  }
  .none,
  .fhint {
    font-size: 12px;
    color: var(--text-3);
  }
  .preview {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    animation: pv-in 200ms var(--ease) both;
  }
  .pn {
    font-weight: 500;
  }
  @keyframes pv-in {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
  .exists {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: -4px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .exists :global(.icon) {
    color: var(--accent-fg);
    flex: none;
  }

  @media (max-width: 700px) {
    li {
      grid-template-columns: auto minmax(0, 1fr) minmax(130px, 170px) auto;
    }
    .arrow {
      display: none;
    }
    .aid {
      display: none;
    }
  }
</style>
