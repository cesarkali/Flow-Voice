<script lang="ts">
  import type { Snippet } from "svelte";

  let { label, aside, children }: { label: string; aside?: Snippet; children: Snippet } = $props();
</script>

<section class="sec">
  <header>
    <h2 class="label">{label}</h2>
    <span class="rule" aria-hidden="true"></span>
    {#if aside}<div class="aside">{@render aside()}</div>{/if}
  </header>
  {@render children()}
</section>

<style>
  .sec {
    margin-top: 34px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
    min-height: 27px;
  }
  h2 {
    font-family: var(--mono);
    letter-spacing: 0.12em;
    flex: none;
  }
  /* hairline that draws itself from the label to the edge */
  .rule {
    flex: 1;
    height: 1px;
    min-width: 12px;
    background: linear-gradient(90deg, var(--line-strong), transparent);
    transform-origin: left;
    animation: rule-in 520ms var(--ease) both;
  }
  @keyframes rule-in {
    from {
      transform: scaleX(0);
      opacity: 0;
    }
  }
  .aside {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
