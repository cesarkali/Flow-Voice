<script lang="ts">
  // Browser-only harness: a fake desktop + the real overlay inside a 560x300 "window".
  import { onMount } from "svelte";
  import { mock, mockEmit, simulateOverlay, stopSimulation, type SimScenario } from "../lib/mock";
  import { EVENTS, type Settings } from "../lib/types";
  import Overlay from "./Overlay.svelte";

  const SCENARIOS: { id: SimScenario; label: string }[] = [
    { id: "live", label: "Ditado + ao vivo" },
    { id: "dictate", label: "Ditado" },
    { id: "translate", label: "Tradução" },
    { id: "ask", label: "Pergunta" },
    { id: "transcribing", label: "Transcrevendo" },
    { id: "refining", label: "Refinando" },
    { id: "done", label: "Resultado" },
    { id: "error", label: "Erro" },
  ];
  let theme = $state<"dark" | "light">(new URLSearchParams(location.search).get("theme") === "light" ? "light" : "dark");
  let bg = $state<"desk" | "checker" | "light">("desk");
  let frame = $state(true);
  let auto = $state(!new URLSearchParams(location.search).has("hold"));

  // Push the theme through the same settings-changed event Rust would send.
  $effect(() => {
    const th = theme;
    mock.invoke<Settings>("get_settings").then((st) => {
      st.theme = th;
      mock.invoke("save_settings", { settings: st }).then(() => mockEmit(EVENTS.settingsChanged, st));
    });
  });
  onMount(() => {
    const sc = new URLSearchParams(location.search).get("scenario") as SimScenario | null;
    setTimeout(() => simulateOverlay(sc ?? "live", auto), 400);
  });
</script>

<div class="desk bg-{bg}">
  <div class="fakewin" aria-hidden="true">
    <div class="fw-bar"><i></i><i></i><i></i></div>
    <div class="fw-body">
      <div class="ln w70"></div>
      <div class="ln w90"></div>
      <div class="ln w50"></div>
      <div class="caret"></div>
    </div>
  </div>

  <div class="panel">
    <strong>Overlay preview</strong>
    <div class="btns">
      {#each SCENARIOS as s}
        <button type="button" onclick={() => simulateOverlay(s.id, auto)}>{s.label}</button>
      {/each}
      <button type="button" onclick={stopSimulation}>Esconder</button>
    </div>
    <div class="btns">
      <button type="button" class:on={theme === "dark"} onclick={() => (theme = "dark")}>Escuro</button>
      <button type="button" class:on={theme === "light"} onclick={() => (theme = "light")}>Claro</button>
      <span class="sep"></span>
      <button type="button" class:on={bg === "desk"} onclick={() => (bg = "desk")}>Desktop</button>
      <button type="button" class:on={bg === "checker"} onclick={() => (bg = "checker")}>Xadrez</button>
      <button type="button" class:on={bg === "light"} onclick={() => (bg = "light")}>Claro</button>
      <span class="sep"></span>
      <button type="button" class:on={frame} onclick={() => (frame = !frame)}>Limites da janela</button>
      <button type="button" class:on={auto} onclick={() => (auto = !auto)}>Parar sozinho</button>
    </div>
  </div>

  <div class="win" class:frame>
    <Overlay />
  </div>
</div>

<style>
  .desk {
    position: fixed;
    inset: 0;
    overflow: hidden;
    font-family: var(--font);
  }
  .bg-desk {
    background:
      radial-gradient(1200px 600px at 20% 10%, #3b4a3a 0%, transparent 60%),
      radial-gradient(900px 700px at 90% 80%, #4a3b2e 0%, transparent 60%),
      #1d1f22;
  }
  .bg-checker {
    background-color: #9a9a9a;
    background-image:
      linear-gradient(45deg, #bdbdbd 25%, transparent 25%),
      linear-gradient(-45deg, #bdbdbd 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #bdbdbd 75%),
      linear-gradient(-45deg, transparent 75%, #bdbdbd 75%);
    background-size: 20px 20px;
    background-position:
      0 0,
      0 10px,
      10px -10px,
      -10px 0;
  }
  .bg-light {
    background: linear-gradient(160deg, #e9e6df, #cfd6dc);
  }
  .fakewin {
    position: absolute;
    left: 50%;
    top: 90px;
    width: min(760px, 80vw);
    height: calc(100vh - 470px);
    min-height: 120px;
    transform: translateX(-50%);
    background: #fbfbfa;
    border-radius: 10px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }
  .fw-bar {
    height: 34px;
    background: #ececea;
    display: flex;
    gap: 7px;
    align-items: center;
    padding: 0 12px;
  }
  .fw-bar i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #cfcfcc;
  }
  .fw-body {
    padding: 26px 34px;
  }
  .ln {
    height: 10px;
    border-radius: 5px;
    background: #e3e3e0;
    margin-bottom: 14px;
  }
  .w70 {
    width: 70%;
  }
  .w90 {
    width: 90%;
  }
  .w50 {
    width: 50%;
  }
  .caret {
    width: 2px;
    height: 18px;
    background: #222;
    animation: blink 1s steps(2, start) infinite;
  }
  @keyframes blink {
    to {
      opacity: 0;
    }
  }
  .panel {
    position: absolute;
    top: 14px;
    left: 14px;
    right: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: #eee;
    z-index: 2;
  }
  .panel strong {
    font-family: var(--mono);
    font-size: 10.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    opacity: 0.8;
    text-shadow: 0 1px 2px #000;
  }
  .btns {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }
  .btns button {
    height: 26px;
    padding: 0 10px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.55);
    color: #eee;
    border: 1px solid rgba(255, 255, 255, 0.15);
    font-size: 12px;
  }
  .btns button:hover {
    background: rgba(0, 0, 0, 0.75);
  }
  .btns button.on {
    border-color: #d7ff3f;
    color: #d7ff3f;
  }
  .sep {
    width: 10px;
  }
  .win {
    position: absolute;
    left: 50%;
    bottom: 40px;
    width: 560px;
    height: 300px;
    transform: translateX(-50%);
  }
  .win.frame {
    outline: 1px dashed rgba(255, 255, 255, 0.45);
    outline-offset: 0;
  }
  .bg-light .win.frame,
  .bg-checker .win.frame {
    outline-color: rgba(0, 0, 0, 0.4);
  }
  /* Overlay uses position:fixed for the real window; scope it to the fake window here. */
  .win :global(.stage) {
    position: absolute;
  }
  .win :global(.shell) {
    max-width: calc(560px - var(--m) * 2);
    max-height: calc(300px - var(--m) * 2);
  }
</style>
