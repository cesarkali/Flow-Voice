<script lang="ts">
  import { onMount } from "svelte";
  import { slide } from "svelte/transition";
  import { api } from "../../lib/api";
  import { t, type TKey } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import Logo from "../../lib/icons/Logo.svelte";
  import type { IconName } from "../../lib/icons/paths";
  import type { AppInfo, SettingsPage } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import Progress from "../../lib/ui/Progress.svelte";
  import { restartOnboarding } from "../../lib/stores.svelte";
  import { author, AUTHOR as ME, loadAuthorAvatar } from "../author.svelte";
  import { checkUpdate, donateOptions, installUpdate, upd, updateFraction } from "../shell-update.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  let info = $state<AppInfo | null>(null);
  let copied = $state(false);
  let imgReady = $state(false);
  let redoing = $state(false);
  onMount(() => {
    api.appInfo().then((i) => (info = i));
    loadAuthorAvatar();
  });
  async function redo() {
    redoing = true;
    await restartOnboarding();
    redoing = false;
  }

  const REPO = "https://github.com/cesarkali/Flow-Voice";
  type Link = { icon: IconName; key: TKey; url: string };
  const AUTHOR: Link[] = [
    { icon: "github", key: "about.profile", url: "https://github.com/cesarkali" },
    { icon: "globe", key: "about.site", url: "https://caliberda.com.br" },
    { icon: "instagram", key: "about.instagram", url: "https://www.instagram.com/cesar.kali/" },
  ];
  const host = (u: string) => u.replace(/^https?:\/\/(www\.)?/, "").replace(/\/$/, "");

  async function copyPath() {
    if (!info) return;
    await api.copyText(info.data_dir);
    copied = true;
    setTimeout(() => (copied = false), 1400);
  }

  const frac = $derived(updateFraction());
  const pct = $derived(Math.round((frac ?? 0) * 100));

  // ---------------------------------------------------------------- feedback (GitHub issues)
  let withLog = $state(true);
  let logPreview = $state<string | null>(null);
  async function toggleLog() {
    logPreview = logPreview === null ? await api.feedbackLog() : null;
  }
</script>

<header class="hero">
  <div class="mark">
    <Logo size={88} animate="loop" />
  </div>
  <div class="ht">
    <h1>FlowVoice <span class="v">2.0</span></h1>
    <p class="tag">{t("about.tagline")}</p>
  </div>
</header>

<p class="story">{t("sh.about_story")}</p>

