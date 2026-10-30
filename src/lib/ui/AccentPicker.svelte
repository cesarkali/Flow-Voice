<script lang="ts">
  // Accent color: curated swatches + a free hex field. Applies live through settings.accent_color.
  import { ACCENT_PRESETS, isValidAccent } from "../theme";

  let {
    value = $bindable("#D7FF3F"),
    label,
    customLabel,
    large = false,
  }: { value: string; label: string; customLabel: string; large?: boolean } = $props();

  let draft = $state("");
  $effect(() => {
    draft = value.toUpperCase();
  });
  const norm = (v: string) => {
    let h = v.trim().replace(/^#/, "").toUpperCase();
    if (h.length === 3) h = h.split("").map((c) => c + c).join("");
    return "#" + h;
  };
  const commit = () => {
    if (isValidAccent(draft)) value = norm(draft);
  };
</script>

<div class="ap" class:large role="radiogroup" aria-label={label}>
  {#each ACCENT_PRESETS as c, i (c)}
    <button
      type="button"
      role="radio"
      class="swatch"
      class:on={value.toUpperCase() === c}
      style:--sw={c}
      style:--d="{i * 25}ms"
      aria-checked={value.toUpperCase() === c}
      aria-label={c}
      title={c}
      onclick={() => (value = c)}
    ></button>
  {/each}
  <label class="hex" class:bad={draft !== "" && !isValidAccent(draft)}>
    <span class="dot" style:--sw={isValidAccent(draft) ? norm(draft) : value}></span>
    <input
      type="text"
      maxlength="7"
      spellcheck="false"
      aria-label={customLabel}
      placeholder="#D7FF3F"
      bind:value={draft}
      onchange={commit}
      onkeydown={(e) => e.key === "Enter" && commit()}
    />
  </label>
</div>

<style>
  .ap {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .swatch {
    --s: 22px;
    width: var(--s);
    height: var(--s);
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--sw);
    box-shadow: 0 0 0 1px var(--line-strong);
    cursor: pointer;
    animation: sw-in 360ms var(--spring) both;
    animation-delay: var(--d);
    transition: transform var(--t-fast) var(--spring), box-shadow var(--t) var(--ease);
  }
  .large .swatch {
    --s: 28px;
  }
  .swatch:hover {
    transform: scale(1.14);
  }
  .swatch:active {
    transform: scale(0.9);
  }
  .swatch.on {
    box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--sw);
    transform: scale(1.06);
  }
  .swatch:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 4px;
  }
  .hex {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--line-strong);
    border-radius: var(--r);
    background: var(--surface);
    transition: border-color var(--t-fast) var(--ease);
  }
  .hex:focus-within {
    border-color: var(--accent-line);
  }
  .hex.bad {
    border-color: var(--danger-line);
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--sw);
    box-shadow: 0 0 0 1px var(--line-strong);
    transition: background var(--t) var(--ease);
  }
  input {
    width: 72px;
    border: 0;
    background: none;
    color: var(--text);
    font: 12px var(--mono);
    text-transform: uppercase;
  }
  input:focus {
    outline: none;
  }
  @keyframes sw-in {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .swatch {
      animation: none;
      transition: none;
    }
  }
</style>
