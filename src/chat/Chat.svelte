<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, on } from "../lib/api";
  import { setLanguage, t } from "../lib/i18n.svelte";
  import Icon from "../lib/icons/Icon.svelte";
  import Logo from "../lib/icons/Logo.svelte";
  import { applyAccent, applyTheme } from "../lib/theme";
  import { rememberAppearance } from "../lib/ui/appearance-cache";
  import { EVENTS, type ChatMessage, type Settings } from "../lib/types";
  import Button from "../lib/ui/Button.svelte";
  import { keycapsOf } from "../lib/ui/hotkeys";
  import TitleBar from "../lib/ui/TitleBar.svelte";
  import { hostOf, renderMarkdown } from "./markdown";

  let messages = $state<ChatMessage[]>([]);
  let settings = $state<Settings | null>(null);
  let input = $state("");
  let awaiting = $state(false);
  let recording = $state(false);
  let error = $state("");
  let copied = $state<number | null>(null);
  let list: HTMLElement | undefined = $state();
  let ta: HTMLTextAreaElement | undefined = $state();
  let root: HTMLElement | undefined = $state();

  /** Re-plays the open animation when the (hidden, not closed) window is shown again. */
  function replayOpen() {
    if (!root || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const ease = "cubic-bezier(0.16, 1, 0.3, 1)";
    root.querySelector(".composer")?.animate(
      [{ opacity: 0, transform: "translate3d(0, 14px, 0) scale(0.98)" }, { opacity: 1, transform: "none" }],
      { duration: 460, easing: ease, fill: "backwards" },
    );
    root.querySelector(".list")?.animate([{ opacity: 0, transform: "translate3d(0, 8px, 0)" }, { opacity: 1, transform: "none" }], {
      duration: 380,
      delay: 40,
      easing: ease,
      fill: "backwards",
    });
  }

  const thinking = $derived(awaiting || messages.at(-1)?.role === "user");

  function adopt(s: Settings) {
    settings = s;
    applyTheme(s.theme);
    applyAccent(s.accent_color);
    rememberAppearance(s.theme, s.accent_color);
    setLanguage(s.ui_language);
  }

  async function scrollDown() {
    await tick();
    list?.scrollTo({ top: list.scrollHeight, behavior: "smooth" });
  }

  onMount(() => {
    api.getSettings().then(adopt);
    api.getChat().then((m) => {
      messages = m;
      scrollDown();
    });
    const subs = [
      on<ChatMessage[]>(EVENTS.chatUpdated, (m) => {
        messages = m ?? [];
        scrollDown();
      }),
      on<{ recording: boolean }>(EVENTS.chatRecording, (p) => (recording = !!p?.recording)),
      on<{ text: string }>(EVENTS.chatTranscript, (p) => {
        if (!p?.text) return;
        input = p.text;
        send();
      }),
      on<Settings>(EVENTS.settingsChanged, adopt),
    ];
    ta?.focus();
    let hiddenAt = 0;
    const vis = () => {
      if (document.visibilityState === "hidden") hiddenAt = Date.now();
      else if (hiddenAt && Date.now() - hiddenAt > 400) {
        replayOpen();
        ta?.focus();
      }
    };
    document.addEventListener("visibilitychange", vis);
    return () => {
      subs.forEach((p) => p.then((f) => f()));
      document.removeEventListener("visibilitychange", vis);
    };
  });

  async function send() {
    const text = input.trim();
    if (!text || awaiting) return;
    error = "";
    awaiting = true;
    input = "";
    messages = [...messages, { role: "user", content: text, sources: [] }];
    scrollDown();
    try {
      messages = await api.chatSend(text);
    } catch (e) {
      error = t("chat.error", { e: String(e) });
    } finally {
      awaiting = false;
      scrollDown();
      ta?.focus();
    }
  }

  async function mic() {
    try {
      recording = await api.chatToggleRecording();
    } catch (e) {
      error = String(e);
    }
  }
  async function reset() {
    await api.chatReset();
    messages = [];
    error = "";
    ta?.focus();
  }
  async function copy(i: number) {
    await api.copyText(messages[i].content);
    copied = i;
    setTimeout(() => copied === i && (copied = null), 1300);
  }
  function linkClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a[data-url]") as HTMLAnchorElement | null;
    if (!a) return;
    e.preventDefault();
    api.openUrl(a.dataset.url!);
  }
  function keydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
    }
  }
  // auto-grow textarea (1–5 lines)
  $effect(() => {
    input;
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = `${Math.min(ta.scrollHeight, 120)}px`;
  });
</script>

