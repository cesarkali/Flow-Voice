<script lang="ts">
  import { keycapsOf } from "./hotkeys";

  // Renders an accelerator ("CommandOrControl+Shift+Space") or a list of labels as keycaps.
  let { accel = "", caps, size = "md" }: { accel?: string; caps?: string[]; size?: "sm" | "md" | "lg" } = $props();
  const list = $derived(caps ?? keycapsOf(accel));
</script>

<span class="keys {size}" aria-label={list.join(" + ")}>
  {#each list as k, i (i)}
    <kbd>{k}</kbd>
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
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-bottom-width: 2px;
    border-radius: 5px;
    white-space: nowrap;
  }
  .sm kbd {
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    font-size: 10px;
    border-radius: 4px;
  }
  .lg kbd {
    min-width: 30px;
    height: 30px;
    padding: 0 9px;
    font-size: 13px;
    border-radius: 7px;
    border-bottom-width: 3px;
  }
</style>
