<script lang="ts">
  import { onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion, Tween } from "svelte/motion";
  import { fly, slide } from "svelte/transition";
  import { api, on } from "../../lib/api";
  import { clockTime, i18n, langName, locale, relTime, t, TARGET_LANGS } from "../../lib/i18n.svelte";
  import Icon from "../../lib/icons/Icon.svelte";
  import ProviderLogo from "../../lib/icons/ProviderLogo.svelte";
  import { app, hasKey, usableLlm } from "../../lib/stores.svelte";
  import { EVENTS, type DictationResult, type SettingsPage } from "../../lib/types";
  import Button from "../../lib/ui/Button.svelte";
  import { keycapsOf } from "../../lib/ui/hotkeys";
  import Keys from "../../lib/ui/Keys.svelte";
  import Modal from "../../lib/ui/Modal.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import { greeting } from "../greetings";
  import { pname, STT_PROVIDERS } from "../providers";

  let { onnavigate }: { onnavigate?: (p: SettingsPage) => void } = $props();
  const s = $derived(app.settings!);
  /** Bumped when the window shows again (closing it only hides it): "Bom dia" doesn't last into
   *  the night. greeting() keeps its pick within the same part of the day, so nothing flickers. */
  let shownAt = $state(0);
  $effect(() => {
    const again = () => {
      if (document.visibilityState === "visible") shownAt = Date.now();
    };
    document.addEventListener("visibilitychange", again);
    window.addEventListener("focus", again);
    return () => {
      document.removeEventListener("visibilitychange", again);
      window.removeEventListener("focus", again);
    };
  });
  /** Casual greeting (one per app open), split into words for the entrance; `nm` marks the name. */
  const hello = $derived.by(() => {
    void shownAt;
    const name = s.user_name?.trim() ?? "";
    const tpl = greeting(i18n.lang, !!name);
    return tpl.split(" ").map((w) => {
      const [pre, post] = w.includes("{n}") ? w.split("{n}") : [w, null];
      return post === null ? { pre, nm: "", post: "" } : { pre, nm: name, post };
    });
  });

  let history = $state<DictationResult[]>([]);
  let loaded = $state(false);
  let q = $state("");
  let open = $state<string | null>(null);
  let clearOpen = $state(false);
  let copied = $state<string | null>(null);
  let copyTimer = 0;

  async function load() {
    history = await api.getHistory();
    loaded = true;
  }

  onMount(() => {
    load();
    const un = on(EVENTS.historyChanged, load);
    return () => un.then((f) => f());
  });

  // ---------------------------------------------------------------- setup line
  const caps = $derived(keycapsOf(s.hotkeys.dictate));
  const hold = $derived(s.hotkey_mode === "hold");

  const stt = $derived.by(() => {
    if (s.stt.engine === "local") return { id: "local", name: t("speech.local_title"), model: s.stt.local_model };
    const p = s.stt.cloud_order.find((id) => hasKey(id));
    return p ? { id: p as string, name: pname(p), model: s.providers[p].stt_model } : null;
  });
  const llm = $derived.by(() => {
    const p = usableLlm()[0];
    return p ? { id: p as string, name: pname(p), model: s.providers[p].model } : null;
  });
  const styleName = (id: string) => s.styles.find((x) => x.id === id)?.name ?? id;
  const needsSetup = $derived((s.stt.engine === "cloud" && !STT_PROVIDERS.some((p) => hasKey(p))) || !llm);

  // ---------------------------------------------------------------- stats
  const words = (txt: string) => txt.trim().split(/\s+/).filter(Boolean).length;
  const stats = $derived.by(() => {
    const today = new Date().toDateString();
    let n = 0;
    let w = 0;
    let saved = 0;
    for (const h of history) {
      // a failed transcription has no text, a cancelled one wasn't used: not dictations for the numbers
      if (h.mode === "ask" || h.failed || h.cancelled) continue;
      const c = words(h.text);
      w += c;
      saved += Math.max(0, (c / 40) * 60 - h.duration_ms / 1000);
      if (new Date(h.ts).toDateString() === today) n++;
    }
    return { today: n, words: w, saved };
  });

  const tw = { duration: 900, easing: cubicOut };
  const tToday = new Tween(0, tw);
  const tWords = new Tween(0, tw);
  const tSaved = new Tween(0, tw);
  $effect(() => {
    const o = prefersReducedMotion.current ? { duration: 0 } : undefined;
    tToday.set(stats.today, o);
    tWords.set(stats.words, o);
    tSaved.set(stats.saved, o);
  });

  const num = (v: number) => Math.round(v).toLocaleString(locale());
  function dur(sec: number) {
    sec = Math.round(sec);
    if (sec < 60) return t("home.sec", { n: sec });
    const m = Math.round(sec / 60);
    if (m < 60) return t("home.min", { n: m });
    return t("home.hours", { h: Math.floor(m / 60), m: m % 60 });
  }

  // ---------------------------------------------------------------- history
  const term = $derived(q.trim().toLowerCase());
  const filtered = $derived(
    term ? history.filter((h) => h.text.toLowerCase().includes(term) || h.raw.toLowerCase().includes(term)) : history,
  );

  /** Splits `text` into plain/match segments for <mark> highlighting. */
  function parts(text: string): { s: string; m: boolean }[] {
    if (!term) return [{ s: text, m: false }];
    const out: { s: string; m: boolean }[] = [];
    const low = text.toLowerCase();
    let i = 0;
    for (;;) {
      const j = low.indexOf(term, i);
      if (j < 0) break;
      if (j > i) out.push({ s: text.slice(i, j), m: false });
      out.push({ s: text.slice(j, j + term.length), m: true });
      i = j + term.length;
    }
    if (i < text.length) out.push({ s: text.slice(i), m: false });
    return out;
  }

  async function copy(h: DictationResult) {
    await api.copyText(h.text);
    copied = h.id;
    clearTimeout(copyTimer);
    copyTimer = window.setTimeout(() => (copied = null), 1500);
  }
  // ---------------------------------------------------------------- redo from history
  /** The item being redone and which control started it (spinner on that control). */
  let redoing = $state<{ id: string; key: string } | null>(null);
  let redoMsg = $state<{ id: string; ok: boolean; text: string } | null>(null);
  let redoTimer = 0;
  const langOptions = $derived([
    { value: null as string | null, label: t("hx.no_translate") },
    ...TARGET_LANGS.map((l) => ({ value: l as string | null, label: langName(l) })),
  ]);
  async function redo(h: DictationResult, styleId: string, target: string | null, key: string) {
    if (redoing) return;
    redoing = { id: h.id, key };
    redoMsg = null;
    try {
      const r = await api.reprocessHistory(h.id, styleId, target);
      history = history.map((x) => (x.id === r.id ? r : x));
      redoMsg = { id: h.id, ok: true, text: t("hx.redone") };
    } catch (e) {
      redoMsg = { id: h.id, ok: false, text: String(e) };
    } finally {
      redoing = null;
      clearTimeout(redoTimer);
      redoTimer = window.setTimeout(() => (redoMsg = null), 2600);
    }
  }

  // ---------------------------------------------------------------- failed transcription
  /** "Tentar de novo" from the history: the same recording, only copied (the user is in the settings). */
  async function retryFailed(h: DictationResult) {
    if (h.retry !== "ready") return;
    // Shown at once; the next history-changed brings the real state.
    history = history.map((x) => (x.id === h.id ? { ...x, retry: "running" } : x));
    await api.retryFailed(h.id, true);
  }

  async function remove(h: DictationResult) {
    if (open === h.id) open = null;
    history = history.filter((x) => x.id !== h.id);
    await api.deleteHistoryItem(h.id);
  }
  async function clearAll() {
    clearOpen = false;
    open = null;
    history = [];
    await api.clearHistory();
  }

  /** Leave transition: fade + collapse height, so the list closes up smoothly. */
  function collapse(node: HTMLElement, { duration = 220 } = {}) {
    const h = node.offsetHeight;
    return {
      duration: prefersReducedMotion.current ? 0 : duration,
      easing: cubicOut,
      css: (k: number) => `opacity:${k};height:${k * h}px;overflow:hidden;transform:translateX(${(1 - k) * 12}px)`,
    };
  }
  const enterDelay = (i: number) => (prefersReducedMotion.current ? 0 : Math.min(i, 8) * 28);
