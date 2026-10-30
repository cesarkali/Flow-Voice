<script lang="ts">
  import type { Snippet } from "svelte";

  // A settings row: label + hint on the left, control on the right. Hairline separated.
  let {
    label,
    hint,
    children,
    below,
    stack = false,
    id,
  }: { label: string; hint?: string; children?: Snippet; below?: Snippet; stack?: boolean; id?: string } = $props();
</script>

<div class="row" class:stack {id}>
  <div class="txt">
    <div class="lbl">{label}</div>
    {#if hint}<div class="hint">{hint}</div>{/if}
  </div>
  {#if children}<div class="ctl">{@render children()}</div>{/if}
  {#if below}<div class="below">{@render below()}</div>{/if}
</div>

<style>
  .row {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 24px;
    row-gap: 10px;
    padding: 14px 0;
    border-top: 1px solid var(--line);
  }
  /* small tick on the left edge: hairline on hover, accent while a control inside is focused */
  .row::before {
    content: "";
    position: absolute;
    left: -12px;
    top: 50%;
    width: 2px;
    height: 16px;
    margin-top: -8px;
    border-radius: 2px;
    background: var(--line-strong);
    opacity: 0;
    transform: scaleY(0.4);
    transition:
      opacity var(--t) var(--ease),
      transform var(--t) var(--spring),
      background-color var(--t) var(--ease);
  }
  .row:hover::before {
    opacity: 1;
    transform: none;
  }
  .row:has(:focus-visible)::before {
    opacity: 1;
    transform: none;
    background: var(--accent-fg);
  }
  .lbl,
  .hint {
    transition: color var(--t) var(--ease);
  }
  .row.stack {
    grid-template-columns: minmax(0, 1fr);
  }
  .lbl {
    font-size: 14px;
    color: var(--text);
  }
  .hint {
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.45;
    max-width: 52ch;
    text-wrap: pretty;
  }
  .ctl {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: flex-end;
  }
  .stack .ctl {
    justify-content: flex-start;
  }
  .below {
    grid-column: 1 / -1;
  }
</style>
