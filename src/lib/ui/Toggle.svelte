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
    --tg-spring: var(--spring);
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: 20px;
    background: var(--surface-3);
    border: 1px solid var(--line-strong);
    flex: none;
    transition:
      background-color var(--t) var(--ease),
      border-color var(--t) var(--ease),
      box-shadow var(--t) var(--ease);
  }
  @supports (transition-timing-function: linear(0, 1)) {
    .toggle {
      --tg-spring: linear(0, 0.063, 0.206, 0.379, 0.549, 0.698, 0.818, 0.907, 0.968, 1.007, 1.028, 1.037, 1.038, 1.034, 1.028, 1.021, 1.015, 1.01, 1.006, 1.003, 1);
    }
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
    border-radius: 14px;
    background: var(--text-2);
    transform-origin: left center;
    transition:
      transform 360ms var(--tg-spring),
      background-color var(--t) var(--ease);
  }
  /* press: the knob stretches toward where it will go */
  .toggle:active:not(:disabled) .knob {
    transform: scaleX(1.28);
  }
  .on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .on:hover:not(:disabled) {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .on .knob {
    transform: translateX(16px);
    background: var(--on-accent);
    transform-origin: right center;
  }
  .on:active:not(:disabled) .knob {
    transform: translateX(16px) scaleX(1.28);
  }
  .toggle:disabled {
    opacity: 0.4;
  }
  .toggle:focus-visible {
    border-radius: 20px;
  }
</style>
