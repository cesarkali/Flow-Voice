<script lang="ts">
  import { onMount } from "svelte";
  import { api, on } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app, restartOnboarding } from "../../lib/stores.svelte";
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

  /** Windows' blur can show now (transparency effects on, no energy saver). */
  let glassOk = $state<boolean | null>(null);
  $effect(() => {
    if (s.overlay_glass !== "solid") api.glassAvailable().then((v) => (glassOk = v)).catch(() => (glassOk = null));
  });

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
  let redoing = $state(false);
  async function redo() {
    redoing = true;
    await restartOnboarding();
    redoing = false;
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
  <!-- live miniature: the pointer focuses a window on each screen, the capsule shows where it would appear -->
  <div
    class="mini"
    aria-hidden="true"
    class:top={s.overlay_position === "top"}
    class:follow={s.overlay_follow_mouse}
    data-w={s.overlay_width}
  >
    <div class="desk">
      <div class="mon">
        <div class="scr main">
          <span class="win a"><b></b></span>
          <span class="cap"><i></i><i></i><i></i></span>
        </div>
        <span class="stand"></span>
        <span class="tag"><span class="star"></span>{t("ctl.mon_main")}</span>
      </div>
      <div class="mon">
        <div class="scr second">
          <span class="win b"><b></b></span>
          <span class="cap"><i></i><i></i><i></i></span>
        </div>
        <span class="stand"></span>
        <span class="tag">{t("ctl.mon_other")}</span>
      </div>
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
  <Row label={t("ctl.ov_glass")} hint={t("ctl.ov_glass_hint")}>
    <Segmented
      label={t("ctl.ov_glass")}
      bind:value={s.overlay_glass}
      options={[
        { value: "solid", label: t("ctl.glass_solid") },
        { value: "glass", label: t("ctl.glass_glass") },
        { value: "translucent", label: t("ctl.glass_translucent") },
      ]}
    />
  </Row>
  {#if s.overlay_glass !== "solid"}
    <Row label={t("ctl.ov_glass_light")} hint={t("ctl.ov_glass_light_hint")}>
      <Toggle label={t("ctl.ov_glass_light")} bind:checked={s.overlay_glass_light} />
    </Row>
    {#if glassOk === false}
      <p class="gnote">{t("ctl.glass_off_note")}</p>
    {/if}
  {/if}
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
  <Row label={t("obx.redo")} hint={t("obx.redo_hint")}>
    <Button size="sm" icon="refresh" loading={redoing} onclick={redo}>{t("obx.redo_btn")}</Button>
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
  /* One 7 s loop: the pointer clicks a window on "Principal" (dictation: capsule there), then
     hops to the other screen and clicks a window there (capsule on that screen when the toggle
     is on, back on "Principal" when it is off). Only opacity/transform animate. */
  .mini {
    --spr: var(--spring);
    --loop: 7s;
    --mw: 168px; /* main screen: 168 x 105 */
    --ow: 128px; /* other screen: 128 x 80 */
    --gap: 22px;
    display: flex;
    justify-content: center;
    padding: 18px 12px 12px;
    margin-bottom: 14px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background:
      radial-gradient(circle at 1px 1px, var(--line) 1px, transparent 0) 0 0 / 12px 12px,
      var(--surface);
    overflow: hidden;
  }
  @supports (transition-timing-function: linear(0, 1)) {
    .mini {
      --spr: linear(0, 0.063, 0.206, 0.379, 0.549, 0.698, 0.818, 0.907, 0.968, 1.007, 1.028, 1.037, 1.038, 1.034, 1.028, 1.021, 1.015, 1.01, 1.006, 1.003, 1);
    }
  }
  .desk {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: var(--gap);
  }
  .mon {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .scr {
    position: relative;
    width: var(--mw);
    aspect-ratio: 16 / 10;
    border-radius: 5px;
    border: 1.5px solid var(--line-strong);
    background: var(--bg);
    overflow: hidden;
  }
  .scr.second {
    width: var(--ow);
  }
  .stand {
    width: 24px;
    height: 7px;
    background: var(--line-strong);
    clip-path: polygon(25% 0, 75% 0, 100% 100%, 0 100%);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 7px;
    font-size: 11px;
    font-weight: 550;
    color: var(--text-2);
    white-space: nowrap;
  }
  .star {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent-fg);
  }

  /* a window on each screen; the focused one gets an accent ring */
  .win {
    position: absolute;
    top: 25%;
    height: 45%;
    border-radius: 3px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .win.a {
    left: 14%;
    width: 54%;
  }
  .win.b {
    left: 18%;
    width: 64%;
  }
  .win::before {
    /* title bar */
    content: "";
    position: absolute;
    inset: 0 0 auto;
    height: 5px;
    border-radius: 3px 3px 0 0;
    background: var(--line-strong);
  }
  .win b {
    position: absolute;
    inset: -1px;
    border-radius: 4px;
    box-shadow:
      0 0 0 1.5px var(--accent-fg),
      0 0 0 4px var(--accent-soft);
    opacity: 0;
  }
  .win.a b {
    animation: focus-a var(--loop) var(--ease) infinite;
  }
  .win.b b {
    animation: focus-b var(--loop) var(--ease) infinite;
  }
  @keyframes focus-a {
    0%, 11% { opacity: 0; }
    13%, 58% { opacity: 1; }
    60%, 100% { opacity: 0; }
  }
  @keyframes focus-b {
    0%, 11% { opacity: 1; }
    13%, 58% { opacity: 0; }
    60%, 100% { opacity: 1; }
  }

  /* the capsule */
  .cap {
    position: absolute;
    left: 50%;
    bottom: 9px;
    height: 10px;
    width: 46%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1.5px;
    border-radius: 6px;
    background: var(--accent);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
    translate: -50% 0;
    opacity: 0;
    scale: 0.6;
    transition:
      bottom 420ms var(--spr),
      width 420ms var(--spr);
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
    bottom: calc(100% - 19px);
  }
  [data-w="compact"] .cap {
    width: 32%;
  }
  [data-w="wide"] .cap {
    width: 70%;
  }
  /* on: the capsule shows on the screen in use; off: always on "Principal" */
  .main .cap {
    animation: cap-both var(--loop) var(--ease-out) infinite;
  }
  .follow .main .cap {
    animation-name: cap-first;
  }
  .follow .second .cap {
    animation: cap-second var(--loop) var(--ease-out) infinite;
  }
  @keyframes cap-first {
    0%, 16% { opacity: 0; scale: 0.6; }
    20%, 42% { opacity: 1; scale: 1; }
    46%, 100% { opacity: 0; scale: 0.85; }
  }
  @keyframes cap-second {
    0%, 63% { opacity: 0; scale: 0.6; }
    67%, 90% { opacity: 1; scale: 1; }
    94%, 100% { opacity: 0; scale: 0.85; }
  }
  @keyframes cap-both {
    0%, 16% { opacity: 0; scale: 0.6; }
    20%, 42% { opacity: 1; scale: 1; }
    46%, 63% { opacity: 0; scale: 0.6; }
    67%, 90% { opacity: 1; scale: 1; }
    94%, 100% { opacity: 0; scale: 0.85; }
  }

  /* the pointer: hops between the two windows and "clicks" each one */
  .mouse {
    --ax: calc(var(--mw) * 0.42);
    --ay: 52px;
    --bx: calc(var(--mw) + var(--gap) + var(--ow) * 0.52);
    --by: 63px;
    position: absolute;
    left: -1px;
    top: -1px;
    fill: var(--text);
    stroke: var(--bg);
    stroke-width: 1;
    filter: drop-shadow(0 1px 1.5px rgba(0, 0, 0, 0.3));
    transform-origin: 1px 1px;
    animation: hop var(--loop) cubic-bezier(0.65, 0, 0.35, 1) infinite;
  }
  @keyframes hop {
    0% { transform: translate(var(--bx), var(--by)); }
    9% { transform: translate(var(--ax), var(--ay)); }
    11% { transform: translate(var(--ax), var(--ay)) scale(0.8); }
    13%, 47% { transform: translate(var(--ax), var(--ay)); }
    56% { transform: translate(var(--bx), var(--by)); }
    58% { transform: translate(var(--bx), var(--by)) scale(0.8); }
    60%, 100% { transform: translate(var(--bx), var(--by)); }
  }

  @media (prefers-reduced-motion: reduce) {
    .mini * {
      animation: none !important;
    }
    .mouse {
      transform: translate(var(--bx), var(--by));
    }
    .win.b b,
    .main .cap,
    .follow .second .cap {
      opacity: 1;
      scale: 1;
    }
    .follow .main .cap {
      opacity: 0;
    }
  }

  /* first block under the section label: the label's hairline is the separator (like Section rows) */
  .upd {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 6px 0 14px;
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
    color: var(--ok);
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
  .gnote {
    margin: -4px 0 12px;
    font-size: 12px;
    color: var(--text-2);
  }
  .langp {
    width: 360px;
    max-width: 100%;
  }
</style>
