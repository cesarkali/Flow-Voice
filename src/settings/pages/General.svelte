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
  <Row label={t("gen.ui_language")}>
    <Segmented
      label={t("gen.ui_language")}
      bind:value={s.ui_language}
      options={[
        { value: "pt", label: "Português" },
        { value: "en", label: "English" },
        { value: "es", label: "Español" },
      ]}
    />
  </Row>
  <Row label={t("gen.theme")}>
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
  <Row label={t("gen.overlay_position")}>
    <Segmented
      label={t("gen.overlay_position")}
      bind:value={s.overlay_position}
      options={[
        { value: "bottom", label: t("gen.pos_bottom"), icon: "arrowDown" },
        { value: "top", label: t("gen.pos_top"), icon: "arrowUp" },
      ]}
    />
  </Row>
</Section>

<Section label={t("gen.sec_behavior")}>
  <Row label={t("gen.auto_paste")} hint={t("gen.auto_paste_hint")}>
    <Toggle label={t("gen.auto_paste")} bind:checked={s.auto_paste} />
  </Row>
  <Row label={t("gen.mute")}>
    <Toggle label={t("gen.mute")} bind:checked={s.mute_on_record} />
  </Row>
  <Row label={t("gen.sounds")}>
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
</style>
