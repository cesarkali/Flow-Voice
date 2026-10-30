<script lang="ts">
  import type { Snippet } from "svelte";

  let { tone = "neutral", children }: { tone?: "neutral" | "accent" | "danger" | "solid"; children: Snippet } = $props();
</script>

<span class="badge {tone}">{@render children()}</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 19px;
    padding: 0 7px;
    border-radius: 4px;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
    border: 1px solid var(--line-strong);
    color: var(--text-2);
    flex: none;
    animation: badge-in 260ms var(--spring) both;
  }
  @keyframes badge-in {
    from {
      opacity: 0;
      transform: scale(0.85);
    }
  }
  .accent {
    color: var(--accent-fg);
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .danger {
    color: var(--danger-fg);
    border-color: var(--danger-line);
    background: var(--danger-soft);
  }
  .solid {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
</style>
