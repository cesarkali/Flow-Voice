<script lang="ts">
  // Shown once after an update (the first time this window opens on a new version): the
  // version's highlights in the interface's language, or its CHANGELOG section. Closing it marks
  // the version as seen; a new install starts as seen (see lib.rs).
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { i18n, t } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import { app } from "../lib/stores.svelte";
  import Button from "../lib/ui/Button.svelte";
  import Modal from "../lib/ui/Modal.svelte";
  import ReleaseNotes from "../lib/ui/ReleaseNotes.svelte";
  import { changelogSince, compareVersions, highlightsSince } from "../lib/whatsnew";
  import Changelog from "./Changelog.svelte";

  let version = $state("");
  let open = $state(false);
  let historyOpen = $state(false);
  onMount(() => {
    api.appInfo().then((i) => (version = i.version)).catch(() => {});
  });

  /** The version seen before this one (read once, when deciding: closing the modal changes it). */
  let since = $state<string | null>(null);
  // Everything since that version: a 2.1 user updating straight to a 2.2.x hotfix still sees 2.2.0.
  const items = $derived(version && since !== null ? highlightsSince(version, since, i18n.lang) : null);
  const section = $derived(version && since !== null && !items ? changelogSince(version, since) : "");

  // Once per version: only when it's newer than the one last seen, and there's something to say.
  let decided = false;
  $effect(() => {
    const s = app.settings;
    if (decided || !s || !s.onboarded || !version) return;
    decided = true;
    since = s.seen_version || "0";
    if (compareVersions(version, since) > 0 && (items || section)) open = true;
  });

  function seen() {
    open = false;
    api.whatsNewSeen().catch(() => {});
  }
</script>

<Modal bind:open title={t("wn.title", { v: version.split(".").slice(0, 2).join(".") })} width="560px" onclose={seen}>
  <p class="lede">{t("wn.lede")}</p>
  {#if items}
    <ul class="hl">
      {#each items as it, i (i)}
        <li style:--i={i}>
          <span class="ico"><Icon name={it.icon} size={16} /></span>
          <span>
            <b>{it.title}</b>
            <span class="tx">{it.text}</span>
          </span>
        </li>
      {/each}
    </ul>
  {:else}
    {#if i18n.lang !== "pt"}<p class="note">{t("wn.pt_only")}</p>{/if}
    <ReleaseNotes md={section} max={360} />
  {/if}
  {#snippet footer()}
    <Button
      variant="quiet"
      icon="history"
      onclick={() => {
        seen();
        historyOpen = true;
      }}
    >{t("wn.all")}</Button>
    <Button variant="primary" onclick={seen}>{t("wn.ok")}</Button>
  {/snippet}
</Modal>

<Changelog bind:open={historyOpen} />

<style>
  .note {
    margin: -6px 0 12px;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .lede {
    margin: -2px 0 14px;
    color: var(--text-2);
    font-size: 13.5px;
  }
  .hl {
    margin: 0;
    padding: 0 4px 0 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: min(420px, 60vh);
    overflow: auto;
  }
  li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    animation: rise 420ms var(--ease-out, cubic-bezier(0.16, 1, 0.3, 1)) backwards;
    animation-delay: calc(var(--i) * 45ms + 120ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  .ico {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 9px;
    background: var(--surface-2);
    color: var(--accent-fg);
  }
  b {
    display: block;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text);
  }
  .tx {
    display: block;
    margin-top: 2px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }
  @media (prefers-reduced-motion: reduce) {
    li {
      animation: none;
    }
  }
</style>
