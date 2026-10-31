<script lang="ts">
  // Local model choice for onboarding: best model for the speech language preselected, a lighter
  // one beside it, and anything suitable already on disk shown as ready (nothing to download).
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import type { LocalModel, ModelProgress } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Progress from "../../lib/ui/Progress.svelte";
  import { fmtSize, type ModelPicks } from "./models";

  let {
    picks,
    lang,
    progs,
    bg,
    fresh,
    value = $bindable(),
    ondownload,
    oncancel,
    onbackground,
  }: {
    picks: ModelPicks;
    /** Display name of the speech language ("português"). */
    lang: string;
    progs: Record<string, ModelProgress>;
    /** Downloads the user sent to the background. */
    bg: Record<string, boolean>;
    /** Downloads that finished while the wizard was open. */
    fresh: Record<string, boolean>;
    value: string;
    ondownload: (m: LocalModel) => void;
    oncancel: (id: string) => void;
    onbackground: (id: string) => void;
  } = $props();

  const running = (id: string) => {
    const p = progs[id];
    return !!p && !p.done && !p.error;
  };
  const active = $derived(picks.options.find((m) => running(m.id)));
  const sel = $derived(picks.options.find((m) => m.id === value) ?? picks.options[0]);
  const frac = (p?: ModelProgress) => (p && p.total ? p.downloaded / p.total : 0);
  const err = $derived(sel && progs[sel.id]?.error);

  function tag(m: LocalModel): string {
    if (m === picks.top) return t("obx.m_best", { lang });
    if (m === picks.light) return t("obx.m_light");
    return t("obx.m_yours");
  }
</script>

{#snippet meter(n: number, label: string)}
  <span class="meter" title="{label}: {n}/5">
    <span class="ml">{label}</span>
    <span class="dots" aria-label="{n}/5">{#each Array(5) as _, i}<i class:on={i < n}></i>{/each}</span>
  </span>
{/snippet}

<div class="mp">
  <div class="opts" role="radiogroup" aria-label={t("obx.m_choose")}>
    {#each picks.options as m (m.id)}
      <button
        type="button"
        role="radio"
        aria-checked={sel === m}
        class="opt"
        class:sel={sel === m}
        disabled={!!active && active !== m}
        onclick={() => (value = m.id)}
      >
        <span class="top">
          <span class="tag mono">{tag(m)}</span>
          {#if m.installed}<Badge>{t("obx.m_installed")}</Badge>{/if}
          <span class="radio" aria-hidden="true"><Icon name="check" size={11} stroke={2.6} /></span>
        </span>
        <span class="lbl">{m.label}</span>
        <span class="meta mono tnum">{t("obx.m_size", { size: fmtSize(m.size_mb), ram: fmtSize(m.ram_mb) })}</span>
        <span class="meters">
          {@render meter(m.quality, t("obx.m_quality"))}
          {@render meter(m.speed, t("obx.m_speed"))}
        </span>
      </button>
    {/each}
  </div>

  <div class="act">
    {#if active}
      {@const p = progs[active.id]}
      <div class="dl">
        <div class="dlh">
          <span class="dlt">{t("obx.m_downloading", { label: active.label })}</span>
          <span class="mono faint tnum">
            {t("obx.m_of", { a: fmtSize((p?.downloaded ?? 0) / 1048576), b: fmtSize((p?.total ?? 0) / 1048576) })} · {Math.round(frac(p) * 100)}%
          </span>
        </div>
        <Progress value={frac(p)} label={active.label} />
        <div class="dlb">
          <Button size="sm" variant="ghost" icon="x" onclick={() => oncancel(active.id)}>{t("obx.m_cancel")}</Button>
          {#if bg[active.id]}
            <span class="bgon mono"><Icon name="check" size={12} stroke={2.2} />{t("obx.m_bg_on")}</span>
          {:else}
            <Button size="sm" variant="quiet" onclick={() => onbackground(active.id)}>{t("obx.m_bg")}</Button>
          {/if}
        </div>
      </div>
    {:else if sel?.installed}
      <p class="okline" role="status">
        <Icon name="check" size={16} stroke={2.2} />
        {fresh[sel.id] ? t("obx.m_ready", { label: sel.label }) : t("obx.m_have", { label: sel.label })}
      </p>
    {:else if sel}
      <div class="go">
        <Button variant="primary" icon="download" onclick={() => ondownload(sel)}>
          {t("obx.m_download", { label: sel.label, size: fmtSize(sel.size_mb) })}
        </Button>
        {#if err}<span class="err">{t("obx.m_error")}</span>{/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .mp {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .opts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 8px;
  }
  .opt {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px 13px;
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    text-align: left;
    transition:
      transform var(--t) var(--ease),
      border-color var(--t) var(--ease),
      background-color var(--t) var(--ease),
      opacity var(--t) var(--ease);
  }
  .opt:hover:not(:disabled) {
    transform: translate3d(0, -1px, 0);
    border-color: var(--text-3);
  }
  .opt:active:not(:disabled) {
    transform: scale(0.985);
    transition-duration: 60ms;
  }
  .opt:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .opt.sel {
    border-color: var(--accent-line);
    background: linear-gradient(var(--accent-soft), var(--accent-soft)), var(--surface);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 20px;
  }
  .tag {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .sel .tag {
    color: var(--accent-fg);
  }
  .radio {
    margin-left: auto;
    width: 18px;
    height: 18px;
    flex: none;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    display: grid;
    place-items: center;
    color: var(--on-accent);
    transition:
      background-color var(--t) var(--ease),
      border-color var(--t) var(--ease);
  }
  .radio :global(.icon) {
    transform: scale(0);
    transition: transform var(--t-slow) var(--spring);
  }
  .sel .radio {
    background: var(--accent);
    border-color: var(--accent);
  }
  .sel .radio :global(.icon) {
    transform: scale(1);
  }
  .lbl {
    font-weight: 600;
    font-size: 14px;
  }
  .meta {
    font-size: 10.5px;
    color: var(--text-3);
    letter-spacing: 0.03em;
  }
  .meters {
    margin-top: 4px;
    display: flex;
    gap: 14px;
  }
  .meter {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-2);
  }
  .dots {
    display: inline-flex;
    gap: 2px;
  }
  .dots i {
    width: 7px;
    height: 4px;
    border-radius: 2px;
    background: var(--surface-3);
    transition: background-color var(--t) var(--ease);
  }
  .dots i.on {
    background: var(--text-2);
  }
  .sel .dots i.on {
    background: var(--accent-fg);
  }
  .act {
    min-height: 40px;
  }
  .go {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    animation: fv-fade var(--t-slow) var(--ease) both;
  }
  .dl {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 520px;
    animation: fv-fade var(--t-slow) var(--ease) both;
  }
  .dlh {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    font-size: 12.5px;
  }
  .dlh .faint {
    font-size: 11px;
  }
  .dlb {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: -8px;
  }
  .bgon {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-2);
  }
  .okline {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ok);
    font-size: 13.5px;
    animation: fv-pop var(--t-slow) var(--spring) both;
  }
  .err {
    color: var(--danger-fg);
    font-size: 12px;
  }
</style>
