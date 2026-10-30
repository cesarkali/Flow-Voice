<script lang="ts">
  import { keycapsOf } from "./hotkeys";

  // Renders an accelerator ("CommandOrControl+Shift+Space") or a list of labels as keycaps.
  // `animate` pops each cap in (used while capturing a new shortcut).
  // `pressed` draws the caps pushed down.
  let {
    accel = "",
    caps,
    size = "md",
    animate = false,
    pressed = false,
  }: { accel?: string; caps?: string[]; size?: "sm" | "md" | "lg"; animate?: boolean; pressed?: boolean } = $props();
  const list = $derived(caps ?? keycapsOf(accel));
</script>

<span class="keys {size}" class:animate class:pressed aria-label={list.join(" + ")}>
  {#each list as k, i (k + i)}
    <kbd style="--i:{i}">{k}</kbd>
  {/each}
</span>

<style>
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    vertical-align: middle;
    flex-wrap: nowrap;
  }
  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    color: var(--text);
    background: linear-gradient(var(--surface), var(--surface-2));
    border: 1px solid var(--line-strong);
    border-radius: 5px;
    box-shadow:
      inset 0 -2px 0 var(--line-strong),
      0 1px 0 rgba(0, 0, 0, 0.12);
    padding-bottom: 2px;
    white-space: nowrap;
    transition:
      transform 120ms var(--ease),
      box-shadow 120ms var(--ease);
  }
  .pressed kbd {
    transform: translateY(1px);
    box-shadow: inset 0 -1px 0 var(--line-strong);
    padding-bottom: 1px;
  }
  .animate kbd {
    animation: cap-in 260ms var(--spring) both;
    animation-delay: calc(var(--i) * 30ms);
  }
  @keyframes cap-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.8);
    }
  }
  .sm kbd {
    min-width: 18px;
    height: 18px;
    padding: 0 4px 1px;
    font-size: 10px;
    border-radius: 4px;
    box-shadow: inset 0 -1.5px 0 var(--line-strong);
  }
  .lg kbd {
    min-width: 30px;
    height: 30px;
    padding: 0 9px 3px;
    font-size: 13px;
    border-radius: 7px;
    box-shadow:
      inset 0 -3px 0 var(--line-strong),
      0 1px 0 rgba(0, 0, 0, 0.14);
  }
</style>
