<script lang="ts">
  // Wizard step "Comportamento": the basic behavior settings, each with a tiny live illustration.
  // Illustrations only move when the value changes (transform/opacity transitions, nothing loops).
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";

  const s = $derived(app.settings!);
</script>

<div class="beh stagger">
  <!-- overlay position -->
  <div class="opt">
    <div class="ill" aria-hidden="true">
      <div class="scr one" class:top={s.overlay_position === "top"}>
        <span class="cap"><i></i><i></i><i></i></span>
      </div>
    </div>
    <div class="txt">
      <span class="ttl">{t("obx.beh_pos")}</span>
      <span class="body">{t("obx.beh_pos_body")}</span>
    </div>
    <div class="ctl">
      <Segmented
        size="sm"
        label={t("obx.beh_pos")}
        bind:value={s.overlay_position}
        options={[
          { value: "bottom", label: t("gen.pos_bottom"), icon: "arrowDown" },
          { value: "top", label: t("gen.pos_top"), icon: "arrowUp" },
        ]}
      />
    </div>
  </div>

  <!-- follow mouse -->
  <div class="opt">
    <div class="ill" aria-hidden="true">
      <div class="duo" class:follow={s.overlay_follow_mouse} class:top={s.overlay_position === "top"}>
        <div class="scr a">
          <span class="cap"><i></i><i></i><i></i></span>
          <span class="n mono">1</span>
        </div>
        <div class="scr b">
          <span class="cap"><i></i><i></i><i></i></span>
          <span class="n mono">2</span>
          <svg class="mouse" viewBox="0 0 12 16" width="8" height="10.5"><path d="M1 1l9 8.5H5.6L3.4 14z" /></svg>
        </div>
      </div>
    </div>
    <div class="txt">
      <span class="ttl">{t("ctl.ov_follow")}</span>
      <span class="body">{t("obx.beh_follow_body")}</span>
    </div>
    <div class="ctl"><Toggle label={t("ctl.ov_follow")} bind:checked={s.overlay_follow_mouse} /></div>
  </div>

  <!-- hotkey mode -->
  <div class="opt">
    <div class="ill" aria-hidden="true">
      <div class="tl" class:hold={s.hotkey_mode === "hold"}>
        <span class="row">
          <span class="lb mono">{t("obx.ill_key")}</span>
          <span class="lane">
            <i class="tap a"></i><i class="tap b"></i><i class="press"></i>
          </span>
        </span>
        <span class="row">
          <span class="lb mono">{t("obx.ill_rec")}</span>
          <span class="lane"><i class="rec"></i></span>
        </span>
      </div>
    </div>
    <div class="txt">
      <span class="ttl">{t("obx.beh_mode")}</span>
      {#key s.hotkey_mode}
        <span class="body swap">{s.hotkey_mode === "hold" ? t("sc.mode_hold_hint") : t("sc.mode_toggle_hint")}</span>
      {/key}
    </div>
    <div class="ctl">
      <Segmented
        size="sm"
        label={t("obx.beh_mode")}
        bind:value={s.hotkey_mode}
        options={[
          { value: "toggle", label: t("obx.mode_toggle") },
          { value: "hold", label: t("obx.mode_hold") },
        ]}
      />
    </div>
  </div>

  <!-- auto paste -->
  <div class="opt">
    <div class="ill" aria-hidden="true">
      <div class="paste" class:on={s.auto_paste}>
        <span class="fieldbox"><i class="ln"></i><i class="caret"></i></span>
        <span class="clip"><Icon name="paste" size={16} /></span>
      </div>
    </div>
    <div class="txt">
      <span class="ttl">{t("gen.auto_paste")}</span>
      <span class="body">{t("obx.beh_paste_body")}</span>
    </div>
    <div class="ctl"><Toggle label={t("gen.auto_paste")} bind:checked={s.auto_paste} /></div>
  </div>

  <!-- sounds -->
  <div class="opt">
    <div class="ill" aria-hidden="true">
      <div class="snd" class:on={s.sounds}>
        <svg viewBox="0 0 48 32" width="60" height="40">
          <path class="spk" d="M8 12h6l7-6v20l-7-6H8z" />
          <path class="w w1" d="M26 11.5a6 6 0 0 1 0 9" />
          <path class="w w2" d="M30 7.5a11.5 11.5 0 0 1 0 17" />
          <path class="x" d="M27 12.5l7 7M34 12.5l-7 7" />
        </svg>
      </div>
    </div>
    <div class="txt">
      <span class="ttl">{t("gen.sounds")}</span>
      <span class="body">{t("ctl.sounds_hint")}</span>
    </div>
    <div class="ctl"><Toggle label={t("gen.sounds")} bind:checked={s.sounds} /></div>
  </div>

  <!-- mute while recording -->
  <div class="opt">
    <div class="ill" aria-hidden="true">
      <div class="mute" class:on={s.mute_on_record}>
        <span class="eq">{#each [0.55, 0.9, 0.7, 1, 0.6, 0.8] as h, i}<i style:--h={h} style:--i={i}></i>{/each}</span>
        <span class="mic"><Icon name="mic" size={14} /></span>
      </div>
    </div>
    <div class="txt">
      <span class="ttl">{t("gen.mute")}</span>
      <span class="body">{t("ctl.mute_hint")}</span>
    </div>
    <div class="ctl"><Toggle label={t("gen.mute")} bind:checked={s.mute_on_record} /></div>
  </div>
</div>

<style>
  .beh {
    margin-top: 20px;
    display: grid;
    grid-template-columns: 1fr;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    overflow: hidden;
    --stagger: 40ms;
    --spr: var(--spring);
  }
  @supports (transition-timing-function: linear(0, 1)) {
    .beh {
      --spr: linear(0, 0.063, 0.206, 0.379, 0.549, 0.698, 0.818, 0.907, 0.968, 1.007, 1.028, 1.037, 1.038, 1.034, 1.028, 1.021, 1.015, 1.01, 1.006, 1.003, 1);
    }
  }
  .opt {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 12px 16px 12px 12px;
    border-top: 1px solid var(--line);
    transition: background-color var(--t) var(--ease);
  }
  .opt:first-child {
    border-top: 0;
  }
  .opt:hover {
    background: var(--hover);
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .ttl {
    font-weight: 600;
    font-size: 13.5px;
  }
  .body {
    font-size: 12px;
    color: var(--text-2);
    line-height: 1.45;
  }
  .swap {
    animation: fv-fade var(--t-slow) var(--ease) both;
  }
  .ctl {
    justify-self: end;
  }
  @media (max-width: 640px) {
    .opt {
      grid-template-columns: 72px minmax(0, 1fr);
    }
    .ctl {
      grid-column: 2;
      justify-self: start;
    }
    .ill {
      transform: scale(0.8);
      transform-origin: left center;
    }
  }

  /* ---------- illustration frame */
  .ill {
    height: 60px;
    border-radius: 9px;
    border: 1px solid var(--line);
    background:
      radial-gradient(circle at 1px 1px, var(--line) 1px, transparent 0) 0 0 / 10px 10px,
      var(--bg);
    display: grid;
    place-items: center;
    overflow: hidden;
    color: var(--text-2);
  }

  /* screens + capsule */
  .scr {
    position: relative;
    border-radius: 3px;
    border: 1.5px solid var(--line-strong);
    background: var(--surface);
  }
  .scr.one {
    width: 64px;
    height: 40px;
  }
  .cap {
    position: absolute;
    left: 50%;
    bottom: 5px;
    width: 46%;
    height: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1.5px;
    border-radius: 4px;
    background: var(--accent);
    translate: -50% 0;
    transition:
      bottom 420ms var(--spr),
      opacity 220ms var(--ease),
      scale 360ms var(--spr);
  }
  .cap i {
    width: 1.5px;
    height: 3px;
    border-radius: 1px;
    background: var(--on-accent);
  }
  .cap i:nth-child(2) {
    height: 4.5px;
  }
  .top .cap {
    bottom: calc(100% - 12px);
  }
  .duo {
    display: flex;
    align-items: flex-end;
    gap: 5px;
  }
  .scr.a {
    width: 42px;
    height: 28px;
  }
  .scr.b {
    width: 34px;
    height: 23px;
  }
  .duo .cap {
    width: 52%;
    height: 6px;
    bottom: 4px;
  }
  .duo.top .cap {
    bottom: calc(100% - 10px);
  }
  .n {
    position: absolute;
    left: 3px;
    bottom: 0;
    font-size: 7px;
    color: var(--text-3);
  }
  .b .cap,
  .follow .a .cap {
    opacity: 0;
    scale: 0.5;
  }
  .follow .b .cap {
    opacity: 1;
    scale: 1;
  }
  .mouse {
    position: absolute;
    right: 4px;
    top: 2px;
    fill: var(--text);
    stroke: var(--surface);
    stroke-width: 1;
    opacity: 0.4;
    transition:
      opacity var(--t) var(--ease),
      transform 420ms var(--spr);
  }
  .follow .mouse {
    opacity: 1;
    transform: translate3d(-4px, 1px, 0);
  }

  /* hotkey mode timeline */
  .tl {
    width: 78px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .lb {
    width: 26px;
    font-size: 6.5px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
  }
  .lane {
    position: relative;
    flex: 1;
    height: 8px;
    border-radius: 2px;
    background: var(--line);
  }
  .lane i {
    position: absolute;
    top: 0;
    height: 100%;
    border-radius: 2px;
  }
  .tap {
    width: 5px;
    background: var(--text);
    transition:
      opacity 200ms var(--ease),
      scale 320ms var(--spr);
  }
  .tap.a {
    left: 6%;
  }
  .tap.b {
    left: 82%;
  }
  .press {
    left: 6%;
    width: 84%;
    background: var(--text);
    transform-origin: left center;
    transform: scaleX(0);
    transition: transform 480ms var(--spr);
  }
  .hold .press {
    transform: scaleX(1);
  }
  .hold .tap {
    opacity: 0;
    scale: 0.4;
  }
  .rec {
    left: 9%;
    width: 75%;
    background: var(--accent);
    transition:
      left 420ms var(--spr),
      width 420ms var(--spr);
  }
  .hold .rec {
    left: 6%;
    width: 84%;
  }

  /* auto paste */
  .paste {
    position: relative;
    width: 80px;
    height: 40px;
  }
  .fieldbox {
    position: absolute;
    left: 0;
    top: 10px;
    width: 52px;
    height: 20px;
    border-radius: 4px;
    border: 1.5px solid var(--line-strong);
    background: var(--surface);
    display: flex;
    align-items: center;
    padding: 0 4px;
    gap: 2px;
  }
  .ln {
    height: 4px;
    width: 30px;
    border-radius: 2px;
    background: var(--text-2);
    transform-origin: left center;
    transform: scaleX(0);
    transition: transform 360ms 160ms var(--ease-out);
  }
  .on .ln {
    transform: scaleX(1);
  }
  .caret {
    width: 1.5px;
    height: 10px;
    background: var(--accent-fg);
  }
  .clip {
    position: absolute;
    right: 0;
    top: 11px;
    display: grid;
    color: var(--text-3);
    transition:
      color var(--t) var(--ease),
      scale 320ms var(--spr);
  }
  .paste:not(.on) .clip {
    color: var(--accent-fg);
    scale: 1.12;
  }

  /* sounds */
  .snd svg {
    display: block;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .spk {
    fill: var(--surface);
    transition: stroke var(--t) var(--ease);
  }
  .on .spk {
    stroke: var(--text);
  }
  .w {
    stroke: var(--accent-fg);
    opacity: 0;
    transform-origin: 22px 16px;
    transform: scale(0.6);
    transition:
      opacity 220ms var(--ease),
      transform 420ms var(--spr);
  }
  .on .w {
    opacity: 1;
    transform: none;
  }
  .on .w2 {
    transition-delay: 70ms;
  }
  .x {
    stroke: var(--text-3);
    transform-origin: 30.5px 16px;
    transition:
      opacity 200ms var(--ease),
      transform 320ms var(--spr);
  }
  .on .x {
    opacity: 0;
    transform: scale(0.4) rotate(45deg);
  }

  /* mute while recording */
  .mute {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    height: 30px;
  }
  .eq {
    display: flex;
    align-items: flex-end;
    gap: 3px;
    height: 100%;
  }
  .eq i {
    width: 4px;
    height: 100%;
    border-radius: 2px;
    background: var(--text-2);
    transform-origin: bottom center;
    transform: scaleY(var(--h));
    transition:
      transform 420ms var(--spr),
      background-color var(--t) var(--ease);
    transition-delay: calc(var(--i) * 25ms);
  }
  .on .eq i {
    transform: scaleY(0.12);
    background: var(--text-3);
  }
  .mic {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-bottom: 4px;
    border-radius: 50%;
    color: var(--text-3);
    border: 1px solid var(--line-strong);
    transition:
      background-color var(--t) var(--ease),
      color var(--t) var(--ease),
      border-color var(--t) var(--ease),
      scale 360ms var(--spr);
  }
  .on .mic {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    scale: 1.08;
  }

  @media (prefers-reduced-motion: reduce) {
    .beh * {
      transition-duration: 0ms !important;
      transition-delay: 0ms !important;
    }
  }
</style>
