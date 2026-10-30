<script lang="ts" generics="T extends string | null">
  import Icon from "../icons/Icon.svelte";

  let {
    value = $bindable(),
    options,
    label,
    onchange,
    mono = false,
    width,
  }: {
    value: T;
    options: { value: T; label: string; disabled?: boolean }[];
    label: string;
    onchange?: (v: T) => void;
    mono?: boolean;
    width?: string;
  } = $props();

  // <select> values are strings; map null <-> "".
  const enc = (v: T) => (v === null ? "\u0000" : String(v));
</script>

<div class="sel" style:width>
  <select
    aria-label={label}
    class:mono
    value={enc(value)}
    onchange={(e) => {
      const raw = (e.currentTarget as HTMLSelectElement).value;
      const found = options.find((o) => enc(o.value) === raw);
      if (found) {
        value = found.value;
        onchange?.(found.value);
      }
    }}
  >
    {#each options as o (enc(o.value))}
      <option value={enc(o.value)} disabled={o.disabled}>{o.label}</option>
    {/each}
  </select>
  <span class="chev"><Icon name="chevronDown" size={16} /></span>
</div>

<style>
  .sel {
    position: relative;
    display: inline-block;
    min-width: 210px;
    flex: none;
  }
  select {
    appearance: none;
    width: 100%;
    height: 32px;
    padding: 0 32px 0 11px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    text-overflow: ellipsis;
    transition: border-color var(--t-fast) var(--ease);
  }
  select.mono {
    font-family: var(--mono);
    font-size: 12px;
  }
  select:hover {
    border-color: var(--text-3);
  }
  select:focus-visible {
    outline: none;
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  option {
    background: var(--surface);
    color: var(--text);
  }
  .chev {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-3);
    pointer-events: none;
  }
</style>
