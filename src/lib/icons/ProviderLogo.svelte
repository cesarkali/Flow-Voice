<script lang="ts">
  // Small provider marks on a brand-colored tile. Gemini and OpenAI use their glyphs; the others
  // are simple drawn marks evoking each brand (no fonts needed).
  let { id, size = 20 }: { id: string; size?: number } = $props();

  const BG: Record<string, string> = {
    groq: "#F55036",
    gemini: "#1A73E8",
    openai: "#ECECEC",
    openrouter: "#6467F2",
    mistral: "#111113",
    ollama: "#F4F3EF",
    custom: "#3A3A3F",
    tavily: "#2563EB",
    local: "#1C1C1F",
  };
  const GEMINI = "M11.04 19.32Q12 21.51 12 24q0-2.49.93-4.68.96-2.19 2.58-3.81t3.81-2.55Q21.51 12 24 12q-2.49 0-4.68-.93a12.3 12.3 0 0 1-3.81-2.58 12.3 12.3 0 0 1-2.58-3.81Q12 2.49 12 0q0 2.49-.96 4.68-.93 2.19-2.55 3.81a12.3 12.3 0 0 1-3.81 2.58Q2.49 12 0 12q2.49 0 4.68.96 2.19.93 3.81 2.55t2.55 3.81";
  const OPENAI = "M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z";
  // Mistral's pixel "M" in its yellow-to-red bands: [col, row, colorIndex]
  const MISTRAL_COLORS = ["#FFD800", "#FFAF00", "#FF8205", "#FA500F", "#E10500"];
  const MISTRAL: [number, number][] = [
    [0, 0], [4, 0],
    [0, 1], [1, 1], [3, 1], [4, 1],
    [0, 2], [1, 2], [2, 2], [3, 2], [4, 2],
    [0, 3], [2, 3], [4, 3],
    [0, 4], [4, 4],
  ];
  const known = $derived(id in BG);
</script>

<svg class="pl" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  <rect width="24" height="24" rx="6" fill={BG[id] ?? "#3A3A3F"} />
  {#if id === "gemini"}
    <g transform="translate(5 5) scale(0.5833)"><path d={GEMINI} fill="#fff" /></g>
  {:else if id === "openai"}
    <g transform="translate(5 5) scale(0.5833)"><path d={OPENAI} fill="#0B0B0C" /></g>
  {:else if id === "groq"}
    <!-- rounded "g": ring + descender hook -->
    <circle cx="12" cy="10.6" r="4.3" fill="none" stroke="#fff" stroke-width="2.3" />
    <path d="M16.3 10.4v4.4a4.1 4.1 0 0 1-7.4 2.4" fill="none" stroke="#fff" stroke-width="2.3" stroke-linecap="round" />
  {:else if id === "openrouter"}
    <!-- one route splitting into two arrows -->
    <path d="M4.5 12h4.2c2.2 0 2.8-4.5 6.2-4.5h3.6M8.7 12c2.2 0 2.8 4.5 6.2 4.5h3.6" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" />
    <path d="M16.6 5.2l2.4 2.3-2.4 2.3M16.6 14.2l2.4 2.3-2.4 2.3" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
  {:else if id === "mistral"}
    {#each MISTRAL as [c, r] (`${c}-${r}`)}
      <rect x={4.5 + c * 3} y={4.5 + r * 3} width="3" height="3" fill={MISTRAL_COLORS[r]} />
    {/each}
  {:else if id === "ollama"}
    <!-- llama head: two ears, face, eyes -->
    <path
      d="M8.6 10.2V5.6c0-1 .5-1.6 1.1-1.6s1.1.6 1.1 1.6v2.6M15.4 10.2V5.6c0-1-.5-1.6-1.1-1.6s-1.1.6-1.1 1.6v2.6M7.2 19.6v-5.2c0-3.3 2.1-5.4 4.8-5.4s4.8 2.1 4.8 5.4v5.2"
      fill="none"
      stroke="#0B0B0C"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <circle cx="10.2" cy="13.3" r=".95" fill="#0B0B0C" />
    <circle cx="13.8" cy="13.3" r=".95" fill="#0B0B0C" />
    <ellipse cx="12" cy="16.2" rx="1.9" ry="1.3" fill="none" stroke="#0B0B0C" stroke-width="1.3" />
  {:else if id === "custom"}
    <!-- plug: any OpenAI-compatible endpoint -->
    <path d="M9.5 4.5v3.5M14.5 4.5v3.5M7 8h10v3.2a5 5 0 0 1-10 0zM12 16.2v3.3" fill="none" stroke="#EDEDEA" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
  {:else if id === "tavily"}
    <!-- search lens with a spark of the web -->
    <circle cx="10.8" cy="10.8" r="5" fill="none" stroke="#fff" stroke-width="2.1" />
    <path d="M14.6 14.6l4.6 4.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" />
    <path d="M6.3 10.8h9M10.8 6.3c-1.5 1.3-1.5 7.7 0 9" fill="none" stroke="#fff" stroke-width="1" opacity=".7" />
  {:else if id === "local"}
    <!-- offline chip -->
    <rect x="7" y="7" width="10" height="10" rx="2" fill="none" stroke="var(--accent, #D7FF3F)" stroke-width="1.8" />
    <path d="M10 4.5v2.5M14 4.5v2.5M10 17v2.5M14 17v2.5M4.5 10H7M4.5 14H7M17 10h2.5M17 14h2.5" stroke="var(--accent, #D7FF3F)" stroke-width="1.6" stroke-linecap="round" />
  {:else if !known}
    <text
      x="12"
      y="12.5"
      text-anchor="middle"
      dominant-baseline="middle"
      fill="#EDEDEA"
      font-family="Segoe UI, system-ui, sans-serif"
      font-weight="800"
      font-size="13">{id.slice(0, 1).toUpperCase()}</text
    >
  {/if}
</svg>

<style>
  .pl {
    display: block;
    flex: none;
    border-radius: 6px;
  }
</style>
