<script lang="ts">
  import { onMount } from "svelte";
  import { api, on } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import { EVENTS, type AppInfo, type SettingsPage, type UpdateInfo } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import Progress from "../../lib/ui/Progress.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import AccentPicker from "../../lib/ui/AccentPicker.svelte";
  import LangPicker from "../../lib/ui/LangPicker.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);


  let info = $state<AppInfo | null>(null);
  let upd = $state<UpdateInfo | null>(null);
  let checking = $state(false);
  let updErr = $state("");
  let installing = $state(false);
  let dl = $state<{ downloaded: number; total: number | null } | null>(null);

  onMount(() => {
    api.appInfo().then((i) => (info = i));
    const un = on<{ downloaded: number; total: number | null }>(EVENTS.updateProgress, (p) => (dl = p));
    return () => un.then((f) => f());
  });

  async function check() {
    checking = true;
    updErr = "";
    try {
      upd = await api.checkUpdate();
    } catch (e) {
      updErr = String(e);
    } finally {
      checking = false;
    }
  }
  async function install() {
    installing = true;
    try {
      await api.installUpdate();
    } catch (e) {
      updErr = String(e);
      installing = false;
    }
  }
  const dlPct = $derived(dl && dl.total ? dl.downloaded / dl.total : 0);
</script>

<header class="ph">
  <h1>{t("gen.title")}</h1>
</header>

<Section label={t("gen.sec_look")}>
  <Row label={t("gen.name")} hint={t("gen.name_hint")}>
    <input
      class="name-input"
      type="text"
      maxlength="40"
      autocomplete="given-name"
      placeholder={t("gen.name_ph")}
      aria-label={t("gen.name")}
      bind:value={s.user_name}
    />
  </Row>
  <Row label={t("gen.accent")} hint={t("gen.accent_hint")}>
    <AccentPicker bind:value={s.accent_color} label={t("gen.accent")} customLabel={t("gen.accent_custom")} />
  </Row>
  <Row label={t("gen.ui_language")}>
    <div class="langp"><LangPicker bind:value={s.ui_language} label={t("gen.ui_language")} /></div>
  </Row>
  <Row label={t("gen.theme")} hint={t("ctl.theme_hint")}>
    <Segmented
      label={t("gen.theme")}
      bind:value={s.theme}
      options={[
        { value: "system", label: t("gen.theme_system"), icon: "monitor" },
        { value: "dark", label: t("gen.theme_dark"), icon: "moon" },
        { value: "light", label: t("gen.theme_light"), icon: "sun" },
      ]}
    />
  </Row>
</Section>

<Section label={t("ctl.sec_overlay")}>
  <!-- live miniature: two monitors, the capsule moves to where it would appear -->
  <div
    class="mini"
    aria-hidden="true"
    class:top={s.overlay_position === "top"}
    class:follow={s.overlay_follow_mouse}
    data-w={s.overlay_width}
  >
    <div class="scr main">
      <span class="cap"><i></i><i></i><i></i></span>
      <span class="tag mono">1</span>
    </div>
    <div class="scr second">
      <span class="cap"><i></i><i></i><i></i></span>
      <span class="tag mono">2</span>
      <svg class="mouse" viewBox="0 0 12 16" width="12" height="16"><path d="M1 1l9 8.5H5.6L3.4 14z" /></svg>
    </div>
  </div>
  <Row label={t("gen.overlay_position")} hint={t("ctl.ov_pos_hint")}>
    <Segmented
      label={t("gen.overlay_position")}
      bind:value={s.overlay_position}
      options={[
        { value: "bottom", label: t("gen.pos_bottom"), icon: "arrowDown" },
        { value: "top", label: t("gen.pos_top"), icon: "arrowUp" },
      ]}
    />
  </Row>
  <Row label={t("ctl.ov_width")} hint={t("ctl.ov_width_hint")}>
    <Segmented
      label={t("ctl.ov_width")}
      bind:value={s.overlay_width}
      options={[
        { value: "compact", label: t("ctl.w_compact") },
        { value: "normal", label: t("ctl.w_normal") },
        { value: "wide", label: t("ctl.w_wide") },
      ]}
    />
  </Row>
  <Row label={t("ctl.ov_follow")} hint={t("ctl.ov_follow_hint")}>
    <Toggle label={t("ctl.ov_follow")} bind:checked={s.overlay_follow_mouse} />
  </Row>
</Section>

<Section label={t("gen.sec_behavior")}>
  <Row label={t("gen.auto_paste")} hint={t("gen.auto_paste_hint")}>
    <Toggle label={t("gen.auto_paste")} bind:checked={s.auto_paste} />
  </Row>
  <Row label={t("gen.mute")} hint={t("ctl.mute_hint")}>
    <Toggle label={t("gen.mute")} bind:checked={s.mute_on_record} />
  </Row>
  <Row label={t("gen.sounds")} hint={t("ctl.sounds_hint")}>
    <Toggle label={t("gen.sounds")} bind:checked={s.sounds} />
  </Row>
  <Row label={t("gen.start_os")} hint={t("gen.start_os_hint")}>
    <Toggle label={t("gen.start_os")} bind:checked={s.start_with_os} />
  </Row>
  <Row label={t("gen.history")} hint={t("gen.history_hint")}>
    <Toggle label={t("gen.history")} bind:checked={s.history_enabled} />
  </Row>
