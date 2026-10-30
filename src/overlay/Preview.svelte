<script lang="ts">
  // Browser-only harness: a fake desktop + the real overlay inside a fake native window that
  // follows the overlay_resize requests (dashed outline = window bounds).
  // Query flags: ?scenario=<id> &theme=light &width=compact|normal|wide &hold (don't auto-stop)
  import { onMount } from "svelte";
  import { on } from "../lib/api";
  import { mock, mockEmit, simulateOverlay, stopSimulation, type SimScenario } from "../lib/mock";
  import { EVENTS, type OverlayEvent, type Settings } from "../lib/types";
  import Overlay from "./Overlay.svelte";

  type Scn = SimScenario | "reprocess" | "longlive";
  const SCENARIOS: { id: Scn; label: string }[] = [
    { id: "live", label: "Ditado + ao vivo" },
    { id: "longlive", label: "Ao vivo longo" },
    { id: "dictate", label: "Ditado" },
    { id: "translate", label: "Tradução" },
    { id: "ask", label: "Pergunta" },
    { id: "transcribing", label: "Transcrevendo" },
    { id: "refining", label: "Refinando" },
    { id: "done", label: "Resultado" },
    { id: "reprocess", label: "Reprocessar" },
    { id: "error", label: "Erro" },
  ];
  const q = new URLSearchParams(location.search);
  let theme = $state<"dark" | "light">(q.get("theme") === "light" ? "light" : "dark");
  let width = $state<Settings["overlay_width"]>((q.get("width") as Settings["overlay_width"]) || "normal");
  let position = $state<Settings["overlay_position"]>(q.get("pos") === "top" ? "top" : "bottom");
  let bg = $state<"desk" | "checker" | "light">("desk");
  const ACCENTS = ["#D7FF3F", "#4FD1FF", "#FF7A45", "#B48CFF"];
  let accent = $state(q.get("accent") ? `#${q.get("accent")}` : ACCENTS[0]);
  let frame = $state(true);
  let auto = $state(!q.has("hold"));
  let win = $state({ w: 560, h: 300 });

  // Push the look through the same settings-changed event Rust would send.
  $effect(() => {
    const th = theme;
    const w = width;
    const p = position;
    const ac = accent;
    mock.invoke<Settings>("get_settings").then((st) => {
      st.theme = th;
      st.overlay_width = w;
      st.overlay_position = p;
      st.accent_color = ac;
      mock.invoke("save_settings", { settings: st }).then(() => mockEmit(EVENTS.settingsChanged, st));
    });
  });

  let timers: number[] = [];
  let lvl = 0;
  const later = (ms: number, fn: () => void) => timers.push(window.setTimeout(fn, ms));
  function clearLocal() {
    timers.forEach(clearTimeout);
    timers = [];
    clearInterval(lvl);
  }

  const LONG =
    "Então, sobre o projeto novo, eu acho que a gente precisa primeiro alinhar com o time de design quais telas entram na primeira versão, depois fechar o cronograma com o pessoal de engenharia e só então apresentar pro cliente na semana que vem, porque se a gente mostrar antes disso vai parecer que está tudo meio solto e sem dono".split(
      " ",
    );

  function run(id: Scn) {
    clearLocal();
    const emit = (e: OverlayEvent) => mockEmit(EVENTS.overlayState, e);
    if (id === "reprocess") {
      simulateOverlay("done", auto);
      // Click another style like the user would; Rust (old builds) also leaked a stray phase.
      later(1600, () => {
        const chip = document.querySelector<HTMLButtonElement>(".win .chipset .chip:not(.on):not(.back)");
        chip?.click();
        later(250, () => emit({ phase: "refining", provider: "groq" }));
      });
      return;
    }
    if (id === "longlive") {
      stopSimulation();
      emit({ phase: "listening", mode: "dictate", style_id: "natural", target: null, live: true });
      let t = 0;
      let n = 0;
      lvl = window.setInterval(() => {
        t += 33;
        const speaking = Math.sin(t / 1100) > -0.6;
        // a quiet mic: speech ~0.4–0.6, silence ~0.1
        mockEmit(EVENTS.overlayLevel, speaking ? 0.38 + Math.abs(Math.sin(t / 110)) * 0.18 + Math.random() * 0.06 : 0.08 + Math.random() * 0.04);
        if (t % 260 < 33 && n < LONG.length) {
          n++;
          mockEmit(EVENTS.overlayPartial, { text: LONG.slice(0, n).join(" ") });
        }
      }, 33);
      if (auto)
        later(16000, () => {
          clearInterval(lvl);
          emit({ phase: "transcribing" });
          later(900, () => emit({ phase: "refining", provider: "groq" }));
          later(2200, () => simulateOverlay("done", false));
        });
      return;
    }
    simulateOverlay(id, auto);
  }
  function hide() {
    clearLocal();
    stopSimulation();
  }

  onMount(() => {
    const sub = on<{ width: number; height: number }>("mock:overlay-resize", (s) => {
      win = { w: Math.min(760, Math.max(120, s.width)), h: Math.min(560, Math.max(40, s.height)) };
    });
    // Cancel / dismiss (mock emits "hidden") also ends the local scenarios.
    const sub2 = on<OverlayEvent>(EVENTS.overlayState, (e) => e.phase === "hidden" && clearLocal());
    setTimeout(() => run((q.get("scenario") as Scn | null) ?? "live"), 400);
    return () => {
      sub.then((f) => f());
      sub2.then((f) => f());
      clearLocal();
    };
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
        <button type="button" onclick={() => run(s.id)}>{s.label}</button>
      {/each}
      <button type="button" onclick={hide}>Esconder</button>
    </div>
    <div class="btns">
      <button type="button" class:on={theme === "dark"} onclick={() => (theme = "dark")}>Escuro</button>
      <button type="button" class:on={theme === "light"} onclick={() => (theme = "light")}>Claro</button>
      <span class="sep"></span>
      <button type="button" class:on={width === "compact"} onclick={() => (width = "compact")}>Compacto</button>
      <button type="button" class:on={width === "normal"} onclick={() => (width = "normal")}>Normal</button>
      <button type="button" class:on={width === "wide"} onclick={() => (width = "wide")}>Largo</button>
      <span class="sep"></span>
      <button type="button" class:on={position === "bottom"} onclick={() => (position = "bottom")}>Embaixo</button>
      <button type="button" class:on={position === "top"} onclick={() => (position = "top")}>Em cima</button>
      <span class="sep"></span>
      {#each ACCENTS as a}
        <button type="button" class="sw" class:on={accent === a} style="--sw:{a}" title={a} aria-label={a} onclick={() => (accent = a)}></button>
      {/each}
      <span class="sep"></span>
      <button type="button" class:on={bg === "desk"} onclick={() => (bg = "desk")}>Desktop</button>
      <button type="button" class:on={bg === "checker"} onclick={() => (bg = "checker")}>Xadrez</button>
      <button type="button" class:on={bg === "light"} onclick={() => (bg = "light")}>Claro</button>
      <span class="sep"></span>
      <button type="button" class:on={frame} onclick={() => (frame = !frame)}>Limites da janela</button>
      <button type="button" class:on={auto} onclick={() => (auto = !auto)}>Parar sozinho</button>
    </div>
  </div>

  <div class="win" class:frame class:top={position === "top"} style="width:{win.w}px;height:{win.h}px">
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
  .btns button.sw {
    width: 26px;
    padding: 0;
    background: var(--sw);
  }
  .btns button.sw.on {
    outline: 2px solid #fff;
    outline-offset: 1px;
  }
  .sep {
    width: 10px;
  }
  .win {
    position: absolute;
    left: 50%;
    bottom: 40px;
    transform: translateX(-50%);
  }
  .win.top {
    bottom: auto;
    top: 150px;
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
</style>
