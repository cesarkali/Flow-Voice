<script lang="ts">
  import { t } from "../i18n.svelte";
  import Icon from "../icons/Icon.svelte";

  let { tags = $bindable([]), placeholder, label }: { tags: string[]; placeholder: string; label: string } = $props();
  let draft = $state("");
  let input: HTMLInputElement;

  function add() {
    const parts = draft
      .split(/[,;\n]/)
      .map((s) => s.trim())
      .filter(Boolean);
    const next = [...tags];
    for (const p of parts) if (!next.some((x) => x.toLowerCase() === p.toLowerCase())) next.push(p);
    tags = next;
    draft = "";
  }
  function remove(i: number) {
    tags = tags.filter((_, j) => j !== i);
    input?.focus();
  }
  function key(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      add();
    } else if (e.key === "Backspace" && !draft && tags.length) {
      remove(tags.length - 1);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="tags" onclick={() => input?.focus()}>
  {#each tags as tag, i (tag)}
    <span class="tag">
      <span class="mono">{tag}</span>
      <button type="button" aria-label={t("styles.remove_term", { term: tag })} onclick={() => remove(i)}>
        <Icon name="x" size={12} stroke={2} />
      </button>
    </span>
  {/each}
  <input
    bind:this={input}
    bind:value={draft}
    aria-label={label}
    {placeholder}
    onkeydown={key}
    onblur={add}
    spellcheck="false"
  />
</div>

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px;
    min-height: 40px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    cursor: text;
  }
  .tags:focus-within {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 26px;
    padding: 0 3px 0 9px;
    border-radius: 5px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    font-size: 12px;
  }
  .tag button {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-3);
  }
  .tag button:hover {
    background: var(--hover);
    color: var(--text);
  }
  input {
    flex: 1;
    min-width: 140px;
    height: 26px;
    border: 0;
    background: transparent;
    padding: 0 6px;
    font-size: 13px;
  }
  input::placeholder {
    color: var(--text-3);
  }
</style>
