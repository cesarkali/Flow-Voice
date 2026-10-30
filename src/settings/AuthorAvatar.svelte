<script lang="ts">
  // Round author photo with an initials fallback; the photo fades in once decoded.
  // Usage: <AuthorAvatar size={32} />  (ring = accent hairline around it)
  import { AUTHOR, author, loadAuthorAvatar } from "./author.svelte";

  let { size = 32, ring = true }: { size?: number; ring?: boolean } = $props();

  let ready = $state(false);
  let failed = $state(false);
  loadAuthorAvatar();
  $effect(() => {
    author.avatar;
    ready = false;
    failed = false;
  });
</script>

<span class="av" class:ring style:--s="{size}px" aria-hidden="true">
  <span class="ini mono" class:hide={ready}>{AUTHOR.initials}</span>
  {#if author.avatar && !failed}
    <img
      src={author.avatar}
      alt=""
      draggable="false"
      class:ready
      onload={() => (ready = true)}
      onerror={() => (failed = true)}
    />
  {/if}
</span>

<style>
  .av {
    position: relative;
    width: var(--s);
    height: var(--s);
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    overflow: hidden;
    background: linear-gradient(150deg, var(--surface-3), var(--surface));
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .av.ring {
    overflow: visible;
    box-shadow:
      inset 0 0 0 1px var(--line-strong),
      0 0 0 2px var(--rail, var(--bg)),
      0 0 0 3px var(--accent-line);
  }
  .ini {
    font-size: calc(var(--s) * 0.34);
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text);
    transition: opacity var(--t) var(--ease);
  }
  .ini.hide {
    opacity: 0;
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    opacity: 0;
    transform: scale(1.06);
    transition:
      opacity var(--t-slow) var(--ease),
      transform var(--t-page) var(--ease-out);
  }
  img.ready {
    opacity: 1;
    transform: none;
  }
</style>
