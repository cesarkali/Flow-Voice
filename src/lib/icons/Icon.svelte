<script lang="ts">
  import { ICONS, type IconName } from "./paths";

  let { name, size = 20, stroke = 1.5, class: cls = "" }: { name: IconName; size?: number; stroke?: number; class?: string } =
    $props();
</script>

<svg
  class="icon {cls}"
  width={size}
  height={size}
  viewBox="0 0 20 20"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {@html ICONS[name]}
</svg>

<style>
  .icon {
    flex: none;
    display: block;
  }
</style>