</script>

<!-- casual greeting, re-rolled each app open -->
<p class="hello">
  <span class="spark" aria-hidden="true"><Icon name="sparkle" size={18} /></span>
  <span class="words">
    {#each hello as w, i (i)}
      <span class="w" style:--i={i}>{w.pre}{#if w.nm}<span class="nm">{w.nm}</span>{/if}{w.post}</span>{" "}
    {/each}
  </span>
</p>

<!-- HERO: teaches the one thing that matters -->
<section class="hero" aria-labelledby="home-hero">
  <div class="hero-top">
    <span class="label">{t("home.kicker")}</span>
    <button type="button" class="link" onclick={() => onnavigate?.("shortcuts")}>
      {t("home.change_hotkey")}<Icon name="chevronRight" size={14} />
    </button>
  </div>

  <div class="caps" aria-label={caps.join(" + ")}>
    {#each caps as k, i (i)}
      {#if i > 0}<span class="plus" style:--i={i} aria-hidden="true">+</span>{/if}
      <kbd class="cap" class:wide={k.length > 3} style:--i={i}><span class="face">{k}</span></kbd>
    {/each}
  </div>

  <h1 id="home-hero">{hold ? t("home.hero_hold") : t("home.hero_toggle")}</h1>
  <p class="sub">{hold ? t("home.hero_sub_hold") : t("home.hero_sub_toggle")}</p>

  <div class="cta">
    <Button variant="primary" size="lg" icon="mic" onclick={() => api.startRecording("dictate")}>{t("home.test_now")}</Button>
    <div class="alts">
      <span class="alt">
        <span class="muted">{t("home.hk_translate")}</span>
        <Keys accel={s.hotkeys.translate} size="sm" />
        <span class="faint mono">→ {s.translation_target.toUpperCase()}</span>
      </span>
      <span class="alt">
        <span class="muted">{t("home.hk_ask")}</span>
        <Keys accel={s.hotkeys.ask} size="sm" />
      </span>
      {#if s.hotkeys.command}
        <span class="alt">
          <span class="muted">{t("home.hk_command")}</span>
          <Keys accel={s.hotkeys.command} size="sm" />
        </span>
      {/if}
      {#if s.hotkeys.proofread}
        <span class="alt">
          <span class="muted">{t("home.hk_proofread")}</span>
          <Keys accel={s.hotkeys.proofread} size="sm" />
        </span>
      {/if}
    </div>
  </div>
</section>

<!-- one-line setup, each part jumps to its page -->
<nav class="setup-line" aria-label={t("home.setup_line")}>
  <button type="button" onclick={() => onnavigate?.("speech")} title={stt?.model}>
    <span class="k">{t("home.voice")}</span>
    {#if stt}<ProviderLogo id={stt.id} size={16} />{/if}
    <span class="v" class:none={!stt}>{stt ? stt.name : t("home.none")}</span>
    <Icon name="chevronRight" size={13} />
  </button>
  <span class="sep" aria-hidden="true"></span>
  <button type="button" onclick={() => onnavigate?.("providers")} title={llm?.model}>
    <span class="k">{t("home.ai")}</span>
    {#if llm}<ProviderLogo id={llm.id} size={16} />{/if}
    <span class="v" class:none={!llm}>{llm ? llm.name : t("home.none")}</span>
    <Icon name="chevronRight" size={13} />
  </button>
</nav>

{#if needsSetup}
  <div class="setup" role="status" in:fly={{ y: 6, duration: 220 }}>
    <Icon name="key" size={18} />
    <p>{llm && !stt ? t("home.setup_voice") : t("home.setup_needed")}</p>
    <Button size="sm" variant="primary" iconRight="arrowRight" onclick={() => onnavigate?.(llm && !stt ? "speech" : "providers")}>{t("home.setup")}</Button>
  </div>
{/if}

{#if s.history_enabled}
  <dl class="stats">
    <div>
      <dt class="label">{t("home.stat_today")}</dt>
      <dd class="tnum">{num(tToday.current)}</dd>
    </div>
    <div>
      <dt class="label">{t("home.stat_words")}</dt>
      <dd class="tnum">{num(tWords.current)}</dd>
    </div>
    <div title={t("home.stat_saved_hint")}>
      <dt class="label">{t("home.stat_saved")}</dt>
      <dd class="tnum">{dur(tSaved.current)}</dd>
    </div>
  </dl>
{/if}

<section class="hist">
  <header>
    <h2 class="label">
      {t("home.history")}
      {#if history.length}
        <span class="count tnum">{term ? t("home.count", { n: filtered.length, total: history.length }) : history.length}</span>
      {/if}
    </h2>
    {#if history.length}
      <div class="tools">
        <label class="search">
          <Icon name="search" size={15} />
          <input bind:value={q} placeholder={t("home.search")} aria-label={t("home.search")} spellcheck="false" />
          {#if q}
            <button type="button" class="clear-q" aria-label={t("c.close")} onclick={() => (q = "")}><Icon name="x" size={13} /></button>
          {/if}
        </label>
        <Button size="sm" variant="ghost" icon="trash" onclick={() => (clearOpen = true)}>{t("home.clear")}</Button>
      </div>
    {/if}
  </header>

  {#if !s.history_enabled}
    <p class="empty-line muted">{t("home.history_off")}</p>
  {:else if loaded && !history.length}
    <div class="empty" in:fly={{ y: 8, duration: 260 }}>
      <div class="ghost-lines" aria-hidden="true">
        <span style:width="72%"></span>
        <span style:width="54%"></span>
        <span style:width="63%"></span>
      </div>
      <h3>{t("home.empty_title")}</h3>
      <p class="muted">{t("home.empty_hint")}</p>
      <div class="empty-keys"><Keys accel={s.hotkeys.dictate} /></div>
    </div>
  {:else if loaded && !filtered.length}
    <p class="empty-line muted">{t("home.no_results", { q })}</p>
  {:else}
    <ul class="items">
      {#each filtered as h, i (h.id)}
        {@const isOpen = open === h.id}
        <li class:open={isOpen} in:fly={{ y: 8, duration: 240, delay: enterDelay(i), easing: cubicOut }} out:collapse>
          <div class="row">
            {#if h.failed}
              <!-- a failed transcription: no text, why it failed, and its kept recording to try again -->
              <div class="main failed">
                <span class="meta">
                  {@render itemMeta(h)}
                  <span class="tag bad"><Icon name="alert" size={12} />{t("hx.failed")}</span>
                </span>
                <span class="ferr">{h.error}</span>
                {#if h.retry}
                  <span class="fretry">
                    <Button size="sm" icon="refresh" loading={h.retry === "running"} disabled={h.retry === "running"} title={t("hx.retry_tip")} onclick={() => retryFailed(h)}>
                      {h.retry === "running" ? t("ovx.retrying") : t("ovx.retry")}
                    </Button>
                  </span>
                {:else}
                  <span class="fgone">{t("hx.failed_gone")}</span>
                {/if}
              </div>
            {:else}
              <button type="button" class="main" aria-expanded={isOpen} onclick={() => (open = isOpen ? null : h.id)}>
                <span class="meta">
                  {@render itemMeta(h)}
                  {#if h.cancelled}
                    <span class="tag cancel" title={t("hx.cancelled_tip")}>{t("hx.cancelled")}</span>
                  {:else if !h.pasted && h.mode !== "ask"}<span class="faint">{t("home.not_pasted")}</span>{/if}
                </span>
                <!-- dictated text runs in its own direction (Arabic, Hebrew…) -->
                {#if h.cancelled && !h.text.trim()}
                  <span class="text none">{h.retry ? t("hx.cancel_no_text") : t("hx.cancel_gone")}</span>
                {:else}
                  <span class="text" class:clamp={!isOpen} dir="auto">
                    {#each parts(h.text) as p, k (k)}{#if p.m}<mark>{p.s}</mark>{:else}{p.s}{/if}{/each}
                  </span>
                {/if}
                {#if term && !isOpen && !h.text.toLowerCase().includes(term)}
                  <span class="raw-hit">
                    <span class="label">{t("home.raw")}</span>
                    <span dir="auto">{#each parts(h.raw) as p, k (k)}{#if p.m}<mark>{p.s}</mark>{:else}{p.s}{/if}{/each}</span>
                  </span>
                {/if}
              </button>
            {/if}
            <div class="acts">
              {#if !h.failed && h.text.trim()}
              <span class="copy-wrap" class:done={copied === h.id}>
                <Button
                  size="sm"
                  variant="ghost"
                  icon={copied === h.id ? "check" : "copy"}
                  aria-label={copied === h.id ? t("c.copied") : t("c.copy")}
                  title={t("c.copy")}
                  onclick={() => copy(h)}
                />
                {#if copied === h.id}
                  <span class="toast mono" role="status" in:fly={{ y: 4, duration: 160 }} out:fly={{ y: -4, duration: 160 }}>{t("c.copied")}</span>
                {/if}
              </span>
              {/if}
              <Button size="sm" variant="ghost" icon="trash" aria-label={t("c.delete")} title={t("c.delete")} onclick={() => remove(h)} />
            </div>
          </div>
          {#if h.cancelled && h.retry}
            <!-- always in sight (not a hover action): the kept recording is the way to get the full text -->
            <div class="cretry">
              <Button
                size="sm"
                icon="wave"
                loading={h.retry === "running"}
                disabled={h.retry === "running"}
                title={t("hx.transcribe_tip")}
                onclick={() => retryFailed(h)}
              >
                {t("ovx.transcribe")}
              </Button>
            </div>
          {/if}
          {#if isOpen && !h.failed}
            <div class="detail" transition:slide={{ duration: prefersReducedMotion.current ? 0 : 220, easing: cubicOut }}>
              <div class="detail-in">
                {#if h.mode === "command" && h.source}
                  <div class="raw">
                    <span class="label">{t("home.f_source")}</span>
                    <p dir="auto">{h.source}</p>
                  </div>
                {/if}
                {#if h.raw && h.raw !== h.text}
                  <div class="raw">
                    <span class="label">{h.mode === "command" ? t("home.f_instruction") : t("home.raw")}</span>
                    <p dir="auto">{#each parts(h.raw) as p, k (k)}{#if p.m}<mark>{p.s}</mark>{:else}{p.s}{/if}{/each}</p>
                  </div>
                {/if}
                <dl class="facts">
                  {#if h.mode === "translate"}
                    <div><dt>{t("home.f_target")}</dt><dd>{langName(h.target)}</dd></div>
                  {:else if h.mode === "dictate"}
                    <div><dt>{t("home.f_style")}</dt><dd>{styleName(h.style_id)}</dd></div>
                  {/if}
                  {#if h.app_name}
                    <div><dt>{t("home.f_app")}</dt><dd>{h.app_name}</dd></div>
                  {/if}
                  <div><dt>{t("home.f_voice")}</dt><dd class="mono">{h.cancelled ? t("hx.f_preview") : h.stt_provider}</dd></div>
                  <div><dt>{t("home.f_ai")}</dt><dd class="mono">{h.provider ?? t("home.no_ai")}</dd></div>
                  <div><dt>{t("home.f_duration")}</dt><dd class="mono">{(h.duration_ms / 1000).toFixed(1)} s</dd></div>
                  <div><dt>{t("home.f_time")}</dt><dd class="mono">{clockTime(h.ts)}</dd></div>
                </dl>
                {#each h.warnings as w (w)}
                  <p class="warn"><Icon name="alert" size={13} />{w}</p>
                {/each}
                {#if h.mode !== "ask" && h.raw}
                  <div class="redo">
                    <span class="label">{t("hx.redo")}</span>
                    <div class="redo-row">
                      {#if h.mode === "command" && h.style_id !== "proofread"}
                        <button
                          type="button"
                          class="rchip"
                          class:on={h.style_id === "command"}
                          class:loading={redoing?.id === h.id && redoing.key === "command"}
                          disabled={!!redoing}
                          title={t("ovx.redo_tip")}
                          onclick={() => redo(h, "command", h.target ?? null, "command")}
                        >
                          <Icon name="refresh" size={13} />{t("ovx.redo")}
                        </button>
                      {/if}
                      {#if h.mode === "command"}
                        <button
                          type="button"
                          class="rchip"
                          class:on={h.style_id === "proofread"}
                          class:loading={redoing?.id === h.id && redoing.key === "proofread"}
                          disabled={!!redoing}
                          title={t("ovx.proofread_tip")}
                          onclick={() => redo(h, "proofread", null, "proofread")}
                        >
                          <Icon name="edit" size={13} />{t("ovx.proofread")}
                        </button>
                      {/if}
                        {#each s.styles as st (st.id)}
                          <button
                            type="button"
                            class="rchip"
                            class:on={h.style_id === st.id}
                            class:loading={redoing?.id === h.id && redoing.key === st.id}
                            disabled={!!redoing}
                            title={st.description || null}
                            onclick={() => redo(h, st.id, h.target ?? null, st.id)}
                          >
                            {st.name}
                          </button>
                        {/each}
                        <span class="redo-lang" class:loading={redoing?.id === h.id && redoing.key === "lang"}>
                          <Select
                            label={t("hx.lang")}
                            value={h.target ?? null}
                            options={langOptions}
                            onchange={(v) => v !== (h.target ?? null) && redo(h, h.style_id, v, "lang")}
                          />
                        </span>
                    </div>
                    {#if redoMsg?.id === h.id}
                      <p class="redo-msg" class:bad={!redoMsg.ok} role="status" in:fly={{ y: 4, duration: 160 }}>
                        <Icon name={redoMsg.ok ? "check" : "alert"} size={13} />{redoMsg.text}
                      </p>
                    {/if}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#snippet itemMeta(h: DictationResult)}
  <span class="time" title={new Date(h.ts).toLocaleString(locale())}>{relTime(h.ts)}</span>
  {#if h.mode === "translate"}
    <span class="tag"><Icon name="translate" size={12} />{langName(h.target)}</span>
  {:else if h.mode === "ask"}
    <span class="tag"><Icon name="question" size={12} />{t("mode.ask")}</span>
  {:else if h.mode === "command" && h.style_id === "proofread"}
    <span class="tag"><Icon name="edit" size={12} />{t("mode.proofread")}</span>
  {:else if h.mode === "command"}
    <span class="tag"><Icon name="sparkle" size={12} />{t("mode.command")}</span>
  {:else}
    <span class="tag">{styleName(h.style_id)}</span>
  {/if}
  {#if h.app_name}<span class="app">{h.app_name}</span>{/if}
{/snippet}

<Modal bind:open={clearOpen} title={t("home.clear_title")} width="400px">
  <p class="modal-p">{t("home.clear_body", { n: history.length })}</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (clearOpen = false)}>{t("c.cancel")}</Button>
    <Button variant="danger" icon="trash" onclick={clearAll}>{t("home.clear_all")}</Button>
  {/snippet}
</Modal>

<style>
  /* ------------------------------------------------------------ greeting */
  .hello {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0 16px 2px;
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 560;
    letter-spacing: -0.025em;
    line-height: 1.25;
    color: var(--text);
  }
  .spark {
    display: grid;
    flex: none;
    color: var(--accent-fg);
    animation: spark-in 900ms var(--spring) 60ms both;
  }
  @keyframes spark-in {
    from {
      opacity: 0;
      transform: rotate(-120deg) scale(0.3);
    }
  }
  .words {
    min-width: 0;
  }
  .w {
    display: inline-block;
    animation: word-in 620ms var(--ease-out) calc(140ms + var(--i) * 55ms) both;
  }
  @keyframes word-in {
    from {
      opacity: 0;
      filter: blur(6px);
      transform: translate3d(0, 6px, 0);
    }
  }
  .nm {
    color: var(--accent-fg);
  }
  @media (prefers-reduced-motion: reduce) {
    .spark,
    .w {
      animation: none;
    }
  }

  /* ------------------------------------------------------------ hero */
  .hero {
    position: relative;
    padding: 18px 22px 22px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  .hero::before {
    /* hairline accent tick, like a hardware power stripe */
    content: "";
    position: absolute;
    left: 22px;
    top: -1px;
    width: 36px;
    height: 2px;
    border-radius: 0 0 2px 2px;
    background: var(--accent-fg);
    transform-origin: left;
    animation: tick 600ms var(--ease) 120ms both;
  }
  @keyframes tick {
    from {
      transform: scaleX(0);
    }
  }
  .hero-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 12px;
    color: var(--text-2);
    padding: 3px 4px 3px 8px;
    border-radius: var(--r-sm);
    transition:
      color var(--t-fast),
      background var(--t-fast);
  }
  .link:hover {
    color: var(--text);
    background: var(--hover);
  }
  .link :global(.icon) {
    transition: transform var(--t) var(--ease);
  }
  .link:hover :global(.icon) {
    transform: translateX(2px);
  }

  .caps {
    margin-top: 18px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .cap {
    --depth: 5px;
    position: relative;
    display: inline-block;
    min-width: 58px;
    height: calc(54px + var(--depth));
    padding-bottom: var(--depth);
    border-radius: 10px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--line-strong);
    animation: cap-in 420ms var(--spring) calc(var(--i) * 70ms) both;
  }
  .face {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 54px;
    padding: 0 16px;
    border-radius: 10px;
    font-family: var(--mono);
    font-size: 17px;
    font-weight: 600;
    color: var(--text);
    background: var(--bg);
    box-shadow:
      inset 0 0 0 1px var(--line-strong),
      inset 0 1px 0 var(--line-strong);
    transform: translateY(0);
    animation: cap-press 520ms var(--ease) calc(500ms + var(--i) * 90ms) both;
    transition: transform 90ms var(--ease);
  }
  .cap.wide .face {
    padding: 0 18px;
    font-size: 15px;
  }
  .cap:hover .face {
    transform: translateY(2px);
  }
  .cap:active .face {
    transform: translateY(var(--depth));
  }
  .plus {
    font-family: var(--mono);
    font-size: 15px;
    color: var(--text-3);
    animation: fade-in 300ms var(--ease) calc(var(--i) * 70ms) both;
  }
  @keyframes cap-in {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
  }
  @keyframes cap-press {
    40% {
      transform: translateY(var(--depth));
    }
  }
  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  .hero h1 {
    margin-top: 20px;
    font-size: 30px;
    line-height: 1.1;
    letter-spacing: -0.035em;
  }
  .sub {
    margin-top: 6px;
    max-width: 52ch;
    font-size: 13.5px;
    color: var(--text-2);
  }
  .cta {
    margin-top: 20px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 14px 22px;
  }
  .alts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    font-size: 12.5px;
  }
  .alt {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    white-space: nowrap;
  }
  .alt .mono {
    font-size: 11px;
  }

  /* ------------------------------------------------------------ setup line */
  .setup-line {
    margin-top: 14px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px;
    font-size: 12.5px;
  }
  .setup-line button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 6px 0 8px;
    border-radius: var(--r-sm);
    color: var(--text-2);
    transition: background var(--t-fast);
  }
  .setup-line button:first-child {
    margin-left: -8px;
  }
  .setup-line button :global(.pl) {
    margin-left: 2px;
    transition: transform var(--t) var(--spring);
  }
  .setup-line button:hover :global(.pl) {
    transform: rotate(-6deg) scale(1.08);
  }
  .setup-line .k {
    font-family: var(--mono);
    font-size: 10.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .setup-line .v {
    color: var(--text);
    font-weight: 500;
  }
  .setup-line .v.none {
    color: var(--danger-fg);
  }
  .setup-line button :global(.icon) {
    color: var(--text-3);
    opacity: 0;
    transform: translateX(-4px);
    transition:
      opacity var(--t-fast),
      transform var(--t) var(--ease);
  }
  .setup-line button:hover {
    background: var(--hover);
  }
  .setup-line button:hover :global(.icon),
  .setup-line button:focus-visible :global(.icon) {
    opacity: 1;
    transform: none;
  }
  .sep {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-3);
    margin: 0 4px;
  }

  .setup {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 12px 11px 14px;
    border-radius: var(--r);
    border: 1px solid var(--accent-line);
    background: var(--accent-soft);
    color: var(--accent-fg);
  }
  .setup p {
    flex: 1;
    font-size: 13px;
    color: var(--text);
  }

  /* ------------------------------------------------------------ stats */
  .stats {
    margin: 28px 0 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }
  .stats > div {
    padding: 14px 12px 14px 0;
    min-width: 0;
  }
  .stats > div + div {
    padding-left: 18px;
    border-left: 1px solid var(--line);
  }
  .stats dd {
    margin: 6px 0 0;
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.03em;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .stats > div:first-child dd {
    color: var(--accent-fg);
  }

  /* ------------------------------------------------------------ history */
  .hist {
    margin-top: 34px;
  }
  .hist header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 12px;
    min-height: 30px;
    margin-bottom: 10px;
  }
  .hist h2 {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .count {
    color: var(--text-2);
    letter-spacing: 0.04em;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 6px 0 9px;
    border-radius: var(--r);
    border: 1px solid var(--line-strong);
    background: var(--surface);
    color: var(--text-3);
    width: 220px;
    transition:
      border-color var(--t-fast),
      box-shadow var(--t-fast),
      width var(--t) var(--ease);
  }
  .search:focus-within {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    font-size: 12.5px;
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--text-3);
  }
  .clear-q {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--text-3);
  }
  .clear-q:hover {
    color: var(--text);
    background: var(--hover);
  }

  .items {
    list-style: none;
    margin: 0 -10px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    position: relative;
    border-radius: var(--r);
    transition: background var(--t) var(--ease);
  }
  li::before {
    /* hairline separators that disappear around the hovered/open item */
    content: "";
    position: absolute;
    left: 10px;
    right: 10px;
    top: -1px;
    height: 1px;
    background: var(--line);
    transition: opacity var(--t-fast);
  }
  li:first-child::before,
  li:hover::before,
  li.open::before,
  li:hover + li::before,
  li.open + li::before {
    opacity: 0;
  }
  li:hover {
    background: var(--hover);
  }
  li.open {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 4px;
    padding-right: 6px;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
    text-align: left;
    padding: 11px 4px 12px 10px;
    border-radius: var(--r);
  }
  .main:focus-visible {
    /* inset so the ring is never clipped by neighbours or the scroll box */
    outline-offset: -2px;
  }
  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 10px;
    font-size: 11px;
    color: var(--text-2);
    font-family: var(--mono);
  }
  .time {
    color: var(--text-3);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
  }
  .open .tag {
    color: var(--accent-fg);
  }
  /* the app the text went to: quiet, after a hairline dot */
  .app {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--text-3);
  }
  .app::before {
    content: "";
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-3);
    opacity: 0.7;
  }
  .text {
    display: block;
    /* .main is left-aligned; dictated text follows its own direction */
    text-align: start;
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text);
    white-space: pre-line;
    overflow-wrap: anywhere;
  }
  .text.clamp {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    /* room for descenders/highlights so nothing looks cropped */
    padding-bottom: 1px;
  }
  .main.failed {
    align-items: flex-start;
  }
  .tag.bad {
    color: var(--danger, #ff5a4e);
  }
  /* cancelled with Esc: told apart from a finished dictation, quieter than a failure */
  .tag.cancel,
  .open .tag.cancel {
    color: var(--warn);
  }
  .cretry {
    padding: 0 10px 12px;
    margin-top: -4px;
  }
  .text.none {
    color: var(--text-3);
    font-style: italic;
  }
  .ferr {
    font-size: 13px;
    line-height: 1.5;
    color: var(--danger, #ff5a4e);
    overflow-wrap: anywhere;
  }
  .fretry {
    margin-top: 2px;
  }
  .fgone {
    font-size: 12px;
    color: var(--text-3);
  }
  .raw-hit {
    display: -webkit-box;
    -webkit-line-clamp: 1;
    line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 12px;
    color: var(--text-2);
  }
  .raw-hit .label {
    margin-right: 6px;
    font-size: 9.5px;
  }
  mark {
    background: var(--accent-soft);
    color: inherit;
    box-shadow: inset 0 -1.5px 0 var(--accent-line);
    border-radius: 2px;
    padding: 0 1px;
  }
  .acts {
    display: flex;
    gap: 2px;
    padding-top: 8px;
    opacity: 0;
    transform: translateX(4px);
    transition:
      opacity var(--t-fast),
      transform var(--t) var(--ease);
  }
  li:hover .acts,
  li:focus-within .acts,
  .open .acts {
    opacity: 1;
    transform: none;
  }
  .copy-wrap {
    position: relative;
    display: inline-flex;
  }
  .copy-wrap.done :global(.btn) {
    color: var(--accent-fg);
  }
  .toast {
    position: absolute;
    right: calc(100% + 4px);
    top: 50%;
    translate: 0 -50%;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10.5px;
    white-space: nowrap;
    color: var(--on-accent);
    background: var(--accent);
    pointer-events: none;
  }

  .detail-in {
    padding: 0 14px 14px 10px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .raw {
    padding: 9px 12px 10px;
    border-left: 2px solid var(--line-strong);
    background: var(--bg);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
  }
  .raw p {
    margin-top: 4px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
    user-select: text;
  }
  .facts {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px 22px;
  }
  .facts div {
    min-width: 0;
  }
  .facts dt {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .facts dd {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text);
  }
  .facts dd.mono {
    font-size: 11.5px;
    color: var(--text-2);
  }
  .redo {
    display: grid;
    gap: 8px;
    margin-top: 4px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .redo .label {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .redo-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .rchip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px;
    border-radius: 999px;
    border: 1px solid var(--line-2, var(--line));
    background: transparent;
    color: var(--text-2);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition:
      background 0.16s,
      color 0.16s,
      border-color 0.16s,
      transform 0.16s;
  }
  .rchip:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--text-3);
    transform: translateY(-1px);
  }
  .rchip.on {
    color: var(--text);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .rchip:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .rchip.loading,
  .redo-lang.loading {
    opacity: 1;
    animation: redo-pulse 0.9s ease-in-out infinite alternate;
  }
  @keyframes redo-pulse {
    to {
      opacity: 0.45;
    }
  }
  .redo-lang {
    margin-left: auto;
    min-width: 170px;
  }
  .redo-msg {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--ok, var(--text-2));
  }
  .redo-msg.bad {
    color: var(--danger, #ff5a4e);
  }
  .warn {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
  }

  .empty {
    padding: 34px 0 30px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }
  .ghost-lines {
    width: 220px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    margin-bottom: 20px;
  }
  .ghost-lines span {
    height: 7px;
    border-radius: 4px;
    background: var(--surface-2);
    transform-origin: left;
    animation: grow 600ms var(--ease) both;
  }
  .ghost-lines span:nth-child(2) {
    animation-delay: 80ms;
  }
  .ghost-lines span:nth-child(3) {
    animation-delay: 160ms;
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
  .empty h3 {
    font-size: 16px;
  }
  .empty p {
    margin-top: 6px;
    max-width: 44ch;
    font-size: 13px;
  }
  .empty-keys {
    margin-top: 16px;
  }
  .empty-line {
    padding: 16px 0;
    border-top: 1px solid var(--line);
    font-size: 13px;
  }
  .modal-p {
    font-size: 13.5px;
    color: var(--text-2);
  }

  @media (max-width: 640px) {
    .stats dd {
      font-size: 21px;
    }
    .search {
      width: 170px;
    }
  }
</style>
