<script lang="ts">
  let { value, label }: { value: number; label: string } = $props();
</script>

<div
  class="prog"
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(value * 100)}
>
  <span style="transform: scaleX({Math.max(0, Math.min(1, value))})"></span>
</div>

<style>
  .prog {
    height: 3px;
    border-radius: 3px;
    background: var(--surface-3);
    overflow: hidden;
    width: 100%;
  }
  span {
    display: block;
    height: 100%;
    background: var(--accent-fg);
    transform-origin: left;
    transition: transform 160ms linear;
  }
</style>
