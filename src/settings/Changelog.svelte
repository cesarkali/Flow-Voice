<script lang="ts">
  // Every version's changes (CHANGELOG.md, bundled with the app, so it works offline), and the
  // site's "Novidades" page in the interface's language.
  import { api } from "../lib/api";
  import { i18n, t } from "../lib/i18n.svelte";
  import Button from "../lib/ui/Button.svelte";
  import Modal from "../lib/ui/Modal.svelte";
  import ReleaseNotes from "../lib/ui/ReleaseNotes.svelte";
  import { changelogHistory } from "../lib/whatsnew";

  let { open = $bindable(false) }: { open: boolean } = $props();
  const md = changelogHistory();
  const SITE = "https://flowvoice.caliberda.com.br";
  const siteUrl = $derived(i18n.lang === "pt" ? `${SITE}/novidades.html` : `${SITE}/${i18n.lang}/novidades.html`);
</script>

<Modal bind:open title={t("wn.history")} width="620px">
  {#if i18n.lang !== "pt"}<p class="note">{t("wn.pt_only")}</p>{/if}
  <ReleaseNotes {md} max={480} />
  {#snippet footer()}
    <Button variant="quiet" icon="globe" onclick={() => api.openUrl(siteUrl).catch(() => {})}>{t("wn.site")}</Button>
  {/snippet}
</Modal>

<style>
  .note {
    margin: -2px 0 12px;
    font-size: 12.5px;
    color: var(--text-3);
  }
</style>
