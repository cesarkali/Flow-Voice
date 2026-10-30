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
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 24px;
    row-gap: 10px;
    padding: 14px 0;
    border-top: 1px solid var(--line);
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
