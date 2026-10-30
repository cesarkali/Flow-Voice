<script lang="ts" module>
  /** Language code → representative country (flag) key. */
  const COUNTRY: Record<string, string> = {
    en: "us", pt: "br", es: "es", fr: "fr", de: "de", it: "it", ja: "jp", zh: "cn", ko: "kr", ru: "ru",
    nl: "nl", pl: "pl", uk: "ua", tr: "tr", ar: "sa", he: "il", fa: "ir", hi: "in", bn: "bd", ur: "pk",
    id: "id", ms: "my", vi: "vn", th: "th", sv: "se", da: "dk", no: "no", nb: "no", nn: "no", fi: "fi",
    cs: "cz", el: "gr", ro: "ro", hu: "hu", ca: "ct", fil: "ph", tl: "ph", bg: "bg", hr: "hr", sr: "rs",
    sk: "sk", sl: "si",
  };
  /** 5-point star polygon points. */
  function star(cx: number, cy: number, r: number, rot = 0) {
    const p: string[] = [];
    for (let i = 0; i < 10; i++) {
      const a = rot + (Math.PI / 5) * i - Math.PI / 2;
      const rr = i % 2 ? r * 0.4 : r;
      p.push(`${(cx + rr * Math.cos(a)).toFixed(2)},${(cy + rr * Math.sin(a)).toFixed(2)}`);
    }
    return p.join(" ");
  }
  export function hasFlag(code: string) {
    return !!COUNTRY[code.toLowerCase()];
  }
</script>

<script lang="ts">
  // Simplified inline flags (Windows doesn't render flag emoji). 3:2 with rounded corners.
  // `code` is a language code (pt, en, es, ... see COUNTRY); unknown codes get a neutral tile.
  let { code, size = 20 }: { code: string; size?: number } = $props();
  const uid = $props.id();
  const c = $derived(COUNTRY[(code ?? "").toLowerCase()] ?? "");
</script>

