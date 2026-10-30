<script lang="ts">
  import type { Snippet } from "svelte";
  import { isTauri } from "../api";
  import { t } from "../i18n.svelte";
  import Icon from "../icons/Icon.svelte";

  let {
    title = "",
    maximizable = true,
    left,
    right,
  }: { title?: string; maximizable?: boolean; left?: Snippet; right?: Snippet } = $props();

  async function win() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow();
  }
  const minimize = async () => isTauri && (await win()).minimize();
  const maximize = async () => isTauri && (await win()).toggleMaximize();
  // The app lives in the tray: closing only hides the window.
  const close = async () => isTauri && (await win()).hide();
</script>

<header class="tb" data-tauri-drag-region>
  <div class="l" data-tauri-drag-region>
    {@render left?.()}
    {#if title}<span class="title" data-tauri-drag-region>{title}</span>{/if}
  </div>
  <div class="r">
    {@render right?.()}
    <div class="wc">
      <button type="button" aria-label={t("c.minimize")} title={t("c.minimize")} onclick={minimize}>
        <Icon name="minus" size={16} />
      </button>
      {#if maximizable}
        <button type="button" aria-label={t("c.maximize")} title={t("c.maximize")} onclick={maximize}>
          <Icon name="square" size={14} />
        </button>
      {/if}
      <button type="button" class="close" aria-label={t("c.close")} title={t("c.close")} onclick={close}>
        <Icon name="x" size={16} />
      </button>
    </div>
  </div>
</header>

<style>
  .tb {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 14px;
    flex: none;
    position: relative;
    z-index: 5;
  }
  .l,
  .r {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 100%;
    min-width: 0;
  }
  .l {
    flex: 1;
  }
  .title {
    font-size: 12.5px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .wc {
    display: flex;
    height: 100%;
  }
  .wc button {
    width: 44px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-2);
    border-radius: 0;
  }
  .wc button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .wc .close:hover {
    background: #e5483d;
    color: #fff;
  }
  .wc button:focus-visible {
    outline-offset: -2px;
  }
</style>
