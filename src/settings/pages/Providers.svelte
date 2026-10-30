<script lang="ts">
  import { t, type TKey } from "../../lib/i18n.svelte";
  import { tick } from "svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import ProviderLogo from "../../lib/icons/ProviderLogo.svelte";
  import { app, hasKey, usableLlm } from "../../lib/stores.svelte";
  import type { SettingsPage } from "../../lib/types";
  import Section from "../../lib/ui/Section.svelte";
  import { pname, PROVIDERS } from "../providers";
  import OrderList from "./OrderList.svelte";
  import ProviderCard from "./ProviderCard.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);

  // Open Groq by default when nothing is configured yet.
  const nothing = !PROVIDERS.some((p) => p.keyed && p.id !== "tavily" && hasKey(p.id as never));
  let open = $state<Record<string, boolean>>({ groq: nothing });

  const ai = PROVIDERS.filter((p) => p.id !== "tavily");
  const search = PROVIDERS.filter((p) => p.id === "tavily");

  // Header: logos of the providers that are working right now.
  const activeAi = $derived.by(() => {
    const ids: string[] = [...usableLlm()];
    for (const p of s.stt.cloud_order) if (hasKey(p) && !ids.includes(p)) ids.push(p);
    if (s.stt.engine === "local") ids.push("local");
    return ids;
  });
  const activeSearch = $derived(hasKey("tavily" as never) ? ["tavily"] : []);

  const groups = $derived<{ key: TKey; ids: string[] }[]>([
    { key: "prov.sec_ai", ids: activeAi },
    { key: "prov.sec_search", ids: activeSearch },
  ]);
  const nameOf = (id: string) => (id === "local" ? t("speech.local_title") : pname(id));

  async function reveal(id: string) {
    if (id === "local") return;
    open[id] = true;
    await tick();
    document.getElementById(`prov-${id}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  const llmReady = (id: string) =>
    id === "ollama" ? true : id === "custom" ? !!s.providers.custom.base_url && !!s.providers.custom.model : hasKey(id as never);
</script>

<header class="ph">
  <div class="pht">
    <h1>{t("prov.title")}</h1>
    <p class="muted">{t("prov.subtitle")}</p>
  </div>
  <div class="active" aria-label={t("obx.prov_active")}>
    {#each groups as g (g.key)}
      <div class="grp">
        <span class="label">{t(g.key)}</span>
        <span class="stack">
          {#each g.ids as id, i (id)}
            <button
              type="button"
              class="lgb"
              style:--i={i}
              style:z-index={20 - i}
              title={nameOf(id)}
              aria-label={nameOf(id)}
              onclick={() => reveal(id)}
            >
              <ProviderLogo {id} size={26} />
            </button>
          {:else}
            <span class="none mono">{t("home.none")}</span>
          {/each}
        </span>
      </div>
    {/each}
  </div>
</header>

<p class="notice"><Icon name="info" size={15} />{t("prov.github_notice")}</p>

<Section label={t("prov.sec_ai")}>
  <div class="cards stagger">
    {#each ai as info (info.id)}
      <ProviderCard {info} bind:open={() => open[info.id] ?? false, (v) => (open[info.id] = v)} />
    {/each}
  </div>
</Section>

<Section label={t("prov.sec_search")}>
  <div class="cards stagger">
    {#each search as info (info.id)}
      <ProviderCard {info} bind:open={() => open[info.id] ?? false, (v) => (open[info.id] = v)} />
    {/each}
  </div>
</Section>

<Section label={t("prov.order_title")}>
  <p class="hint">{t("prov.order_hint")} <span class="faint">· {t("obx.drag_hint")}</span></p>
  <div class="orders">
    <div>
      <h3 class="sub">{t("prov.order_llm")}</h3>
      <OrderList bind:items={s.llm_order} label={t("prov.order_llm")} nameOf={pname} ready={llmReady} />
    </div>
    <div>
      <h3 class="sub">{t("prov.order_stt")}</h3>
      <OrderList bind:items={s.stt.cloud_order} label={t("prov.order_stt")} nameOf={pname} ready={(id) => hasKey(id)} />
    </div>
  </div>
</Section>

<style>
  .ph {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    flex-wrap: wrap;
  }
  .pht {
    flex: 1 1 320px;
    min-width: 0;
  }
  .ph h1 {
    font-size: 26px;
  }
  .active {
    display: flex;
    gap: 18px;
    flex: none;
  }
  .grp {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .grp .label {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.1em;
  }
  .stack {
    display: flex;
    align-items: center;
    min-height: 30px;
    padding-left: 6px;
  }
  .lgb {
    position: relative;
    margin-left: -6px;
    padding: 0;
    border-radius: 8px;
    box-shadow: 0 0 0 2px var(--bg);
    transition: transform var(--t) var(--spring);
    animation: lg-in 420ms var(--spring) both;
    animation-delay: calc(var(--i) * 50ms);
  }
  .lgb:hover,
  .lgb:focus-visible {
    transform: translate3d(0, -3px, 0) rotate(-6deg);
    z-index: 30 !important;
  }
  .lgb:active {
    transform: scale(0.94);
    transition-duration: 60ms;
  }
  .none {
    margin-left: -6px; /* cancels the stack's overlap padding, so it lines up with the label */
    font-size: 11px;
    color: var(--text-3);
  }
  @keyframes lg-in {
    from {
      opacity: 0;
      transform: translate3d(-8px, 0, 0) scale(0.8);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .lgb {
      animation: none;
    }
  }
  .ph p {
    margin-top: 6px;
    max-width: 60ch;
    font-size: 13.5px;
  }
  .notice {
    margin-top: 18px;
    display: flex;
    gap: 9px;
    align-items: flex-start;
    font-size: 12.5px;
    color: var(--text-2);
    padding: 10px 12px;
    border-radius: var(--r);
    border: 1px dashed var(--line-strong);
  }
  .notice :global(.icon) {
    margin-top: 1px;
    flex: none;
  }
  .cards {
    display: flex;
    flex-direction: column;
  }
  .hint {
    font-size: 12.5px;
    color: var(--text-2);
    margin-bottom: 14px;
  }
  .orders {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 20px;
  }
  .sub {
    font-family: var(--font);
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0;
    margin-bottom: 8px;
  }
</style>