<div class="chat" bind:this={root}>
  <TitleBar maximizable={false}>
    {#snippet left()}
      <span class="brand" data-tauri-drag-region><Logo size={18} glow={false} /><span data-tauri-drag-region>{t("chat.title")}</span></span>
    {/snippet}
    {#snippet right()}
      <Button size="sm" variant="ghost" icon="edit" aria-label={t("chat.new")} title={t("chat.new")} onclick={reset} />
    {/snippet}
  </TitleBar>

  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="list" bind:this={list} onclick={linkClick} aria-live="polite">
    {#if !messages.length && !thinking}
      <div class="empty stagger" style="--stagger: 60ms">
        <span class="micbig"><Icon name="mic" size={26} /></span>
        <h2>{t("chat.empty_title")}</h2>
        <p class="muted">{t("chat.empty_body", { hotkey: keycapsOf(settings?.hotkeys.ask ?? "Alt+Space").join("+") })}</p>
      </div>
    {/if}

    {#each messages as m, i (i)}
      {#if m.role === "user"}
        <div class="msg user"><p>{m.content}</p></div>
      {:else}
        <div class="msg bot">
          <div class="md">{@html renderMarkdown(m.content)}</div>
          {#if m.sources.length}
            <div class="sources">
              <span class="label">{t("chat.sources")}</span>
              <div class="schips">
                {#each m.sources as src, j}
                  <a class="src" href={src.url} data-url={src.url} title={src.title} style="--i: {Math.min(j, 8)}">
                    <span class="n mono">{j + 1}</span>
                    <span class="st">{src.title || hostOf(src.url)}</span>
                    <span class="host mono">{hostOf(src.url)}</span>
                  </a>
                {/each}
              </div>
            </div>
          {/if}
          <div class="tools">
            <button type="button" aria-label={t("chat.copy")} title={t("chat.copy")} onclick={() => copy(i)}>
              <Icon name={copied === i ? "check" : "copy"} size={15} />
            </button>
          </div>
        </div>
      {/if}
    {/each}

    {#if thinking}
      <div class="msg bot typing" aria-label={t("chat.thinking")}>
        <span class="scan" aria-hidden="true"><i></i></span>
        <span class="faint">{t("chat.thinking")}</span>
      </div>
    {/if}
    {#if error}<p class="err">{error}</p>{/if}
  </div>

  <form
    class="composer"
    class:rec={recording}
    onsubmit={(e) => {
      e.preventDefault();
      send();
    }}
  >
    <button type="button" class="mic" class:on={recording} aria-label={recording ? t("chat.mic_stop") : t("chat.mic")} title={recording ? t("chat.mic_stop") : t("chat.mic")} onclick={mic}>
      {#if recording}
        <span class="bars" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
      {:else}
        <Icon name="mic" size={18} />
      {/if}
    </button>
    <textarea
      bind:this={ta}
      bind:value={input}
      rows="1"
      placeholder={recording ? t("chat.listening") : t("chat.placeholder")}
      aria-label={t("chat.placeholder")}
      onkeydown={keydown}
    ></textarea>
    <button type="submit" class="send" aria-label={t("chat.send")} title={t("chat.send")} disabled={!input.trim() || awaiting}>
      <Icon name="arrowUp" size={18} stroke={2} />
    </button>
  </form>
</div>

<style>
  .chat {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .chat > :global(.tb) {
    animation: fv-fade var(--t-slow) var(--ease) backwards;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    /* readable measure when the window is maximized: the thread stays a ~760px column */
    padding: 8px max(18px, calc((100% - 760px) / 2)) 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    user-select: text;
    animation: fv-fade var(--t-page) var(--ease) 60ms backwards;
  }
  .empty {
    margin: auto;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0 18px;
  }
  .micbig {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1px solid var(--line-strong);
    color: var(--accent-fg);
    margin-bottom: 16px;
    background: radial-gradient(circle at 50% 35%, var(--accent-soft), transparent 70%);
  }
  .empty h2 {
    font-size: 18px;
  }
  .empty p {
    margin-top: 6px;
    font-size: 13px;
    max-width: 42ch;
    text-wrap: balance;
  }
  .msg {
    max-width: 100%;
    font-size: 14px;
    line-height: 1.55;
  }
  .msg.bot {
    animation: fv-rise var(--t-page) var(--ease-out) backwards;
  }
  .msg.user {
    transform-origin: 100% 100%;
    animation: msg-user var(--t-slow) var(--spring) backwards;
  }
  @keyframes msg-user {
    from {
      opacity: 0;
      transform: translate3d(12px, 6px, 0) scale(0.94);
    }
  }
  .user {
    align-self: flex-end;
    max-width: 82%;
    padding: 8px 12px;
    border-radius: 14px 14px 4px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    white-space: pre-wrap;
  }
  .bot {
    position: relative;
    padding-right: 4px;
  }
  .md :global(p) {
    margin: 0 0 8px;
  }
  .md :global(p.h) {
    font-weight: 600;
  }
  .md :global(ul),
  .md :global(ol) {
    margin: 0 0 8px;
    padding-left: 20px;
  }
  .md :global(li) {
    margin: 2px 0;
  }
  .md :global(li::marker) {
    color: var(--text-3);
  }
  .md :global(strong) {
    font-weight: 600;
  }
  .md :global(code) {
    font-family: var(--mono);
    font-size: 0.86em;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface-2);
    border: 1px solid var(--line);
  }
  .md :global(pre) {
    margin: 0 0 8px;
    padding: 10px 12px;
    border-radius: var(--r);
    background: var(--surface);
    border: 1px solid var(--line);
    overflow-x: auto;
  }
  .md :global(pre code) {
    padding: 0;
    border: 0;
    background: none;
    font-size: 12px;
  }
  .md :global(a) {
    color: var(--accent-fg);
    text-decoration: underline;
    text-decoration-color: var(--accent-line);
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .sources {
    margin-top: 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .schips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .src {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    height: 28px;
    padding: 0 10px 0 4px;
    border-radius: 14px;
    border: 1px solid var(--line-strong);
    text-decoration: none;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    animation: fv-pop var(--t-slow) var(--spring) backwards;
    animation-delay: calc(160ms + var(--i, 0) * 50ms);
    transition:
      border-color var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease),
      transform var(--t) var(--spring);
  }
  .src:active {
    transform: scale(0.96);
  }
  .src:hover {
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .src .n {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    font-size: 10px;
    color: var(--text-2);
    flex: none;
  }
  .src .st {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .src .host {
    font-size: 10.5px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .tools {
    margin-top: 6px;
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .bot:hover .tools,
  .bot:focus-within .tools,
  .bot:last-of-type .tools {
    opacity: 1;
  }
  .tools button {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--text-3);
  }
  .tools button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .typing {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
  }
  /* a short lime segment sweeping a hairline: reads as "working" without bouncing dots */
  .scan {
    position: relative;
    width: 40px;
    height: 2px;
    border-radius: 2px;
    background: var(--line-strong);
    overflow: hidden;
  }
  .scan i {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 14px;
    border-radius: 2px;
    background: var(--accent-fg);
    animation: scan 900ms var(--ease-in-out) infinite alternate;
  }
  @keyframes scan {
    to {
      transform: translateX(26px);
    }
  }
  .err {
    font-size: 12.5px;
    color: var(--danger-fg);
    font-family: var(--mono);
  }
  .composer {
    margin: 0 max(12px, calc((100% - 784px) / 2)) 12px;
    display: flex;
    align-items: flex-end;
    gap: 6px;
    padding: 6px;
    border-radius: 14px;
    border: 1px solid var(--line-strong);
    background: var(--surface);
    transition:
      border-color var(--t-fast),
      box-shadow var(--t);
    animation: fv-rise var(--t-page) var(--ease-out) 90ms backwards;
    --rise: 14px;
  }
  .composer:focus-within {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .composer.rec {
    border-color: var(--accent-line);
  }
  textarea {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    resize: none;
    font-size: 14px;
    line-height: 20px;
    padding: 7px 4px;
    max-height: 120px;
  }
  textarea:focus-visible {
    /* the composer frame carries the focus ring */
    outline: none;
  }
  textarea::placeholder {
    color: var(--text-3);
  }
  .mic,
  .send {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    flex: none;
    color: var(--text-2);
    transition:
      background-color var(--t-fast),
      color var(--t-fast),
      transform var(--t) var(--spring);
  }
  .mic:active,
  .send:active:not(:disabled) {
    transform: scale(0.9);
    transition-duration: 60ms;
  }
  .send :global(.icon) {
    transition: transform var(--t) var(--spring);
  }
  .send:hover:not(:disabled) :global(.icon) {
    transform: translateY(-2px);
  }
  .mic:hover {
    background: var(--hover);
    color: var(--text);
  }
  .mic.on {
    background: var(--accent);
    color: var(--on-accent);
  }
  .send {
    background: var(--accent);
    color: var(--on-accent);
  }
  .send:disabled {
    background: var(--surface-3);
    color: var(--text-3);
  }
  .bars {
    display: flex;
    gap: 2px;
    align-items: center;
    height: 16px;
  }
  .bars i {
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: currentColor;
    animation: eq 700ms ease-in-out infinite;
    transform-origin: center;
  }
  .bars i:nth-child(2) {
    animation-delay: 120ms;
  }
  .bars i:nth-child(3) {
    animation-delay: 240ms;
  }
  .bars i:nth-child(4) {
    animation-delay: 360ms;
  }
  @keyframes eq {
    0%,
    100% {
      transform: scaleY(0.3);
    }
    50% {
      transform: scaleY(1);
    }
  }
</style>
