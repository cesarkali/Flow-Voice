<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app, hasKey } from "../../lib/stores.svelte";
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

  const llmReady = (id: string) =>
    id === "ollama" ? true : id === "custom" ? !!s.providers.custom.base_url && !!s.providers.custom.model : hasKey(id as never);
</script>

<header class="ph">
  <h1>{t("prov.title")}</h1>
  <p class="muted">{t("prov.subtitle")}</p>
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
  .ph h1 {
    font-size: 26px;
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
