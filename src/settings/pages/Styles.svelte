<script lang="ts">
  import { api } from "../../lib/api";
  import { langName, t, TARGET_LANGS } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import type { SettingsPage, Style } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import TagInput from "../../lib/ui/TagInput.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);

  let editing = $state<string | null>(null);
  let confirmReset = $state(false);

  function duplicate(st: Style) {
    const copy: Style = {
      ...$state.snapshot(st),
      id: `custom-${Date.now().toString(36)}`,
      name: `${st.name} ${t("styles.copy_suffix")}`,
      builtin: false,
      kind: "llm",
      prompt: st.prompt || "Reescreva a transcrição de forma clara. Responda só com o texto final.",
    };
    const i = s.styles.findIndex((x) => x.id === st.id);
    s.styles.splice(i + 1, 0, copy);
    editing = copy.id;
  }
  function remove(st: Style) {
    s.styles = s.styles.filter((x) => x.id !== st.id);
    if (s.active_style === st.id) s.active_style = s.styles[0]?.id ?? "natural";
    if (editing === st.id) editing = null;
  }
  async function reset() {
    if (!confirmReset) {
      confirmReset = true;
      setTimeout(() => (confirmReset = false), 3000);
      return;
    }
    confirmReset = false;
    editing = null;
    app.settings = await api.resetStyles();
  }
  const langOptions = $derived(TARGET_LANGS.map((l) => ({ value: l, label: langName(l) })));
</script>

<header class="ph">
  <h1>{t("styles.title")}</h1>
  <p class="muted">{t("styles.subtitle")}</p>
</header>

<Section label={t("nav.styles")}>
  {#snippet aside()}
    <Button size="sm" variant={confirmReset ? "danger" : "ghost"} icon="refresh" onclick={reset}>
      {confirmReset ? t("styles.reset_confirm") : t("styles.reset")}
    </Button>
  {/snippet}
  <ul class="styles" role="radiogroup" aria-label={t("nav.styles")}>
    {#each s.styles as st (st.id)}
      {@const active = s.active_style === st.id}
      {@const open = editing === st.id}
      <li class:active class:open>
        <div class="head">
          <button
            type="button"
            class="pick"
            role="radio"
            aria-checked={active}
            onclick={() => (s.active_style = st.id)}
          >
            <span class="radio" aria-hidden="true"></span>
            <span class="names">
              <span class="name">
                {st.name}
                <Badge tone={st.kind === "llm" ? "accent" : "neutral"}>
                  {st.kind === "llm" ? t("styles.kind_llm") : t("styles.kind_off")}
                </Badge>
                {#if !st.builtin}<span class="faint mono small">{t("styles.custom")}</span>{/if}
              </span>
              <span class="desc">{st.description}</span>
            </span>
          </button>
          <div class="acts">
            {#if active}<span class="on mono">{t("styles.active")}</span>{/if}
            <Button
              size="sm"
              variant="ghost"
              icon={open ? "chevronUp" : "edit"}
              aria-expanded={open}
              aria-label={t("styles.edit")}
              title={t("styles.edit")}
              onclick={() => (editing = open ? null : st.id)}
            />
            <Button size="sm" variant="ghost" icon="copy" aria-label={t("styles.duplicate")} title={t("styles.duplicate")} onclick={() => duplicate(st)} />
            {#if !st.builtin}
              <Button size="sm" variant="ghost" icon="trash" aria-label={t("styles.delete")} title={t("styles.delete")} onclick={() => remove(st)} />
            {/if}
          </div>
        </div>
        {#if open}
          <div class="edit">
            {#if !st.builtin}
              <div class="two">
                <label>
                  <span class="label">{t("styles.name")}</span>
                  <input class="field" bind:value={st.name} />
                </label>
                <label>
                  <span class="label">{t("styles.description")}</span>
                  <input class="field" bind:value={st.description} />
                </label>
              </div>
            {/if}
            {#if st.kind === "llm"}
              <label class="pr">
                <span class="label">{t("styles.prompt")}</span>
                <textarea class="field" rows="5" bind:value={st.prompt} spellcheck="false"></textarea>
                <span class="hint">{t("styles.prompt_hint")}</span>
              </label>
            {:else}
              <p class="nollm"><Icon name="lock" size={15} />{t("styles.no_llm")}</p>
            {/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
</Section>

<Section label={t("styles.sec_translation")}>
  <Row label={t("styles.translation_target")} hint={t("styles.translation_hint")}>
    <Select label={t("styles.translation_target")} bind:value={s.translation_target} options={langOptions} />
  </Row>
</Section>

<Section label={t("styles.sec_cleanup")}>
  <Row label={t("styles.remove_fillers")} hint={t("styles.remove_fillers_hint")}>
    <Toggle label={t("styles.remove_fillers")} bind:checked={s.remove_fillers} />
  </Row>
</Section>

<Section label={t("styles.sec_dictionary")}>
  <Row label={t("styles.sec_dictionary")} hint={t("styles.dictionary_hint")} stack>
    {#snippet below()}
      <TagInput bind:tags={s.dictionary} placeholder={t("styles.dictionary_placeholder")} label={t("styles.sec_dictionary")} />
    {/snippet}
  </Row>
</Section>

<style>
  .ph h1 {
    font-size: 26px;
  }
  .ph p {
    margin-top: 6px;
    max-width: 58ch;
    font-size: 13.5px;
  }
  .styles {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    border-top: 1px solid var(--line);
  }
  li:last-child {
    border-bottom: 1px solid var(--line);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .pick {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 13px 0;
    text-align: left;
  }
  .radio {
    margin-top: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    flex: none;
    display: grid;
    place-items: center;
    transition: border-color var(--t-fast);
  }
  .radio::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    transform: scale(0);
    transition: transform var(--t) var(--spring);
  }
  .pick:hover .radio {
    border-color: var(--text-2);
  }
  .active .radio {
    border-color: var(--accent);
  }
  .active .radio::after {
    transform: scale(1);
  }
  .names {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 500;
  }
  .small {
    font-size: 10.5px;
  }
  .desc {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .on {
    font-size: 10.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--accent-fg);
    margin-right: 8px;
  }
  .edit {
    padding: 0 0 16px 30px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: enter 160ms var(--ease) both;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 10px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  textarea {
    font-size: 13px;
    min-height: 96px;
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  .nollm {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-2);
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
