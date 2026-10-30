<script lang="ts">
  import { langName, t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import type { SettingsPage } from "../../lib/types";
  import HotkeyInput from "../../lib/ui/HotkeyInput.svelte";
  import Keys from "../../lib/ui/Keys.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);
</script>

<header class="ph">
  <h1>{t("sc.title")}</h1>
  <p class="muted">{t("sc.subtitle")}</p>
</header>

<Section label={t("sc.title")}>
  <Row label={t("sc.dictate")} hint={t("sc.dictate_hint")}>
    <HotkeyInput label={t("sc.dictate")} bind:value={s.hotkeys.dictate} />
  </Row>
  <Row label={t("sc.translate")} hint={t("sc.translate_hint", { lang: langName(s.translation_target) })}>
    <HotkeyInput label={t("sc.translate")} bind:value={s.hotkeys.translate} />
  </Row>
  <Row label={t("sc.ask")} hint={t("sc.ask_hint")}>
    <HotkeyInput label={t("sc.ask")} bind:value={s.hotkeys.ask} />
  </Row>
</Section>

<Section label={t("sc.mode")}>
  <Row label={t("sc.mode")} hint={s.hotkey_mode === "toggle" ? t("sc.mode_toggle_hint") : t("sc.mode_hold_hint")}>
    <Segmented
      label={t("sc.mode")}
      bind:value={s.hotkey_mode}
      options={[
        { value: "toggle", label: t("sc.mode_toggle") },
        { value: "hold", label: t("sc.mode_hold") },
      ]}
    />
  </Row>
  <div class="diagram" aria-hidden="true">
    {#if s.hotkey_mode === "toggle"}
      <div class="track">
        <span class="press">1×</span><span class="bar"></span><span class="press">1×</span>
      </div>
    {:else}
      <div class="track">
        <span class="hold"></span>
      </div>
    {/if}
  </div>
  <p class="esc"><Icon name="info" size={15} /><Keys caps={["Esc"]} size="sm" /><span>{t("sc.esc_info")}</span></p>
</Section>

<style>
  .ph h1 {
    font-size: 26px;
  }
  .ph p {
    margin-top: 6px;
    font-size: 13.5px;
  }
  .diagram {
    padding: 4px 0 16px;
  }
  .track {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 22px;
    max-width: 320px;
  }
  .press {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--on-accent);
    background: var(--accent);
    border-radius: 4px;
    padding: 2px 6px;
    animation: pop 220ms var(--spring) both;
  }
  .bar {
    flex: 1;
    height: 2px;
    background: repeating-linear-gradient(90deg, var(--accent-fg) 0 4px, transparent 4px 8px);
    opacity: 0.6;
  }
  .hold {
    flex: 1;
    height: 18px;
    border-radius: 4px;
    background: var(--accent);
    animation: grow 420ms var(--ease) both;
    transform-origin: left;
  }
  @keyframes pop {
    from {
      transform: scale(0.6);
      opacity: 0;
    }
  }
  @keyframes grow {
    from {
      transform: scaleX(0.05);
    }
  }
  .esc {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-2);
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
</style>
