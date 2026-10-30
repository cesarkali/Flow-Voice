<script lang="ts">
  // Browser-only harness: a fake desktop + the real overlay inside a fake native window with the
  // same FIXED 760×560 size and anchoring as Rust (bottom/top-center of the work area, 28px in).
  // Dashed outline = window bounds; accent outline = the hitbox the overlay reports (the only
  // part of the real window that catches the mouse). overlay-pointer is emulated from it.
  // Query flags: ?scenario=<id> &theme=light &width=compact|normal|wide &pos=top &accent=4FD1FF
  // &hold (don't auto-stop). livefix = local guess (dimmed) confirmed later by the cloud
  // (`stable` in overlay-partial). Scenarios: live, livefix, longlive, dictate, translate, translong,
  // ask, transcribing, refining, done, reprocess, error, command (selection edit by voice).
  // Static phases for screenshots (stay put): cmddone, cmdnosel,
  // listen (no live text), long (long result), errnow (error card), answer.
  // Combine with the mock's ?qa= steps, e.g. &qa=wait:600~click:.chip[data-k=translate].
  import { onMount } from "svelte";
  import { on } from "../lib/api";
  import { mock, mockEmit, simulateOverlay, stopSimulation, type SimScenario } from "../lib/mock";
  import { EVENTS, type OverlayEvent, type Settings } from "../lib/types";
  import Overlay from "./Overlay.svelte";

  type Scn = SimScenario | "reprocess" | "longlive" | "livefix" | "translong" | "listen" | "long" | "errnow" | "answer";
  const SCENARIOS: { id: Scn; label: string }[] = [
    { id: "live", label: "Ditado + ao vivo" },
    { id: "longlive", label: "Ao vivo longo" },
    { id: "livefix", label: "Ao vivo local + confirmação" },
    { id: "dictate", label: "Ditado" },
    { id: "translate", label: "Tradução" },
    { id: "translong", label: "Tradução longa + idiomas" },
    { id: "ask", label: "Pergunta" },
    { id: "transcribing", label: "Transcrevendo" },
    { id: "refining", label: "Refinando" },
    { id: "done", label: "Resultado" },
    { id: "reprocess", label: "Reprocessar" },
    { id: "error", label: "Erro" },
    { id: "listen", label: "Só ouvindo" },
    { id: "long", label: "Resultado longo" },
    { id: "errnow", label: "Erro (direto)" },
    { id: "answer", label: "Resposta" },
    { id: "command", label: "Comando" },
    { id: "cmddone", label: "Comando (resultado)" },
    { id: "cmdnosel", label: "Comando sem seleção" },
  ];
  const q = new URLSearchParams(location.search);
  let theme = $state<"dark" | "light">(q.get("theme") === "light" ? "light" : "dark");
  let width = $state<Settings["overlay_width"]>((q.get("width") as Settings["overlay_width"]) || "normal");
  let position = $state<Settings["overlay_position"]>(q.get("pos") === "top" ? "top" : "bottom");
  let bg = $state<"desk" | "checker" | "light">("desk");
  const ACCENTS = ["#D7FF3F", "#4FD1FF", "#FF2E00", "#B48CFF"];
  let accent = $state(q.get("accent") ? `#${q.get("accent")}` : ACCENTS[0]);
  let frame = $state(true);
  let auto = $state(!q.has("hold"));
  /** windows::OVERLAY_W/H and MARGIN in Rust. */
  const WIN = { w: 760, h: 560, margin: 28 };
  type HB = { x: number; y: number; width: number; height: number } | null;
  let hitbox = $state<HB>(null);
  let winEl = $state<HTMLDivElement>();
  let inside = false;
  /** Rust polls the cursor against the hitbox and emits overlay-pointer on change. */
  function pointerAt(cx: number, cy: number) {
    const hb = hitbox;
    const r = winEl?.getBoundingClientRect();
    const now = !!hb && !!r && cx >= r.left + hb.x && cy >= r.top + hb.y && cx <= r.left + hb.x + hb.width && cy <= r.top + hb.y + hb.height;
    if (now === inside) return;
    inside = now;
    mockEmit(EVENTS.overlayPointer, { inside: now });
  }
  let lastPt = { x: -1, y: -1 };

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

  // Live text the way Rust builds it: finished segments + current tail, where each finished
  // segment is later REPLACED in place by the (better) cloud transcript.
  const SEGS: [string, string][] = [
    ["eu queria saber se a gente consegue marcar", "Eu queria saber se a gente consegue marcar"],
    ["aquela reunião pra quinta de tarde", "aquela reunião para quinta-feira à tarde,"],
    ["porque na quarta eu não consigo", "porque na quarta eu não consigo."],
    ["e também preciso revisar a apresentação", "Também preciso revisar a apresentação"],
    ["antes de mandar pro cliente", "antes de mandar para o cliente."],
  ];
  const LONG_TR =
    "About the new project: I think we first need to align with the design team on which screens make it into the first version. Then we should close the schedule with the engineering folks, and only after that present it to the client next week. If we show it earlier, it will look loose and ownerless, and that is the last impression we want to give. I'd also like us to agree on who owns each part of the delivery, so nobody is surprised later. Finally, let's keep a short list of open risks — budget, hiring and the API migration — and review it every Monday until launch, so that the whole team stays on the same page and nothing slips through the cracks.";

  function run(id: Scn) {
    clearLocal();
    const emit = (e: OverlayEvent) => mockEmit(EVENTS.overlayState, e);
    if (id === "listen") {
      stopSimulation();
      emit({ phase: "listening", mode: "dictate", style_id: "natural", target: null, live: false });
      let t = 0;
      lvl = window.setInterval(() => {
        t += 33;
        mockEmit(EVENTS.overlayLevel, 0.3 + Math.abs(Math.sin(t / 120)) * 0.4 + Math.random() * 0.1);
      }, 33);
      return;
    }
    if (id === "answer") {
      stopSimulation();
      emit({ phase: "answer", query: "Qual a previsão do tempo pra amanhã?" });
      return;
    }
    if (id === "errnow") {
      stopSimulation();
      emit({
        phase: "error",
        message: "Não foi possível refinar o texto: todos os provedores falharam.",
        detail:
          'groq: HTTP 429 Too Many Requests — {"error":{"message":"Rate limit reached for model `openai/gpt-oss-20b` in organization on tokens per minute (TPM): Limit 8000, Used 7920, Requested 412. Please try again in 2.4s.","type":"tokens","code":"rate_limit_exceeded"}}\ngemini: sem chave configurada',
        raw: LONG.slice(0, 20).join(" "),
      });
      return;
    }
    if (id === "long") {
      stopSimulation();
      emit({
        phase: "done",
        result: {
          id: `long-${Date.now()}`,
          ts: new Date().toISOString(),
          mode: "dictate",
          raw: LONG.join(" "),
          text: `${LONG_TR}\n\n${LONG_TR}`,
          style_id: "natural",
          target: null,
          provider: "groq · openai/gpt-oss-20b",
          stt_provider: "groq · whisper-large-v3-turbo",
          pasted: false,
          duration_ms: 31200,
          warnings: [],
        },
      });
      return;
    }
    if (id === "livefix") {
      stopSimulation();
      emit({ phase: "listening", mode: "dictate", style_id: "natural", target: null, live: true });
      const fin: string[] = []; // finished segments (rough → refined)
      const ok: boolean[] = []; // segment confirmed by the cloud
      let seg = 0;
      let w = 0;
      let t = 0;
      const push = () => {
        // Like Rust: `stable` = chars of the leading run of confirmed segments.
        const tail = seg < SEGS.length ? SEGS[seg][0].split(" ").slice(0, w).join(" ") : "";
        let full = "";
        let stable = 0;
        let run = true;
        for (let k = 0; k < fin.length; k++) {
          full += (full ? " " : "") + fin[k];
          if (ok[k] && run) stable = Array.from(full).length;
          else run = false;
        }
        if (tail) full += (full ? " " : "") + tail;
        mockEmit(EVENTS.overlayPartial, { text: full, stable });
      };
      lvl = window.setInterval(() => {
        t += 33;
        mockEmit(EVENTS.overlayLevel, 0.3 + Math.abs(Math.sin(t / 120)) * 0.4 + Math.random() * 0.1);
        if (t % 297 >= 33 || seg >= SEGS.length) return;
        const words = SEGS[seg][0].split(" ");
        if (w < words.length) {
          w++;
        } else {
          const k = seg;
          fin.push(SEGS[k][0]);
          ok.push(false);
          seg++;
          w = 0;
          // the cloud transcript of segment k lands a bit later and replaces it mid-string
          later(1300, () => {
            fin[k] = SEGS[k][1];
            ok[k] = true;
            push();
          });
        }
        push();
      }, 33);
      if (auto)
        later(14000, () => {
          clearInterval(lvl);
          emit({ phase: "transcribing" });
          later(900, () => simulateOverlay("done", false));
        });
      return;
    }
    if (id === "translong") {
      stopSimulation();
      emit({
        phase: "done",
        result: {
          id: `translong-${Date.now()}`,
          ts: new Date().toISOString(),
          mode: "translate",
          raw: LONG.join(" "),
          text: LONG_TR,
          style_id: "natural",
          target: "en",
          provider: "groq · openai/gpt-oss-20b",
          stt_provider: "groq · whisper-large-v3-turbo",
          pasted: true,
          duration_ms: 14200,
          warnings: [],
        },
      });
      later(1500, () => document.querySelector<HTMLButtonElement>('.win .chip[data-k="translate"]')?.click());
      return;
    }
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
          // cloud-driven: everything shown is confirmed
          const text = LONG.slice(0, n).join(" ");
          mockEmit(EVENTS.overlayPartial, { text, stable: Array.from(text).length });
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
  /** Follow-mouse monitor switch, as Rust does it: out → move (~140ms) → in. */
  function move() {
    mockEmit("overlay-move", { phase: "out" });
    later(140, () => {
      shift = shift === 0 ? 1 : 0;
      later(16, () => mockEmit("overlay-move", { phase: "in" }));
    });
  }
  let shift = $state(0);
  function hide() {
    clearLocal();
    stopSimulation();
  }

  onMount(() => {
    // Same mock instance the overlay talks to (Vite may re-version the module after a reload),
    // for scripted checks driven from devtools/CDP.
    (window as unknown as { __ovPreview: unknown }).__ovPreview = { mock, mockEmit, stopSimulation, EVENTS };
    // Every hitbox report, timestamped (read by the frame-sampling validation).
    const log: { t: number; rect: HB }[] = ((window as unknown as { __hb: unknown[] }).__hb = []) as never;
    const sub = on<HB>("mock:overlay-hitbox", (r) => {
      hitbox = r;
      log.push({ t: performance.now(), rect: r });
      pointerAt(lastPt.x, lastPt.y);
    });
    const mv = (e: PointerEvent) => {
      lastPt = { x: e.clientX, y: e.clientY };
      pointerAt(e.clientX, e.clientY);
    };
    window.addEventListener("pointermove", mv);
    // Cancel / dismiss (mock emits "hidden") also ends the local scenarios.
    const sub2 = on<OverlayEvent>(EVENTS.overlayState, (e) => e.phase === "hidden" && clearLocal());
    setTimeout(() => run((q.get("scenario") as Scn | null) ?? "live"), 400);
    return () => {
      sub.then((f) => f());
      sub2.then((f) => f());
      window.removeEventListener("pointermove", mv);
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
      <button type="button" onclick={move}>Trocar de monitor</button>
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

  <div class="work">
    <div
      class="win"
      class:frame
      class:top={position === "top"}
      bind:this={winEl}
      style="width:{WIN.w}px;height:{WIN.h}px;--wm:{WIN.margin}px;margin-left:{shift * 180}px"
    >
      <Overlay />
      {#if frame && hitbox}
        <div class="hb" aria-hidden="true" style="left:{hitbox.x}px;top:{hitbox.y}px;width:{hitbox.width}px;height:{hitbox.height}px"></div>
      {/if}
    </div>
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
  /* the monitor's work area (below the preview's control panel) */
  .work {
    position: absolute;
    inset: 104px 0 0;
    pointer-events: none;
  }
  .win {
    position: absolute;
    left: 50%;
    bottom: var(--wm);
    transform: translateX(-50%);
  }
  .win.top {
    bottom: auto;
    top: var(--wm);
  }
  .hb {
    position: absolute;
    outline: 1px solid rgba(215, 255, 63, 0.75);
    outline-offset: -1px;
    pointer-events: none;
    z-index: 10;
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
