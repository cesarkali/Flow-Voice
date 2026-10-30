<script lang="ts">
  import type { Snippet } from "svelte";

  let { label, aside, children }: { label: string; aside?: Snippet; children: Snippet } = $props();
</script>

<section class="sec">
  <header>
    <h2 class="label">{label}</h2>
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
  }
  .aside {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
