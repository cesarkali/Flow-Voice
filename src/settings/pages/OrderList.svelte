<script lang="ts" generics="T extends string">
  import { flip } from "svelte/animate";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";

  let {
    items = $bindable(),
    label,
    nameOf,
    ready,
  }: { items: T[]; label: string; nameOf: (id: T) => string; ready: (id: T) => boolean } = $props();

  let dragging = $state<T | null>(null);

  function move(from: number, to: number) {
    if (to < 0 || to >= items.length || from === to) return;
    const next = [...items];
    const [x] = next.splice(from, 1);
    next.splice(to, 0, x);
    items = next;
  }
  function onDragOver(e: DragEvent, i: number) {
    if (dragging === null) return;
    e.preventDefault();
    const from = items.indexOf(dragging);
    if (from !== i) move(from, i);
  }
  function key(e: KeyboardEvent, i: number) {
    if (e.altKey && e.key === "ArrowUp") {
      e.preventDefault();
      move(i, i - 1);
    } else if (e.altKey && e.key === "ArrowDown") {
      e.preventDefault();
      move(i, i + 1);
    }
  }
</script>

<ol class="order" aria-label={label}>
  {#each items as id, i (id)}
    <li
      animate:flip={{ duration: 160 }}
      class:dragging={dragging === id}
      class:off={!ready(id)}
      draggable="true"
      ondragstart={(e) => {
        dragging = id;
        e.dataTransfer?.setData("text/plain", id);
        if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
      }}
      ondragend={() => (dragging = null)}
      ondragover={(e) => onDragOver(e, i)}
      ondrop={(e) => e.preventDefault()}
    >
      <span class="grip" title={t("c.drag")}><Icon name="grip" size={16} /></span>
      <span class="pos mono">{i + 1}</span>
      <span class="nm">{nameOf(id)}</span>
      {#if !ready(id)}<span class="faint mono tiny">{t("prov.no_key_short")}</span>{/if}
      <span class="arrows">
        <button type="button" aria-label="{t('c.move_up')}: {nameOf(id)}" disabled={i === 0} onclick={() => move(i, i - 1)} onkeydown={(e) => key(e, i)}>
          <Icon name="chevronUp" size={15} />
        </button>
        <button
          type="button"
          aria-label="{t('c.move_down')}: {nameOf(id)}"
          disabled={i === items.length - 1}
          onclick={() => move(i, i + 1)}
          onkeydown={(e) => key(e, i)}
        >
          <Icon name="chevronDown" size={15} />
        </button>
      </span>
    </li>
  {/each}
</ol>

<style>
  .order {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 4px 0 6px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--surface);
    font-size: 13px;
    cursor: grab;
  }
  li.dragging {
    border-color: var(--accent-line);
    background: var(--surface-2);
    opacity: 0.85;
  }
  li.off .nm {
    color: var(--text-3);
  }
  .grip {
    color: var(--text-3);
    display: grid;
  }
  .pos {
    width: 14px;
    font-size: 11px;
    color: var(--text-3);
  }
  li:first-child .pos {
    color: var(--accent-fg);
  }
  .nm {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tiny {
    font-size: 10.5px;
  }
  .arrows {
    display: flex;
  }
  .arrows button {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-2);
  }
  .arrows button:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .arrows button:disabled {
    opacity: 0.25;
  }
</style>
