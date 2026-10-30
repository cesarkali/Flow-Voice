<script lang="ts">
  let { value, label }: { value: number; label: string } = $props();
  const v = $derived(Math.max(0, Math.min(1, value || 0)));
  const busy = $derived(v > 0 && v < 1);
</script>

<div
  class="prog"
  class:busy
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(v * 100)}
>
  <span class="fill" style="transform: scaleX({v})"></span>
</div>

<style>
  .prog {
    position: relative;
    height: 3px;
    border-radius: 3px;
    background: var(--surface-3);
    overflow: hidden;
    width: 100%;
  }
  .fill {
    position: relative;
    overflow: hidden;
    display: block;
    height: 100%;
    background: var(--accent-fg);
    transform-origin: left;
    transition: transform 260ms var(--ease);
  }
  /* a soft highlight travels along while it is actually moving */
  .busy .fill::after {
    content: "";
    position: absolute;
    inset: 0;
    width: 30%;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.45), transparent);
    animation: sheen 1.3s var(--ease) infinite;
    mix-blend-mode: overlay;
  }
  @keyframes sheen {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .busy .fill::after {
      display: none;
    }
  }
</style>