</Section>

<Section label={t("gen.sec_updates")}>
  <div class="upd">
    <div class="uinfo">
      <span class="mono ver">{t("gen.current", { v: info?.version ?? "…" })}</span>
      {#if updErr}
        <span class="err">{t("gen.update_error", { e: updErr })}</span>
      {:else if upd && !upd.available}
        <span class="ok"><Icon name="check" size={14} stroke={2} />{t("gen.up_to_date")}</span>
      {:else if upd?.available}
        <span class="new">{t("gen.available", { v: upd.version ?? "" })}</span>
        {#if upd.notes}<p class="notes">{upd.notes}</p>{/if}
      {/if}
      {#if installing && dl}
        <div class="pg">
          <Progress value={dlPct} label={t("gen.install")} />
          <span class="mono faint tnum">{t("gen.installing", { pct: Math.round(dlPct * 100) })}</span>
        </div>
      {/if}
    </div>
    {#if upd?.available}
      <Button variant="primary" icon="download" loading={installing} onclick={install}>{t("gen.install")}</Button>
    {:else}
      <Button icon="refresh" loading={checking} onclick={check}>{checking ? t("gen.checking") : t("gen.check")}</Button>
    {/if}
  </div>
</Section>

<style>
  .ph h1 {
    font-size: 26px;
  }

  /* ---------------------------------------------------------------- overlay miniature */
  .mini {
    --spr: var(--spring);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 10px;
    padding: 16px 12px 14px;
    margin-bottom: 14px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background:
      radial-gradient(circle at 1px 1px, var(--line) 1px, transparent 0) 0 0 / 12px 12px,
      var(--surface);
  }
  @supports (transition-timing-function: linear(0, 1)) {
    .mini {
      --spr: linear(0, 0.063, 0.206, 0.379, 0.549, 0.698, 0.818, 0.907, 0.968, 1.007, 1.028, 1.037, 1.038, 1.034, 1.028, 1.021, 1.015, 1.01, 1.006, 1.003, 1);
    }
  }
  .scr {
    position: relative;
    width: 168px;
    aspect-ratio: 16 / 10;
    border-radius: 4px;
    border: 1.5px solid var(--line-strong);
    background: var(--bg);
  }
  .scr.second {
    width: 128px;
  }
  .tag {
    position: absolute;
    left: 5px;
    bottom: 2px;
    font-size: 9px;
    color: var(--text-3);
  }
  .cap {
    position: absolute;
    left: 50%;
    bottom: 12px;
    height: 10px;
    width: 46%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1.5px;
    border-radius: 6px;
    background: var(--accent);
    translate: -50% 0;
    transition:
      bottom 420ms var(--spr),
      width 420ms var(--spr),
      opacity 220ms var(--ease),
      scale 320ms var(--spr);
  }
  .cap i {
    width: 2px;
    height: 4px;
    border-radius: 1px;
    background: var(--on-accent);
  }
  .cap i:nth-child(2) {
    height: 6px;
  }
  .top .cap {
    bottom: calc(100% - 18px);
  }
  [data-w="compact"] .cap {
    width: 32%;
  }
  [data-w="wide"] .cap {
    width: 70%;
  }
  /* follow mouse: capsule shows on the second screen (where the pointer is) */
  .second .cap,
  .follow .main .cap {
    opacity: 0;
    scale: 0.6;
  }
  .follow .second .cap {
    opacity: 1;
    scale: 1;
  }
  .mouse {
    position: absolute;
    right: 22%;
    top: 46%;
    fill: var(--text);
    stroke: var(--bg);
    stroke-width: 1;
    opacity: 0.35;
    transition: opacity var(--t) var(--ease);
  }
  .follow .mouse {
    opacity: 1;
  }

  .upd {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    padding: 14px 0;
    border-top: 1px solid var(--line);
  }
  .uinfo {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
    font-size: 13px;
  }
  .ver {
    font-size: 12px;
    color: var(--text-2);
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--accent-fg);
  }
  .new {
    color: var(--accent-fg);
    font-weight: 600;
  }
  .notes {
    color: var(--text-2);
    font-size: 12.5px;
  }
  .err {
    color: var(--danger-fg);
    font-size: 12.5px;
  }
  .pg {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    max-width: 320px;
    font-size: 11px;
  }
  .name-input {
    width: 240px;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: var(--r);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    transition: border-color var(--t-fast) var(--ease), box-shadow var(--t-fast) var(--ease);
  }
  .name-input:focus {
    outline: none;
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .langp {
    width: 360px;
    max-width: 100%;
  }
</style>
