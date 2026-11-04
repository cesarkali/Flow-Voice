<script lang="ts">
  // A release's notes (its CHANGELOG section, Markdown) as readable blocks: "###" headings become
  // small section titles, "- " items a list, **bold** and `code` keep their look. Built from the
  // text with no HTML injection. Everything after a "---" rule is GitHub-only (download hint) and
  // is left out.
  let { md, max = 260 }: { md: string; max?: number } = $props();

  type Span = { t: string; b?: boolean; c?: boolean };
  // "v": a version heading ("## 2.2.0"), when the whole changelog is shown
  type Block = { kind: "v" | "h" | "li" | "p"; spans: Span[] };

  function inline(s: string): Span[] {
    const out: Span[] = [];
    // [text](url) → text; then **bold** and `code`
    s = s.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
    const re = /\*\*(.+?)\*\*|`([^`]+)`/g;
    let last = 0;
    for (let m; (m = re.exec(s)); ) {
      if (m.index > last) out.push({ t: s.slice(last, m.index) });
      out.push(m[1] !== undefined ? { t: m[1], b: true } : { t: m[2], c: true });
      last = m.index + m[0].length;
    }
    if (last < s.length) out.push({ t: s.slice(last) });
    return out;
  }

  function parse(src: string): Block[] {
    const blocks: Block[] = [];
    let cur: { kind: Block["kind"]; text: string } | null = null;
    const flush = () => {
      if (cur && cur.text.trim()) blocks.push({ kind: cur.kind, spans: inline(cur.text.trim()) });
      cur = null;
    };
    for (const raw of src.replace(/\r/g, "").split("\n")) {
      const line = raw.trimEnd();
      if (/^\s*-{3,}\s*$/.test(line)) break; // GitHub-only footer
      const h = /^(#{1,6})\s+(.*)$/.exec(line);
      const li = /^\s*[-*]\s+(.*)$/.exec(line);
      if (h) {
        flush();
        cur = { kind: h[1].length <= 2 ? "v" : "h", text: h[2] };
        flush();
      } else if (li) {
        flush();
        cur = { kind: "li", text: li[1] };
      } else if (!line.trim()) {
        flush();
      } else if (cur) {
        cur.text += " " + line.trim(); // wrapped line of the same item/paragraph
      } else {
        cur = { kind: "p", text: line };
      }
    }
    flush();
    return blocks;
  }

  const blocks = $derived(parse(md ?? ""));
</script>

<div class="rn" style="max-height:{max}px">
  {#each blocks as b, i (i)}
    {#if b.kind === "v"}
      <h3>{#each b.spans as s, j (j)}{s.t}{/each}</h3>
    {:else if b.kind === "h"}
      <h4>{#each b.spans as s, j (j)}{s.t}{/each}</h4>
    {:else}
      <p class:li={b.kind === "li"}>
        {#each b.spans as s, j (j)}{#if s.b}<strong>{s.t}</strong>{:else if s.c}<code>{s.t}</code>{:else}{s.t}{/if}{/each}
      </p>
    {/if}
  {/each}
</div>

<style>
  .rn {
    overflow: auto;
    padding-right: 6px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-2);
    user-select: text;
    -webkit-user-select: text;
  }
  h3 {
    margin: 22px 0 4px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    font-size: 15px;
    font-weight: 700;
    color: var(--text);
    font-family: var(--mono);
  }
  h3:first-child {
    margin-top: 0;
    padding-top: 0;
    border-top: 0;
  }
  h4 {
    margin: 14px 0 6px;
    font-size: 12.5px;
    font-weight: 650;
    color: var(--text);
    letter-spacing: 0.01em;
  }
  h4:first-child {
    margin-top: 2px;
  }
  p {
    margin: 0 0 6px;
  }
  p.li {
    position: relative;
    padding-left: 16px;
  }
  p.li::before {
    content: "";
    position: absolute;
    left: 4px;
    top: 0.68em;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent-fg);
  }
  strong {
    color: var(--text);
    font-weight: 600;
  }
  code {
    font-family: var(--mono);
    font-size: 0.92em;
    padding: 0 4px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text);
  }
</style>
