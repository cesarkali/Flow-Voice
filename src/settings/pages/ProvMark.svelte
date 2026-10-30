<script lang="ts">
  // Test result glyph: the check draws itself on success; the alert mark fades/scales in on failure.
  let { ok, size = 16 }: { ok: boolean; size?: number } = $props();
</script>

<svg class="mark" class:ok width={size} height={size} viewBox="0 0 20 20" aria-hidden="true">
  {#if ok}
    <circle class="ring" cx="10" cy="10" r="8.25" />
    <path class="tick" d="M6.4 10.3l2.5 2.5 4.8-5.3" />
  {:else}
    <circle class="ring-x" cx="10" cy="10" r="8.25" />
    <path class="bang" d="M10 6v4.6M10 13.6v.1" />
  {/if}
</svg>

<style>
  .mark {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    overflow: visible;
  }
  .ring {
    stroke-width: 1.4;
    stroke-dasharray: 52;
    stroke-dashoffset: 52;
    transform: rotate(-90deg);
    transform-origin: center;
    animation: draw 420ms var(--ease-out) forwards;
  }
  .tick {
    stroke-width: 2;
    stroke-dasharray: 12;
    stroke-dashoffset: 12;
    animation: draw 260ms 280ms var(--ease-out) forwards;
  }
  .ring-x {
    stroke-width: 1.4;
    transform-origin: center;
    animation: pop 320ms var(--spring) both;
  }
  .bang {
    stroke-width: 1.9;
    transform-origin: center;
    animation: pop 320ms 90ms var(--spring) both;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
  }
</style>
