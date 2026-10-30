<script lang="ts">
  // Simplified inline flags (Windows doesn't render flag emoji). 3:2 with rounded corners.
  let { code, size = 20 }: { code: "pt" | "en" | "es"; size?: number } = $props();
  const uid = $props.id();
</script>

<svg class="flag" width={size} height={Math.round((size * 2) / 3)} viewBox="0 0 30 20" aria-hidden="true">
  <defs>
    <clipPath id="fl-{uid}"><rect width="30" height="20" rx="3" /></clipPath>
  </defs>
  <g clip-path="url(#fl-{uid})">
    {#if code === "pt"}
      <rect width="30" height="20" fill="#229e45" />
      <path d="M15 2.6 27 10 15 17.4 3 10Z" fill="#f8e509" />
      <circle cx="15" cy="10" r="4.6" fill="#2b49a3" />
      <path d="M10.6 9.1c3-.9 6.3-.6 8.9.9" stroke="#fff" stroke-width="1" fill="none" />
    {:else if code === "en"}
      <rect width="30" height="20" fill="#fff" />
      {#each [0, 2, 4, 6, 8, 10, 12] as i (i)}
        <rect y={i * (20 / 13)} width="30" height={20 / 13} fill="#b22234" />
      {/each}
      <rect width="13" height={(20 / 13) * 7} fill="#3c3b6e" />
      {#each [[2.2, 2], [6.5, 2], [10.8, 2], [4.3, 4.7], [8.6, 4.7], [2.2, 7.4], [6.5, 7.4], [10.8, 7.4]] as [x, y] (`${x}-${y}`)}
        <circle cx={x} cy={y} r="0.75" fill="#fff" />
      {/each}
    {:else}
      <rect width="30" height="20" fill="#c60b1e" />
      <rect y="5" width="30" height="10" fill="#ffc400" />
      <rect x="7" y="7.6" width="3.2" height="4.8" rx="0.8" fill="#ad1519" opacity="0.85" />
    {/if}
  </g>
  <rect width="30" height="20" rx="3" fill="none" stroke="rgba(0,0,0,0.25)" stroke-width="0.6" />
</svg>

<style>
  .flag {
    display: block;
    flex: none;
  }
</style>