{#snippet h3(cols: string[])}
  {#each cols as col, i (i)}<rect y={(i * 20) / cols.length} width="30" height={20 / cols.length + 0.05} fill={col} />{/each}
{/snippet}
{#snippet v3(cols: string[])}
  {#each cols as col, i (i)}<rect x={i * 10} width="10.05" height="20" fill={col} />{/each}
{/snippet}
{#snippet nordic(bg: string, cross: string, x: number, w: number, y: number, h: number)}
  <rect width="30" height="20" fill={bg} />
  <rect {x} width={w} height="20" fill={cross} />
  <rect {y} width="30" height={h} fill={cross} />
{/snippet}

<svg class="flag" width={size} height={Math.round((size * 2) / 3)} viewBox="0 0 30 20" aria-hidden="true">
  <defs>
    <clipPath id="fl-{uid}"><rect width="30" height="20" rx="3" /></clipPath>
  </defs>
  <g clip-path="url(#fl-{uid})">
    {#if c === "br"}
      <rect width="30" height="20" fill="#229e45" />
      <path d="M15 2.6 27 10 15 17.4 3 10Z" fill="#f8e509" />
      <circle cx="15" cy="10" r="4.6" fill="#2b49a3" />
      <path d="M10.6 9.1c3-.9 6.3-.6 8.9.9" stroke="#fff" stroke-width="1" fill="none" />
    {:else if c === "us"}
      <rect width="30" height="20" fill="#fff" />
      {#each [0, 2, 4, 6, 8, 10, 12] as i (i)}
        <rect y={i * (20 / 13)} width="30" height={20 / 13} fill="#b22234" />
      {/each}
      <rect width="13" height={(20 / 13) * 7} fill="#3c3b6e" />
      {#each [[2.2, 2], [6.5, 2], [10.8, 2], [4.3, 4.7], [8.6, 4.7], [2.2, 7.4], [6.5, 7.4], [10.8, 7.4]] as [x, y] (`${x}-${y}`)}
        <circle cx={x} cy={y} r="0.75" fill="#fff" />
      {/each}
    {:else if c === "es"}
      <rect width="30" height="20" fill="#c60b1e" />
      <rect y="5" width="30" height="10" fill="#ffc400" />
      <rect x="7" y="7.6" width="3.2" height="4.8" rx="0.8" fill="#ad1519" opacity="0.85" />
    {:else if c === "fr"}
      {@render v3(["#0055a4", "#fff", "#ef4135"])}
    {:else if c === "it"}
      {@render v3(["#009246", "#fff", "#ce2b37"])}
    {:else if c === "ro"}
      {@render v3(["#002b7f", "#fcd116", "#ce1126"])}
    {:else if c === "de"}
      {@render h3(["#000", "#dd0000", "#ffce00"])}
    {:else if c === "ru"}
      {@render h3(["#fff", "#0039a6", "#d52b1e"])}
    {:else if c === "nl"}
      {@render h3(["#ae1c28", "#fff", "#21468b"])}
    {:else if c === "hu"}
      {@render h3(["#ce2939", "#fff", "#477050"])}
    {:else if c === "bg"}
      {@render h3(["#fff", "#00966e", "#d62612"])}
    {:else if c === "ir"}
      {@render h3(["#239f40", "#fff", "#da0000"])}
      <circle cx="15" cy="10" r="2" fill="none" stroke="#da0000" stroke-width="0.9" />
    {:else if c === "in"}
      {@render h3(["#ff9933", "#fff", "#138808"])}
      <circle cx="15" cy="10" r="2.4" fill="none" stroke="#000080" stroke-width="0.8" />
      <circle cx="15" cy="10" r="0.6" fill="#000080" />
    {:else if c === "hr"}
      {@render h3(["#ff0000", "#fff", "#171796"])}
      <path d="M12 5.2h6v6.3a3 3 0 0 1-6 0Z" fill="#fff" stroke="#ff0000" stroke-width="0.5" />
      {#each [[12, 5.2], [15, 5.2], [13.5, 6.7], [16.5, 6.7], [12, 8.2], [15, 8.2], [13.5, 9.7], [16.5, 9.7], [12, 11.2], [15, 11.2], [13.5, 12.7]] as [x, y] (`${x}-${y}`)}
        <rect {x} {y} width="1.5" height="1.5" fill="#ff0000" />
      {/each}
    {:else if c === "rs"}
      {@render h3(["#c6363c", "#0c4076", "#fff"])}
      <path d="M7 5h4.4v5.4a2.2 2.2 0 0 1-4.4 0Z" fill="#c6363c" stroke="#fff" stroke-width="0.6" />
      <path d="M9.2 5.8v5.4M7.6 8.1h3.2" stroke="#fff" stroke-width="0.7" />
    {:else if c === "sk"}
      {@render h3(["#fff", "#0b4ea2", "#ee1c25"])}
      <path d="M6.6 4.6h6.2v6.6a3.1 3.1 0 0 1-6.2 0Z" fill="#ee1c25" stroke="#fff" stroke-width="0.7" />
      <path d="M9.7 5.6v7M8.1 7.6h3.2M7.6 9.6h4.2" stroke="#fff" stroke-width="0.8" />
      <path d="M7.3 13.2a2.5 2.5 0 0 1 4.8 0a3 3 0 0 1-4.8 0Z" fill="#0b4ea2" />
    {:else if c === "si"}
      {@render h3(["#fff", "#005da4", "#ed1c24"])}
      <path d="M6.5 4.4h4.4v4.8a2.2 2.2 0 0 1-4.4 0Z" fill="#005da4" stroke="#ed1c24" stroke-width="0.5" />
      <path d="M7 9.6 8.7 7.1l1.7 2.5Z" fill="#fff" />
    {:else if c === "pl"}
      <rect width="30" height="20" fill="#fff" />
      <rect y="10" width="30" height="10" fill="#dc143c" />
    {:else if c === "ua"}
      <rect width="30" height="10" fill="#0057b7" />
      <rect y="10" width="30" height="10" fill="#ffd700" />
    {:else if c === "id"}
      <rect width="30" height="10" fill="#ce1126" />
      <rect y="10" width="30" height="10" fill="#fff" />
    {:else if c === "jp"}
      <rect width="30" height="20" fill="#fff" />
      <circle cx="15" cy="10" r="6" fill="#bc002d" />
    {:else if c === "cn"}
      <rect width="30" height="20" fill="#de2910" />
      <polygon points={star(5.5, 5.5, 3.2)} fill="#ffde00" />
      {#each [[10.5, 2.2], [12.4, 4.2], [12.4, 7], [10.5, 9]] as [x, y] (`${x}-${y}`)}
        <polygon points={star(x, y, 1.05)} fill="#ffde00" />
      {/each}
    {:else if c === "kr"}
      <rect width="30" height="20" fill="#fff" />
      <g transform="rotate(33.7 15 10)">
        <path d="M10 10a5 5 0 0 1 10 0a2.5 2.5 0 0 1-5 0a2.5 2.5 0 0 0-5 0Z" fill="#cd2e3a" />
        <path d="M10 10a5 5 0 0 0 10 0a2.5 2.5 0 0 1-5 0a2.5 2.5 0 0 0-5 0Z" fill="#0047a0" />
      </g>
      {#each [[6, 4.4, -56.3], [24, 4.4, 56.3], [6, 15.6, 56.3], [24, 15.6, -56.3]] as [x, y, a] (`${x}-${y}`)}
        <g transform="translate({x} {y}) rotate({a})" fill="#000">
          <rect x="-2.2" y="-1.9" width="4.4" height="0.9" />
          <rect x="-2.2" y="-0.45" width="4.4" height="0.9" />
          <rect x="-2.2" y="1" width="4.4" height="0.9" />
        </g>
      {/each}
    {:else if c === "tr"}
      <rect width="30" height="20" fill="#e30a17" />
      <circle cx="11.2" cy="10" r="5" fill="#fff" />
      <circle cx="12.5" cy="10" r="4" fill="#e30a17" />
      <polygon points={star(18, 10, 2.3, -Math.PI / 2)} fill="#fff" />
    {:else if c === "sa"}
      <rect width="30" height="20" fill="#006c35" />
      <path d="M8 7.6c1.5-.9 2.4.6 3.6-.3s2.2.7 3.5-.2 2.3.6 3.6-.2 2 .5 3.3-.2" stroke="#fff" stroke-width="1.1" fill="none" />
      <rect x="8" y="9.4" width="13.8" height="0.9" fill="#fff" />
      <path d="M8.6 13.4h12.4l1.2-.6" stroke="#fff" stroke-width="0.8" fill="none" />
    {:else if c === "il"}
      <rect width="30" height="20" fill="#fff" />
      <rect y="2" width="30" height="2.8" fill="#0038b8" />
      <rect y="15.2" width="30" height="2.8" fill="#0038b8" />
      <path d="M15 6.6 18 11.8H12ZM15 13.4 12 8.2H18Z" fill="none" stroke="#0038b8" stroke-width="0.75" />
    {:else if c === "bd"}
      <rect width="30" height="20" fill="#006a4e" />
      <circle cx="13.5" cy="10" r="5.6" fill="#f42a41" />
    {:else if c === "pk"}
      <rect width="30" height="20" fill="#01411c" />
      <rect width="7.5" height="20" fill="#fff" />
      <circle cx="19.2" cy="10.4" r="5" fill="#fff" />
      <circle cx="20.4" cy="9.4" r="4.4" fill="#01411c" />
      <polygon points={star(22.2, 7.6, 1.6, 0.4)} fill="#fff" />
    {:else if c === "my"}
      {#each Array.from({ length: 14 }, (_, i) => i) as i (i)}
        <rect y={(i * 20) / 14} width="30" height={20 / 14 + 0.02} fill={i % 2 ? "#fff" : "#cc0001"} />
      {/each}
      <rect width="15" height={(20 / 14) * 8} fill="#010066" />
      <circle cx="5.6" cy="5.7" r="3.6" fill="#fc0" />
      <circle cx="6.6" cy="5.7" r="3" fill="#010066" />
      <polygon points={star(10.8, 5.7, 2.1)} fill="#fc0" />
    {:else if c === "vn"}
      <rect width="30" height="20" fill="#da251d" />
      <polygon points={star(15, 10.4, 5.4)} fill="#ff0" />
    {:else if c === "th"}
      <rect width="30" height="20" fill="#a51931" />
      <rect y="3.33" width="30" height="13.34" fill="#f4f5f8" />
      <rect y="6.67" width="30" height="6.66" fill="#2d2a4a" />
    {:else if c === "se"}
      {@render nordic("#006aa7", "#fecc00", 9.4, 3.8, 8, 4)}
    {:else if c === "dk"}
      {@render nordic("#c8102e", "#fff", 9.4, 3.6, 8.2, 3.6)}
    {:else if c === "fi"}
      {@render nordic("#fff", "#002f6c", 8.3, 5, 7.3, 5.4)}
    {:else if c === "no"}
      {@render nordic("#ba0c2f", "#fff", 8.2, 5.4, 7.5, 5)}
      <rect x="9.55" width="2.7" height="20" fill="#00205b" />
      <rect y="8.75" width="30" height="2.5" fill="#00205b" />
    {:else if c === "cz"}
      <rect width="30" height="10" fill="#fff" />
      <rect y="10" width="30" height="10" fill="#d7141a" />
      <path d="M0 0 15 10 0 20Z" fill="#11457e" />
    {:else if c === "gr"}
      {#each Array.from({ length: 9 }, (_, i) => i) as i (i)}
        <rect y={(i * 20) / 9} width="30" height={20 / 9 + 0.02} fill={i % 2 ? "#fff" : "#0d5eaf"} />
      {/each}
      <rect width="11.1" height="11.1" fill="#0d5eaf" />
      <rect x="4.44" width="2.22" height="11.1" fill="#fff" />
      <rect y="4.44" width="11.1" height="2.22" fill="#fff" />
    {:else if c === "ct"}
      {#each Array.from({ length: 9 }, (_, i) => i) as i (i)}
        <rect y={(i * 20) / 9} width="30" height={20 / 9 + 0.02} fill={i % 2 ? "#da121a" : "#fcdd09"} />
      {/each}
    {:else if c === "ph"}
      <rect width="30" height="10" fill="#0038a8" />
      <rect y="10" width="30" height="10" fill="#ce1126" />
      <path d="M0 0 17.3 10 0 20Z" fill="#fff" />
      <circle cx="5.8" cy="10" r="2.1" fill="#fcd116" />
      {#each [[1.6, 2.6], [1.6, 17.4], [14, 10]] as [x, y] (`${x}-${y}`)}
        <polygon points={star(x, y, 1)} fill="#fcd116" />
      {/each}
    {:else}
      <rect width="30" height="20" fill="currentColor" opacity="0.14" />
      <text x="15" y="13.6" text-anchor="middle" font-size="9.5" font-weight="600" font-family="ui-monospace, Consolas, monospace" fill="currentColor" opacity="0.8">
        {(code ?? "").slice(0, 2).toUpperCase()}
      </text>
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
