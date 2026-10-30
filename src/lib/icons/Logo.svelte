<script lang="ts">
  // The FlowVoice mark: black rounded square with a glowing white sine burst.
  let { size = 40, glow = true }: { size?: number; glow?: boolean } = $props();
  const uid = Math.random().toString(36).slice(2, 8);
</script>

<svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" class="logo">
  <defs>
    <radialGradient id="lg-{uid}" cx="50%" cy="38%" r="70%">
      <stop offset="0" stop-color="#2a2a2d" />
      <stop offset="1" stop-color="#050505" />
    </radialGradient>
    {#if glow}
      <filter id="gl-{uid}" x="-20%" y="-50%" width="140%" height="200%">
        <feGaussianBlur stdDeviation="1.6" />
      </filter>
    {/if}
  </defs>
  <rect x="1" y="1" width="62" height="62" rx="14" fill="url(#lg-{uid})" stroke="rgba(255,255,255,.12)" />
  {#if glow}
    <path
      d="M7 33h7c2.5 0 3.3-4 5-4s2.6 9 4.6 9 3-22 5.4-22 3.4 31 5.8 31 3.2-21 5.2-21 2.6 12 4.4 12 2.4-6 4-6H57"
      fill="none"
      stroke="#fff"
      stroke-width="3"
      opacity=".45"
      filter="url(#gl-{uid})"
    />
  {/if}
  <path
    d="M7 33h7c2.5 0 3.3-4 5-4s2.6 9 4.6 9 3-22 5.4-22 3.4 31 5.8 31 3.2-21 5.2-21 2.6 12 4.4 12 2.4-6 4-6H57"
    fill="none"
    stroke="#fff"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>

<style>
  .logo {
    display: block;
    flex: none;
  }
</style>
