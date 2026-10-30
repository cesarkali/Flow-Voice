<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../../lib/api";
  import { t, type TKey } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import Logo from "../../lib/icons/Logo.svelte";
  import type { IconName } from "../../lib/icons/paths";
  import type { AppInfo, SettingsPage } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  let info = $state<AppInfo | null>(null);
  let copied = $state(false);
  onMount(() => {
    api.appInfo().then((i) => (info = i));
  });

  const LINKS: { icon: IconName; key: TKey; url: string }[] = [
    { icon: "github", key: "about.profile", url: "https://github.com/cesarkali" },
    { icon: "github", key: "about.repo", url: "https://github.com/cesarkali/Flow-Voice" },
    { icon: "globe", key: "about.site", url: "https://caliberda.com.br" },
    { icon: "instagram", key: "about.instagram", url: "https://www.instagram.com/cesar.kali/" },
  ];
  async function copyPath() {
    if (!info) return;
    await api.copyText(info.data_dir);
    copied = true;
    setTimeout(() => (copied = false), 1400);
  }
</script>

<div class="about">
  <div class="mark">
    <Logo size={88} />
  </div>
  <h1>FlowVoice <span class="v">2.0</span></h1>
  <p class="muted tag">{t("about.tagline")}</p>
  <p class="by">{t("about.made_by")}</p>

  <ul class="links">
    {#each LINKS as l}
      <li>
        <button type="button" onclick={() => api.openUrl(l.url)}>
          <Icon name={l.icon} size={18} />
          <span class="ln">{t(l.key)}</span>
          <span class="mono url">{l.url.replace(/^https?:\/\/(www\.)?/, "")}</span>
          <Icon name="external" size={15} />
        </button>
      </li>
    {/each}
  </ul>

  <dl class="facts">
    <div>
      <dt class="label">{t("about.version")}</dt>
      <dd class="mono">{info?.version ?? "…"}{info ? ` · ${info.os}` : ""}</dd>
    </div>
    <div>
      <dt class="label">{t("about.data_dir")}</dt>
      <dd class="path">
        <span class="mono">{info?.data_dir ?? "…"}</span>
        <Button size="sm" variant="ghost" icon={copied ? "check" : "copy"} aria-label={t("about.copy_path")} title={t("about.copy_path")} onclick={copyPath} />
      </dd>
    </div>
  </dl>
  <p class="lic faint">{t("about.license")} © 2026 Júlio Caliberda</p>
</div>

<style>
  .about {
    padding-top: 18px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .mark {
    filter: drop-shadow(0 10px 24px rgba(0, 0, 0, 0.35));
    margin-bottom: 22px;
  }
  h1 {
    font-size: 34px;
    letter-spacing: -0.035em;
  }
  .v {
    color: var(--accent-fg);
    font-weight: 500;
  }
  .tag {
    margin-top: 4px;
  }
  .by {
    margin-top: 14px;
    font-size: 13.5px;
  }
  .links {
    list-style: none;
    margin: 28px 0 0;
    padding: 0;
    width: 100%;
  }
  .links li {
    border-top: 1px solid var(--line);
  }
  .links li:last-child {
    border-bottom: 1px solid var(--line);
  }
  .links button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 44px;
    padding: 0 4px;
    text-align: left;
    color: var(--text-2);
    border-radius: 0;
  }
  .links button:hover {
    color: var(--text);
    background: var(--hover);
  }
  .ln {
    color: var(--text);
    font-size: 13.5px;
    min-width: 130px;
  }
  .url {
    flex: 1;
    font-size: 11.5px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .facts {
    margin: 26px 0 0;
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 24px;
  }
  dd {
    margin: 6px 0 0;
    font-size: 12px;
  }
  .path {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .path span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .lic {
    margin-top: 30px;
    font-size: 12px;
  }
</style>
