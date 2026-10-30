<script lang="ts">
  import Segmented from "../../lib/ui/Segmented.svelte";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "svelte/motion";
  import { fly } from "svelte/transition";
  import { langName, t, TARGET_LANGS, type TKey } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import { app } from "../../lib/stores.svelte";
  import type { SettingsPage, Style } from "../../lib/types";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Modal from "../../lib/ui/Modal.svelte";
  import Row from "../../lib/ui/Row.svelte";
  import Section from "../../lib/ui/Section.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import TagInput from "../../lib/ui/TagInput.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import AppStyles from "./AppStyles.svelte";

  let { onnavigate: _ }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);

  const builtins = $derived(s.styles.filter((x) => x.builtin));
  const customs = $derived(s.styles.filter((x) => !x.builtin));

  // ------------------------------------------------------------ create / edit modal
  let formOpen = $state(false);
  let editingId = $state<string | null>(null);
  let draft = $state({ name: "", description: "", prompt: "" });
  let promptEl: HTMLTextAreaElement | undefined = $state();
  const canSave = $derived(draft.name.trim().length > 0 && draft.prompt.trim().length > 0);

  const EXAMPLES: { label: TKey; text: TKey }[] = [
    { label: "home.sty_ex1_l", text: "home.sty_ex1" },
    { label: "home.sty_ex2_l", text: "home.sty_ex2" },
    { label: "home.sty_ex3_l", text: "home.sty_ex3" },
    { label: "home.sty_ex4_l", text: "home.sty_ex4" },
  ];

  function openCreate() {
    editingId = null;
    draft = { name: "", description: "", prompt: "" };
    formOpen = true;
  }
  function openEdit(st: Style) {
    editingId = st.id;
    draft = { name: st.name, description: st.description, prompt: st.prompt };
    formOpen = true;
  }
  function useExample(ex: { label: TKey; text: TKey }) {
    draft.prompt = t(ex.text);
    if (!draft.name.trim()) draft.name = t(ex.label);
    promptEl?.focus();
  }
  function save() {
    if (!canSave) return;
    const clean = { name: draft.name.trim(), description: draft.description.trim(), prompt: draft.prompt.trim() };
    if (editingId) {
      const st = s.styles.find((x) => x.id === editingId);
      if (st) Object.assign(st, clean);
    } else {
      const st: Style = { id: `custom-${Date.now().toString(36)}`, kind: "llm", builtin: false, ...clean };
      s.styles.push(st);
      s.active_style = st.id;
    }
    formOpen = false;
  }

  // ------------------------------------------------------------ delete
  let toDelete = $state<Style | null>(null);
  let deleteOpen = $state(false);
  function askDelete(st: Style) {
    toDelete = st;
    deleteOpen = true;
  }
  function confirmDelete() {
    const st = toDelete;
    deleteOpen = false;
    if (!st) return;
    s.styles = s.styles.filter((x) => x.id !== st.id);
    if (s.active_style === st.id) s.active_style = s.styles.find((x) => x.builtin)?.id ?? "natural";
    // Per-app rules that used it go too (Rust drops them on save anyway).
    if (s.app_styles.some((r) => r.style_id === st.id)) s.app_styles = s.app_styles.filter((r) => r.style_id !== st.id);
  }

  // ------------------------------------------------------------ radio keyboard nav (roving)
  function radioKey(e: KeyboardEvent) {
    if (!["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(e.key)) return;
    if (!(e.target as HTMLElement).closest(".pick")) return; // e.g. the intensity segmented control
    e.preventDefault();
    const ids = s.styles.map((x) => x.id);
    const i = ids.indexOf(s.active_style);
    const d = e.key === "ArrowDown" || e.key === "ArrowRight" ? 1 : -1;
    const next = ids[(i + d + ids.length) % ids.length];
    s.active_style = next;
    const root = e.currentTarget as HTMLElement;
    queueMicrotask(() => root.querySelector<HTMLButtonElement>(`[data-style="${CSS.escape(next)}"]`)?.focus());
  }

  const langOptions = $derived(TARGET_LANGS.map((l) => ({ value: l, label: langName(l) })));
  const dur = () => (prefersReducedMotion.current ? 0 : 220);
</script>

{#snippet card(st: Style, i: number)}
  {@const active = s.active_style === st.id}
  <li class:active in:fly={{ y: 8, duration: dur(), delay: prefersReducedMotion.current ? 0 : i * 30, easing: cubicOut }}>
    <button
      type="button"
      class="pick"
      role="radio"
      data-style={st.id}
      aria-checked={active}
      tabindex={active ? 0 : -1}
      onclick={() => (s.active_style = st.id)}
    >
      <span class="radio" aria-hidden="true"></span>
      <span class="names">
        <span class="name">
          <span class="nm">{st.name}</span>
          <Badge tone={st.kind === "llm" ? "accent" : "neutral"}>
            {st.kind === "llm" ? t("styles.kind_llm") : t("styles.kind_off")}
          </Badge>
        </span>
        {#if st.description}<span class="desc">{st.description}</span>{/if}
      </span>
      <span class="on mono" aria-hidden="true">{t("styles.active")}</span>
    </button>
    {#if !st.builtin}
      <div class="acts">
        <Button size="sm" variant="ghost" icon="edit" aria-label={t("styles.edit")} title={t("styles.edit")} onclick={() => openEdit(st)} />
        <Button size="sm" variant="ghost" icon="trash" aria-label={t("styles.delete")} title={t("styles.delete")} onclick={() => askDelete(st)} />
      </div>
    {/if}
    {#if st.id === "prompt"}
      <!-- intensity: its own line under the text, revealed only while Prompt is the active style -->
      <div class="strength-wrap" class:open={active} inert={!active}>
        <div class="strength">
          <div class="strength-row">
            <span class="slabel">{t("home.sty_strength")}</span>
            <Segmented
              size="sm"
              label={t("home.sty_strength")}
              bind:value={s.prompt_strength}
              options={[
                { value: "light", label: t("home.sty_strength_light") },
                { value: "medium", label: t("home.sty_strength_medium") },
                { value: "strong", label: t("home.sty_strength_strong") },
              ]}
            />
          </div>
          {#key s.prompt_strength}
            <p class="shint" in:fly={{ y: 4, duration: dur(), easing: cubicOut }}>{t(`home.sty_strength_${s.prompt_strength}_hint`)}</p>
          {/key}
        </div>
      </div>
    {/if}
  </li>
{/snippet}

<header class="ph">
  <h1>{t("styles.title")}</h1>
  <p class="muted">{t("styles.subtitle")}</p>
</header>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="groups" role="radiogroup" aria-label={t("nav.styles")} onkeydown={radioKey}>
  <Section label={t("home.sty_builtin_group")}>
    <ul class="styles">
      {#each builtins as st, i (st.id)}
        {@render card(st, i)}
      {/each}
    </ul>
    <p class="note faint"><Icon name="lock" size={13} />{t("home.sty_hint_builtin")}</p>
  </Section>

  <Section label={t("home.sty_custom_group")}>
    {#snippet aside()}
      <Button size="sm" icon="plus" onclick={openCreate}>{t("home.sty_create")}</Button>
    {/snippet}
    {#if customs.length}
      <ul class="styles">
        {#each customs as st, i (st.id)}
          {@render card(st, i)}
        {/each}
      </ul>
    {:else}
      <button type="button" class="empty" onclick={openCreate}>
        <span class="plus-box" aria-hidden="true"><Icon name="plus" size={18} /></span>
        <span>
          <span class="empty-t">{t("home.sty_create")}</span>
          <span class="empty-d">{t("home.sty_custom_empty")}</span>
        </span>
      </button>
    {/if}
  </Section>
</div>

<AppStyles />

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

<Section label={t("sh.sec_vocab")}>
  <Row label={t("styles.sec_dictionary")} hint={t("styles.dictionary_hint")} stack>
    {#snippet below()}
      <TagInput bind:tags={s.dictionary} placeholder={t("styles.dictionary_placeholder")} label={t("styles.sec_dictionary")} />
    {/snippet}
  </Row>
</Section>

<!-- create / edit -->
<Modal bind:open={formOpen} title={editingId ? t("home.sty_edit_title") : t("home.sty_new_title")} width="560px">
  <form
    class="form"
    id="style-form"
    onsubmit={(e) => {
      e.preventDefault();
      save();
    }}
  >
    <div class="two">
      <label>
        <span class="label">{t("styles.name")}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={draft.name} placeholder={t("home.sty_name_ph")} maxlength="40" autofocus />
      </label>
      <label>
        <span class="label">{t("styles.description")} <span class="opt">· {t("home.sty_optional")}</span></span>
        <input class="field" bind:value={draft.description} placeholder={t("home.sty_desc_ph")} maxlength="90" />
      </label>
    </div>
    <label>
      <span class="label">{t("home.sty_prompt")}</span>
      <textarea class="field" rows="5" bind:value={draft.prompt} bind:this={promptEl} placeholder={t("home.sty_prompt_ph")}></textarea>
    </label>
    <div class="examples">
      <span class="ex-label">{t("home.sty_examples")}</span>
      <div class="chips">
        {#each EXAMPLES as ex (ex.label)}
          <button type="button" class="chip" class:sel={draft.prompt === t(ex.text)} title={t(ex.text)} onclick={() => useExample(ex)}>
            {t(ex.label)}
          </button>
        {/each}
      </div>
    </div>
    <p class="safety"><Icon name="lock" size={14} /><span>{t("home.sty_safety")}</span></p>
  </form>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (formOpen = false)}>{t("c.cancel")}</Button>
    <Button variant="primary" type="submit" form="style-form" icon="check" disabled={!canSave}>
      {editingId ? t("home.sty_save") : t("home.sty_create_btn")}
    </Button>
  {/snippet}
</Modal>

<!-- delete confirm -->
<Modal bind:open={deleteOpen} title={t("home.sty_delete_title")} width="400px">
  <p class="modal-p">{t("home.sty_delete_body", { name: toDelete?.name ?? "" })}</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deleteOpen = false)}>{t("c.cancel")}</Button>
    <Button variant="danger" icon="trash" onclick={confirmDelete}>{t("c.delete")}</Button>
  {/snippet}
</Modal>

<style>
  .ph h1 {
    font-size: 26px;
  }
  .ph p {
    margin-top: 6px;
    max-width: 58ch;
    font-size: 13.5px;
  }
  .groups {
    display: contents;
  }

  /* ------------------------------------------------------------ style rows */
  .styles {
    list-style: none;
    margin: 0 -10px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 4px;
    padding-right: 8px;
    border-radius: var(--r);
    transition: background var(--t) var(--ease);
  }
  li::after {
    /* accent rail that grows in on the active style */
    content: "";
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 2px;
    border-radius: 2px;
    background: var(--accent-fg);
    transform: scaleY(0);
    transition: transform 260ms var(--spring);
  }
  li:hover {
    background: var(--hover);
  }
  li.active {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  li.active::after {
    transform: scaleY(1);
  }
  .pick {
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 12px 6px 12px 14px;
    text-align: left;
    border-radius: var(--r);
  }
  .pick:focus-visible {
    outline-offset: -2px;
  }
  .pick:active .radio {
    transform: scale(0.88);
  }
  .radio {
    margin-top: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
    flex: none;
    display: grid;
    place-items: center;
    transition:
      border-color var(--t-fast),
      transform var(--t-fast) var(--ease);
  }
  .radio::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-fg);
    transform: scale(0);
    transition: transform 280ms var(--spring);
  }
  .pick:hover .radio {
    border-color: var(--text-2);
  }
  .active .radio {
    border-color: var(--accent-fg);
  }
  .active .radio::after {
    transform: scale(1);
  }
  .names {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 500;
    min-width: 0;
  }
  .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .on {
    align-self: center;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent-fg);
    opacity: 0;
    transform: translateX(-6px);
    transition:
      opacity var(--t-fast),
      transform var(--t) var(--ease);
  }
  .active .on {
    opacity: 1;
    transform: none;
  }
  .acts {
    display: flex;
    gap: 2px;
    opacity: 0.55;
    transition: opacity var(--t-fast);
  }
  li:hover .acts,
  li:focus-within .acts {
    opacity: 1;
  }
  .note {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
  }

  .empty {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px;
    text-align: left;
    border: 1px dashed var(--line-strong);
    border-radius: var(--r);
    transition:
      border-color var(--t-fast),
      background var(--t-fast);
  }
  .empty:hover {
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .plus-box {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--r);
    background: var(--surface-2);
    color: var(--accent-fg);
    transition: transform var(--t) var(--spring);
  }
  .empty:hover .plus-box {
    transform: rotate(90deg);
  }
  .empty-t {
    display: block;
    font-size: 13.5px;
    font-weight: 500;
  }
  .empty-d {
    display: block;
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  /* ------------------------------------------------------------ modal form */
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1.6fr;
    gap: 10px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .opt {
    text-transform: none;
    letter-spacing: 0;
  }
  textarea {
    font-size: 13px;
    min-height: 110px;
  }
  .examples {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 10px;
    margin-top: -4px;
  }
  .ex-label {
    font-size: 12px;
    color: var(--text-3);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    height: 26px;
    padding: 0 10px;
    border-radius: 13px;
    border: 1px solid var(--line-strong);
    font-size: 12px;
    color: var(--text-2);
    transition:
      color var(--t-fast),
      border-color var(--t-fast),
      background var(--t-fast),
      transform var(--t-fast) var(--ease);
  }
  .chip:hover {
    color: var(--text);
    border-color: var(--text-3);
  }
  .chip:active {
    transform: scale(0.96);
  }
  .chip.sel {
    color: var(--accent-fg);
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .safety {
    display: flex;
    gap: 9px;
    align-items: flex-start;
    padding: 10px 12px;
    border-radius: var(--r);
    background: var(--bg);
    border: 1px solid var(--line);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .safety :global(.icon) {
    flex: none;
    margin-top: 2px;
    color: var(--accent-fg);
  }
  .modal-p {
    font-size: 13.5px;
    color: var(--text-2);
  }
  @media (max-width: 640px) {
    .two {
      grid-template-columns: 1fr;
    }
  }
  /* ------------------------------------------------------------ Prompt intensity */
  .strength-wrap {
    grid-column: 1 / -1;
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    transition:
      grid-template-rows var(--t-slow) var(--ease-out),
      opacity var(--t) var(--ease);
  }
  .strength-wrap.open {
    grid-template-rows: 1fr;
    opacity: 1;
  }
  .strength {
    min-height: 0;
    overflow: hidden;
    /* aligned with the name/description column: pick padding 14 + radio 16 + gap 14 */
    padding: 0 8px 0 44px;
    transform: translateY(-4px);
    transition: transform var(--t-slow) var(--ease-out);
  }
  .open .strength {
    transform: none;
  }
  .strength-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .slabel {
    font: 500 10.5px var(--mono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .shint {
    margin: 8px 0 0;
    padding-bottom: 14px;
    color: var(--text-2);
    font-size: 12px;
    line-height: 1.45;
  }
  @media (prefers-reduced-motion: reduce) {
    .strength-wrap,
    .strength {
      transition: none;
    }
  }
</style>