<section class="support">
  <span class="sheart" aria-hidden="true"><Icon name="heart" size={20} /></span>
  <div class="stext">
    <h2>{t("sh.about_support_title")}</h2>
    <p>{t("sh.about_support_body")}</p>
  </div>
  <div class="sact">
    {#each donateOptions() as d, i (d.id)}
      <Button
        variant={i === 0 ? "primary" : "default"}
        icon={d.id === "pix" ? "heart" : "github"}
        title={t(d.key)}
        onclick={() => api.openUrl(d.url)}
      >
        {d.id === "pix" ? t("sh.about_pix_cta") : t("sh.about_support_cta")}
      </Button>
    {/each}
    <button type="button" class="star" onclick={() => api.openUrl(REPO)}>
      <Icon name="github" size={14} />
      <span>{t("sh.about_star")}</span>
    </button>
  </div>
</section>

<div class="grid-2 cols">
  <section class="card blk">
    <h2 class="label">{t("sh.about_author")}</h2>
    <div class="person">
      <span class="av mono" class:has={imgReady}>
        <span class="ini" aria-hidden="true">{ME.initials}</span>
        {#if author.avatar}
          <img src={author.avatar} alt={t("obx.author_photo")} draggable="false" onload={() => (imgReady = true)} />
        {/if}
      </span>
      <div>
        <p class="pn">{ME.name}</p>
        <p class="ph mono">@{ME.handle}</p>
      </div>
    </div>
    <ul class="links">
      {#each AUTHOR as l}
        <li>
          <button type="button" onclick={() => api.openUrl(l.url)}>
            <Icon name={l.icon} size={16} />
            <span class="ln">{t(l.key)}</span>
            <span class="mono url">{host(l.url)}</span>
            <span class="go"><Icon name="external" size={13} /></span>
          </button>
        </li>
      {/each}
    </ul>
  </section>

  <section class="card blk">
    <h2 class="label">{t("sh.about_project")}</h2>
    <ul class="links">
      <li>
        <button type="button" onclick={() => api.openUrl(REPO)}>
          <Icon name="github" size={16} />
          <span class="ln">{t("about.repo")}</span>
          <span class="mono url">{REPO.replace("https://github.com/", "")}</span>
          <span class="go"><Icon name="external" size={13} /></span>
        </button>
      </li>
    </ul>
    <div class="redo">
      <div>
        <p class="rt">{t("obx.redo")}</p>
        <p class="rh">{t("obx.redo_hint")}</p>
      </div>
      <Button size="sm" icon="refresh" loading={redoing} onclick={redo}>{t("obx.redo_btn")}</Button>
    </div>
    <dl class="facts">
      <div>
        <dt class="label">{t("about.version")}</dt>
        <!-- just "Windows 11" here; the full version string (edition, build) goes into bug reports -->
        <dd class="mono" title={info?.os}>{info?.version ?? "…"}{info ? ` · ${info.os.split(" ").slice(0, 2).join(" ")}` : ""}</dd>
      </div>
      <div>
        <dt class="label">{t("sh.about_open_source")}</dt>
        <dd class="mono">GPL-3.0</dd>
      </div>
      <div class="wide">
        <dt class="label">{t("about.data_dir")}</dt>
        <dd class="path">
          <span class="mono" title={info?.data_dir}>{info?.data_dir ?? "…"}</span>
          <Button
            size="sm"
            variant="ghost"
            icon={copied ? "check" : "copy"}
            aria-label={copied ? t("sh.copied") : t("about.copy_path")}
            title={copied ? t("sh.copied") : t("about.copy_path")}
            onclick={copyPath}
          />
        </dd>
      </div>
    </dl>
  </section>

  <section class="card blk span-all fbcard" id="feedback">
    <div class="fb-top">
      <div class="fb-intro">
        <h2 class="label">{t("fb.title")}</h2>
        <p class="rh">{t("fb.hint")}</p>
      </div>
      <div class="fb-acts">
        <Button size="sm" icon="alert" onclick={() => api.openFeedback("bug", undefined, withLog)}>{t("fb.bug")}</Button>
        <Button size="sm" variant="ghost" icon="sparkle" onclick={() => api.openFeedback("idea")}>{t("fb.idea")}</Button>
      </div>
    </div>
    <div class="fb-logrow">
      <label class="fb-log">
        <input type="checkbox" bind:checked={withLog} />
        <span>
          <span class="fl-t">{t("fb.log")}</span>
          <span class="rh">{t("fb.log_hint")}</span>
        </span>
      </label>
      {#if withLog}
        <button type="button" class="fl-show" aria-expanded={logPreview !== null} onclick={toggleLog}>
          {logPreview !== null ? t("fb.log_hide") : t("fb.log_show")}
        </button>
      {/if}
    </div>
    {#if withLog && logPreview !== null}
      <pre class="fl-pre mono">{logPreview || t("fb.log_empty")}</pre>
    {/if}
  </section>

  <section class="card blk span-all upd" class:avail={upd.info?.available}>
    <div class="uhead">
      <div>
        <h2 class="label">{t("gen.sec_updates")}</h2>
        <p class="ustate">
          {#if upd.error}
            <span class="err">{t("gen.update_error", { e: upd.error })}</span>
          {:else if upd.installing}
            {frac === null ? t("gen.installing", { pct: "…" }) : t("gen.installing", { pct })}
          {:else if upd.info?.available && upd.info.version}
            <span class="newv">{t("gen.available", { v: upd.info.version })}</span>
          {:else if upd.checked}
            {t("gen.up_to_date")}
          {:else}
            {t("gen.current", { v: info?.version ?? "…" })}
          {/if}
        </p>
      </div>
      {#if upd.info?.available && !upd.installing}
        <Button variant="primary" icon="download" onclick={installUpdate}>{t("gen.install")}</Button>
      {:else if !upd.installing}
        <Button icon="refresh" loading={upd.checking} onclick={checkUpdate}>
          {upd.checking ? t("gen.checking") : t("gen.check")}
        </Button>
      {/if}
    </div>
    {#if upd.installing}
      <div class="prog" transition:slide={{ duration: 200 }}>
        <Progress value={frac ?? 0.05} label={t("gen.installing", { pct })} />
      </div>
    {/if}
    {#if upd.info?.available && upd.info.notes}
      <div class="notes" transition:slide={{ duration: 220 }}>
        <span class="label">{t("sh.about_notes")}</span>
        <p>{upd.info.notes}</p>
      </div>
    {/if}
  </section>
</div>

<footer class="foot faint">
  <span>{t("about.license")}</span>
  <span>© 2026 Júlio Caliberda · {t("sh.about_built")}</span>
</footer>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 22px;
    padding-top: 18px;
  }
  .mark {
    border-radius: 18px;
    box-shadow:
      0 14px 30px rgba(0, 0, 0, 0.35),
      0 0 0 1px var(--line);
    transition: transform 520ms var(--spring);
  }
  .mark:hover {
    transform: rotate(-5deg) scale(1.04);
  }
  h1 {
    font-size: 36px;
    line-height: 1.05;
    letter-spacing: -0.04em;
  }
  .v {
    color: var(--accent-fg);
    font-weight: 500;
  }
  .tag {
    margin-top: 6px;
    color: var(--text-2);
    font-size: 14px;
  }
  .story {
    margin-top: 22px;
    max-width: 62ch;
    font-size: 15px;
    line-height: 1.6;
    color: var(--text);
    text-wrap: pretty;
  }

  /* donation band: ink panel with one lime stroke, quiet until hovered */
  .support {
    position: relative;
    margin-top: 26px;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 16px 18px;
    padding: 18px 20px;
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px solid var(--accent-line);
    overflow: hidden;
  }
  .support::before {
    content: "";
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: var(--accent);
  }
  .sheart {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--accent-fg);
    background: var(--accent-soft);
    transition: transform 420ms var(--spring);
  }
  .support:hover .sheart {
    transform: scale(1.1) rotate(-8deg);
  }
  .stext h2 {
    font-size: 15px;
    letter-spacing: -0.015em;
  }
  .stext p {
    margin-top: 3px;
    font-size: 13px;
    color: var(--text-2);
    max-width: 52ch;
  }
  .sact {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }
  .star {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 24px;
    font-size: 11.5px;
    color: var(--text-3);
    border-radius: var(--r-sm);
    transition: color var(--t-fast) var(--ease);
  }
  .star:hover {
    color: var(--text);
  }
  @container content (max-width: 640px) {
    .support {
      grid-template-columns: auto 1fr;
    }
    .sact {
      grid-column: 1 / -1;
      flex-direction: row;
      align-items: center;
    }
  }

  .cols {
    margin-top: 16px;
    /* side-by-side cards share one height */
    align-items: stretch;
    gap: 14px;
  }
  .blk {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .person {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .av {
    position: relative;
    overflow: hidden;
    flex: none;
    width: 52px;
    height: 52px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 13px;
    font-weight: 600;
    background: linear-gradient(150deg, var(--surface-3), var(--surface-2));
    box-shadow:
      inset 0 0 0 1px var(--line-strong),
      0 0 0 2px var(--surface),
      0 0 0 3px var(--accent-line);
  }
  .av img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
    opacity: 0;
    transform: scale(1.08);
    transition:
      opacity 420ms var(--ease),
      transform 620ms var(--ease-out);
  }
  .av.has img {
    opacity: 1;
    transform: none;
  }
  .ini {
    transition: opacity var(--t) var(--ease);
  }
  .av.has .ini {
    opacity: 0;
  }
  .person:hover .av.has img {
    transform: scale(1.06);
  }
  .redo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 0;
    margin-bottom: 12px;
    border-bottom: 1px solid var(--line);
  }
  .rt {
    font-size: 13px;
    font-weight: 600;
  }
  .rh {
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-2);
    max-width: 44ch;
  }
  .pn {
    font-weight: 600;
    font-size: 14px;
  }
  .ph {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .links {
    list-style: none;
    margin: 0 -8px;
    padding: 0;
  }
  .links button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 8px;
    border-radius: var(--r);
    text-align: left;
    color: var(--text-2);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }
  .links button:hover {
    color: var(--text);
    background: var(--hover);
  }
  .ln {
    color: var(--text);
    font-size: 13px;
    white-space: nowrap;
  }
  .url {
    flex: 1;
    min-width: 0;
    text-align: right;
    font-size: 11px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .go {
    display: grid;
    color: var(--text-3);
    transition: transform var(--t) var(--spring);
  }
  .links button:hover .go {
    color: var(--accent-fg);
    transform: translate(2px, -2px);
  }
  .facts {
    margin: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .facts .wide {
    grid-column: 1 / -1;
  }
  dd {
    margin: 4px 0 0;
    font-size: 12px;
  }
  .path {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    margin-top: 2px;
  }
  .path span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
    color: var(--text-2);
  }

  .upd.avail {
    border-color: var(--accent-line);
  }
  .uhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .ustate {
    margin-top: 5px;
    font-size: 13.5px;
  }
  .newv {
    color: var(--accent-fg);
    font-weight: 600;
  }
  .err {
    color: var(--danger-fg);
    font-family: var(--mono);
    font-size: 12px;
  }
  .notes {
    padding-top: 12px;
    border-top: 1px solid var(--line);
    font-size: 13px;
    color: var(--text-2);
  }
  .notes p {
    margin-top: 4px;
  }

  .foot {
    margin-top: 28px;
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 6px 16px;
    font-size: 11.5px;
  }
  .fbcard {
    display: grid;
    gap: 14px;
  }
  .fb-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px 24px;
    flex-wrap: wrap;
  }
  .fb-intro {
    display: grid;
    gap: 6px;
    max-width: 520px;
  }
  .fb-acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .fb-logrow {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px 24px;
    flex-wrap: wrap;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .fb-logrow .fb-log {
    max-width: 640px;
    margin-top: 0;
  }
  .fb-log {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin-top: 6px;
    cursor: pointer;
  }
  .fb-log input {
    margin-top: 3px;
    accent-color: var(--accent);
  }
  .fb-log > span {
    display: grid;
    gap: 2px;
  }
  .fl-t {
    font-size: 13px;
    color: var(--text);
  }
  .fl-show {
    flex: none;
    margin-top: 2px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-fg, var(--text-2));
    font: inherit;
    font-size: 12.5px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .fl-pre {
    max-height: 180px;
    overflow: auto;
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--bg-2, rgba(127, 127, 127, 0.06));
    font-size: 11px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-2);
  }
</style>
