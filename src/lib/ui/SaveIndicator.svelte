<script lang="ts">
  import { app } from "../stores.svelte";
  import { t } from "../i18n.svelte";
  import Icon from "../icons/Icon.svelte";
</script>

<div class="save" class:show={app.save !== "idle"} class:err={app.save === "error"} aria-live="polite" title={app.saveError}>
  {#if app.save === "saving"}
    <span class="dot"></span>{t("c.saving")}
  {:else if app.save === "saved"}
    <Icon name="check" size={14} stroke={2} />{t("c.saved")}
  {:else if app.save === "error"}
    <Icon name="alert" size={14} />{t("ov.error")}
  {/if}
</div>

<style>
  .save {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--accent-fg);
    opacity: 0;
    transform: translateY(2px);
    transition:
      opacity var(--t) var(--ease),
      transform var(--t) var(--ease);
    white-space: nowrap;
  }
  .show {
    opacity: 1;
    transform: none;
  }
  .err {
    color: var(--danger-fg);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.6;
  }
</style>
