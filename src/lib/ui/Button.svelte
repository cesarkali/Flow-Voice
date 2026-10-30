<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon from "../icons/Icon.svelte";
  import type { IconName } from "../icons/paths";

  type Props = HTMLButtonAttributes & {
    variant?: "default" | "primary" | "ghost" | "danger" | "quiet";
    size?: "sm" | "md" | "lg";
    icon?: IconName;
    iconRight?: IconName;
    loading?: boolean;
    children?: Snippet;
  };

  let {
    variant = "default",
    size = "md",
    icon,
    iconRight,
    loading = false,
    children,
    class: cls = "",
    type = "button",
    disabled,
    ...rest
  }: Props = $props();
  const iconSize = $derived(size === "lg" ? 18 : size === "sm" ? 15 : 16);
</script>

<button
  {type}
  class="btn {variant} {size} {cls}"
  class:icon-only={!children}
  class:loading
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  {...rest}
>
  {#if loading}
    <span class="spin" aria-hidden="true"></span>
  {:else if icon}
    <Icon name={icon} size={iconSize} />
  {/if}
  {#if children}<span class="lbl">{@render children()}</span>{/if}
  {#if iconRight}<Icon name={iconRight} size={iconSize} />{/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 32px;
    padding: 0 12px;
    border-radius: var(--r);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--text);
    transition:
      background-color var(--t-fast) var(--ease),
      border-color var(--t-fast) var(--ease),
      transform 80ms var(--ease);
    flex: none;
  }
  .btn:hover:not(:disabled) {
    background: var(--surface-2);
    border-color: var(--text-3);
  }
  .btn:active:not(:disabled) {
    transform: translateY(0.5px);
  }
  .btn:disabled {
    opacity: 0.45;
  }
  .btn.loading {
    opacity: 0.8;
  }
  .sm {
    height: 27px;
    padding: 0 9px;
    font-size: 12.5px;
    gap: 5px;
    border-radius: var(--r-sm);
  }
  .lg {
    height: 42px;
    padding: 0 18px;
    font-size: 14.5px;
    gap: 9px;
    border-radius: 10px;
  }
  .icon-only {
    width: 32px;
    padding: 0;
  }
  .icon-only.sm {
    width: 27px;
  }
  .icon-only.lg {
    width: 42px;
  }

  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .primary:hover:not(:disabled) {
    background: #e3ff74;
    border-color: #e3ff74;
  }
  .ghost,
  .quiet {
    background: transparent;
    border-color: transparent;
    color: var(--text-2);
  }
  .ghost:hover:not(:disabled),
  .quiet:hover:not(:disabled) {
    background: var(--hover);
    border-color: transparent;
    color: var(--text);
  }
  .quiet {
    color: var(--text-3);
  }
  .danger {
    color: var(--danger-fg);
  }
  .danger:hover:not(:disabled) {
    background: var(--danger-soft);
    border-color: var(--danger-line);
  }

  .spin {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    border-right-color: transparent;
    animation: spin 0.7s linear infinite;
    flex: none;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
