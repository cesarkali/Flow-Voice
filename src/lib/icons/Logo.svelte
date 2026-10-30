<script lang="ts">
  // The FlowVoice mark ("Pauta"): voice bars landing on a ruled line as a lime text caret.
  // animate: "intro" plays once on mount (and on hover); "loop" keeps the bars gently speaking
  // and the caret blinking (hero spots only); "none" is static. Small sizes use simplified art.
  let {
    size = 40,
    glow = true,
    animate = "intro",
  }: { size?: number; glow?: boolean; animate?: "none" | "intro" | "loop" } = $props();

  const uid = $props.id();
  const small = $derived(size <= 28);
  let replay = $state(0);

  // Bars follow a natural speech envelope: short attack, peak, decay into the caret.
  const BARS = [
    { x: 168, y: 427, h: 170 },
    { x: 288, y: 317, h: 390 },
    { x: 408, y: 382, h: 260 },
    { x: 520, y: 447, h: 130 },
  ];
  const SMALL_BARS = [
    { x: 182, y: 362, h: 300 },
    { x: 382, y: 252, h: 520 },
  ];
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 1024 1024"
  aria-hidden="true"
  class="logo anim-{animate}"
  class:small
  onmouseenter={() => animate !== "none" && replay++}
>
  <defs>
    <linearGradient id="tile-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={small ? "#222226" : "#1E1E21"} />
      <stop offset="1" stop-color="#0B0B0C" />
    </linearGradient>
    <linearGradient id="rim-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity=".22" />
      <stop offset=".18" stop-color="#fff" stop-opacity=".04" />
      <stop offset=".85" stop-color="#000" stop-opacity="0" />
      <stop offset="1" stop-color="#000" stop-opacity=".5" />
    </linearGradient>
    <linearGradient id="matte-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" />
      <stop offset="1" stop-color="#C9C9C4" />
    </linearGradient>
    <linearGradient id="lime-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#EBFF8F" />
      <stop offset="1" stop-color="#B9E01F" />
    </linearGradient>
    {#if !small}
      <pattern id="dots-{uid}" width="32" height="32" patternUnits="userSpaceOnUse">
        <circle cx="16" cy="16" r="2.4" fill="#fff" fill-opacity=".05" />
      </pattern>
      {#if glow}
        <filter id="bloom-{uid}" x="-60%" y="-30%" width="220%" height="160%">
          <feGaussianBlur stdDeviation="30" />
        </filter>
      {/if}
    {/if}
  </defs>

  <rect width="1024" height="1024" rx="232" fill="url(#tile-{uid})" />
  {#if !small}
    <rect x="5" y="5" width="1014" height="1014" rx="227" fill="none" stroke="url(#rim-{uid})" stroke-width="10" />
    <rect width="1024" height="1024" rx="232" fill="url(#dots-{uid})" />
    {#each [300, 512, 724] as y (y)}
      <path d="M150 {y}H874" stroke="#000" stroke-opacity=".55" stroke-width="6" />
      <path d="M150 {y + 5}H874" stroke="#fff" stroke-opacity=".10" stroke-width="4" />
    {/each}
  {/if}

  {#key replay}
    {#if !small}
      <path class="base" d="M180 774H874" stroke="#D7FF3F" stroke-opacity=".45" stroke-width="12" stroke-linecap="round" pathLength="1" />
    {/if}
    <g class="bars">
      {#each small ? SMALL_BARS : BARS as b, i (i)}
        <rect
          class="bar"
          style:--i={i}
          x={b.x}
          y={b.y}
          width={small ? 136 : 92}
          height={b.h}
          rx={small ? 68 : 46}
          fill={small ? "#EDEDEA" : `url(#matte-${uid})`}
        />
      {/each}
    </g>
    <g class="caret">
      {#if !small && glow}
        <g class="bloom" filter="url(#bloom-{uid})" fill="#D7FF3F">
          <rect x="710" y="250" width="84" height="524" rx="42" />
        </g>
      {/if}
      <g fill={small ? "#D7FF3F" : `url(#lime-${uid})`}>
        {#if small}
          <rect x="666" y="196" width="136" height="632" rx="40" />
          <rect x="584" y="164" width="300" height="128" rx="56" />
          <rect x="584" y="732" width="300" height="128" rx="56" />
        {:else}
          <rect x="710" y="250" width="84" height="524" rx="42" />
          <rect x="644" y="208" width="216" height="84" rx="42" />
          <rect x="644" y="732" width="216" height="84" rx="42" />
        {/if}
      </g>
    </g>
  {/key}
</svg>

<style>
  .logo {
    display: block;
    flex: none;
    overflow: visible;
  }
  .bar {
    transform-box: fill-box;
    transform-origin: 50% 50%;
  }
  .caret {
    transform-box: fill-box;
    transform-origin: 50% 0%;
  }
  .bloom {
    opacity: 0.55;
  }

  /* ---- intro: baseline draws, bars rise one by one, caret drops in and flashes */
  .anim-intro .base,
  .anim-loop .base {
    stroke-dasharray: 1;
    animation: fv-logo-draw 700ms cubic-bezier(0.65, 0, 0.35, 1) both;
  }
  .anim-intro .bar,
  .anim-loop .bar {
    animation: fv-logo-rise 620ms cubic-bezier(0.2, 0.9, 0.25, 1.25) both;
    animation-delay: calc(140ms + var(--i) * 80ms);
  }
  .anim-intro .caret,
  .anim-loop .caret {
    animation: fv-logo-drop 560ms cubic-bezier(0.2, 0.9, 0.25, 1.2) 420ms both;
  }
  .anim-intro .bloom {
    animation: fv-logo-flash 1200ms ease-out 700ms both;
  }

  /* ---- loop (hero): bars keep speaking softly, the caret blinks like a text cursor */
  .anim-loop .bar {
    animation:
      fv-logo-rise 620ms cubic-bezier(0.2, 0.9, 0.25, 1.25) both,
      fv-logo-speak 2.8s ease-in-out infinite;
    animation-delay: calc(140ms + var(--i) * 80ms), calc(1.2s + var(--i) * 140ms);
  }
  .anim-loop .bloom {
    animation: fv-logo-glow 2.8s ease-in-out 1.2s infinite;
  }
  .anim-loop .caret > g:last-child {
    animation: fv-logo-blink 1.1s steps(1, end) 2s infinite;
  }

  @keyframes fv-logo-draw {
    from {
      stroke-dashoffset: 1;
    }
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes fv-logo-rise {
    from {
      opacity: 0;
      transform: scaleY(0.15);
    }
    to {
      opacity: 1;
      transform: scaleY(1);
    }
  }
  @keyframes fv-logo-drop {
    from {
      opacity: 0;
      transform: translateY(-18%) scaleY(0.9);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes fv-logo-flash {
    0% {
      opacity: 0;
    }
    35% {
      opacity: 1;
    }
    100% {
      opacity: 0.55;
    }
  }
  @keyframes fv-logo-speak {
    0%,
    100% {
      transform: scaleY(1);
    }
    18% {
      transform: scaleY(0.62);
    }
    36% {
      transform: scaleY(1.1);
    }
    54% {
      transform: scaleY(0.82);
    }
    72% {
      transform: scaleY(1.04);
    }
  }
  @keyframes fv-logo-glow {
    0%,
    100% {
      opacity: 0.45;
    }
    40% {
      opacity: 0.8;
    }
  }
  @keyframes fv-logo-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .logo *,
    .logo {
      animation: none !important;
    }
  }
</style>
