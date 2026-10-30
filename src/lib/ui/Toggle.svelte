<script lang="ts">
  let {
    checked = $bindable(false),
    label,
    disabled = false,
    onchange,
  }: { checked?: boolean; label: string; disabled?: boolean; onchange?: (v: boolean) => void } = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  class="toggle"
  class:on={checked}
  {disabled}
  onclick={() => {
    checked = !checked;
    onchange?.(checked);
  }}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 34px;
    height: 20px;
    border-radius: 20px;
    background: var(--surface-3);
    border: 1px solid var(--line-strong);
    flex: none;
    transition:
      background-color var(--t) var(--ease),
      border-color var(--t) var(--ease);
  }
  .toggle:hover:not(:disabled) {
    border-color: var(--text-3);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-2);
    transition:
      transform var(--t) var(--ease),
      background-color var(--t) var(--ease);
  }
  .on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .on .knob {
    transform: translateX(14px);
    background: var(--on-accent);
  }
  .toggle:disabled {
    opacity: 0.4;
  }
  .toggle:focus-visible {
    border-radius: 20px;
  }
</style>
