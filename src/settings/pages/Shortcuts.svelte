<script lang="ts">
  import { i18n, langName, t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import type { SettingsPage } from "../../lib/types";
  import HotkeyInput from "../../lib/ui/HotkeyInput.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Keys from "../../lib/ui/Keys.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);
  let replay = $state(0);

  type Action = "dictate" | "translate" | "ask" | "command" | "paste_last";
  const LABELS: Record<Action, "sc.dictate" | "sc.translate" | "sc.ask" | "sc.command" | "sc.paste_last"> = {
    dictate: "sc.dictate",
    translate: "sc.translate",
    ask: "sc.ask",
    command: "sc.command",
    paste_last: "sc.paste_last",
  };
  /** The other actions' shortcuts (one combination, one action). */
  const others = (me: Action) =>
    (Object.keys(LABELS) as Action[]).filter((a) => a !== me).map((a) => ({ accel: s.hotkeys[a], label: t(LABELS[a]) }));
</script>

<header class="ph">
  <h1>{t("sc.title")}</h1>
  <p class="muted">{t("sc.subtitle")}</p>
</header>

<Section label={t("sc.title")}>
  <Row id="set-sc-dictate" label={t("sc.dictate")} hint={t("sc.dictate_hint")}>
    <HotkeyInput label={t("sc.dictate")} bind:value={s.hotkeys.dictate} taken={others("dictate")} />
  </Row>
  <Row id="set-sc-translate" label={t("sc.translate")} hint={t("sc.translate_hint", { lang: i18n.lang === "en" ? langName(s.translation_target) : langName(s.translation_target).toLocaleLowerCase() })}>
    <HotkeyInput label={t("sc.translate")} bind:value={s.hotkeys.translate} taken={others("translate")} />
  </Row>
  <Row id="set-sc-ask" label={t("sc.ask")} hint={t("sc.ask_hint")}>
    <HotkeyInput label={t("sc.ask")} bind:value={s.hotkeys.ask} taken={others("ask")} />
  </Row>
</Section>

<Section label={t("sc.sec_more")}>
  <Row id="set-sc-command" label={t("sc.command")} hint={t("sc.command_hint")}>
    <HotkeyInput label={t("sc.command")} bind:value={s.hotkeys.command} clearable taken={others("command")} />
  </Row>
  <Row id="set-sc-paste-last" label={t("sc.paste_last")} hint={t("sc.paste_last_hint")}>
    <HotkeyInput label={t("sc.paste_last")} bind:value={s.hotkeys.paste_last} clearable taken={others("paste_last")} />
  </Row>
</Section>

<Section label={t("sh.sec_recording")}>
  <Row id="set-sc-mode" label={t("sc.mode")} hint={s.hotkey_mode === "toggle" ? t("sc.mode_toggle_hint") : t("sc.mode_hold_hint")}>
    <Segmented
      label={t("sc.mode")}
      bind:value={s.hotkey_mode}
      options={[
        { value: "toggle", label: t("ctl.mode_toggle") },
        { value: "hold", label: t("ctl.mode_hold") },
      ]}
    />
  </Row>
  <div class="demo" aria-hidden="true">
    <div class="dhead">
      <span class="label">{t("ctl.demo_label")}</span>
      <Button size="sm" variant="quiet" icon="refresh" onclick={() => replay++}>{t("ctl.demo_replay")}</Button>
    </div>
    {#key `${s.hotkey_mode}-${replay}`}
      <div class="stage {s.hotkey_mode}">
        <div class="key"><Keys accel={s.hotkeys.dictate} size="lg" /></div>
        <div class="track">
          <span class="rail"></span>
          <span class="fill"></span>
          <span class="mark m1"><i></i><em>{s.hotkey_mode === "toggle" ? t("ctl.demo_tap") : t("ctl.demo_down")}</em></span>
          <span class="mark m2"><i></i><em>{s.hotkey_mode === "toggle" ? t("ctl.demo_tap") : t("ctl.demo_up")}</em></span>
          <span class="rec mono">{t("ctl.demo_rec")}</span>
        </div>
      </div>
    {/key}
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
  /* ---------------------------------------------------------------- mode demo (plays once) */
  .demo {
    padding: 14px 0 16px;
    border-top: 1px solid var(--line);
  }
  .dhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .stage {
    --dur: 2.8s;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 16px 18px 26px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
  }
  .key {
    flex: none;
    animation: tap var(--dur) var(--ease) both;
  }
  .hold .key {
    animation-name: hold;
  }
  /* toggle: two short taps; hold: pressed the whole time */
  @keyframes tap {
    0%, 6% { transform: none; }
    9% { transform: translateY(2px) scale(0.97); }
    14%, 82% { transform: none; }
    85% { transform: translateY(2px) scale(0.97); }
    90%, 100% { transform: none; }
  }
  @keyframes hold {
    0%, 6% { transform: none; }
    10%, 84% { transform: translateY(2px) scale(0.97); }
    90%, 100% { transform: none; }
  }
  .track {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 30px;
  }
  .rail,
  .fill {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 6px;
    margin-top: -3px;
    border-radius: 3px;
  }
  .rail {
    background: repeating-linear-gradient(90deg, var(--line-strong) 0 4px, transparent 4px 8px);
  }
  .fill {
    background: var(--accent);
    transform-origin: left;
    animation: fill var(--dur) linear both;
  }
  @keyframes fill {
    0%, 10% { transform: scaleX(0); }
    84%, 100% { transform: scaleX(1); }
  }
  .hold .fill {
    background: var(--accent);
  }
  .mark {
    position: absolute;
    top: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    translate: -50% 0;
  }
  .m1 {
    left: 0;
    animation: mark var(--dur) var(--spring) both;
    --at: 9%;
  }
  .m2 {
    left: 100%;
    animation: mark2 var(--dur) var(--spring) both;
  }
  .mark i {
    width: 2px;
    height: 100%;
    border-radius: 2px;
    background: var(--text);
  }
  .mark em {
    position: absolute;
    top: 100%;
    margin-top: 4px;
    font-style: normal;
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-2);
    white-space: nowrap;
  }
  @keyframes mark {
    0%, 7% { opacity: 0; transform: scaleY(0.3); }
    12%, 100% { opacity: 1; transform: none; }
  }
  @keyframes mark2 {
    0%, 83% { opacity: 0; transform: scaleY(0.3); }
    89%, 100% { opacity: 1; transform: none; }
  }
  .rec {
    position: absolute;
    left: 50%;
    bottom: 100%;
    translate: -50% 2px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent-fg);
    animation: rec var(--dur) var(--ease) both;
  }
  @keyframes rec {
    0%, 12% { opacity: 0; transform: translateY(4px); }
    20%, 100% { opacity: 1; transform: none; }
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
