/* FlowVoice site: header, mobile menu, hero dictation demo, styles switcher. No dependencies.
   Header and menu are shared by every page (index, guia, privacidade, novidades). */
(() => {
  "use strict";
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  /* ---------- header + mobile nav ---------- */
  const top = $(".top");
  if (top) {
    const onScroll = () => top.classList.toggle("scrolled", scrollY > 8);
    addEventListener("scroll", onScroll, { passive: true });
    onScroll();
  }
  /* Home on phones: the header's "Baixar" waits until the hero's own download button has scrolled away. */
  const heroCta = $(".hero .btn-primary");
  if (top && heroCta && top.hasAttribute("data-hero-cta")) {
    new IntersectionObserver(([e]) => {
      top.classList.toggle("past-hero", !e.isIntersecting && e.boundingClientRect.top < 0);
    }).observe(heroCta);
  }
  const menuBtn = $(".menu-btn");
  const nav = $("#nav");
  if (menuBtn && nav) {
    const setMenu = (open) => {
      menuBtn.setAttribute("aria-expanded", String(open));
      menuBtn.setAttribute("aria-label", open ? "Fechar menu" : "Abrir menu");
      nav.classList.toggle("open", open);
    };
    menuBtn.addEventListener("click", () => setMenu(menuBtn.getAttribute("aria-expanded") !== "true"));
    nav.addEventListener("click", (e) => { if (e.target.closest("a")) setMenu(false); });
    addEventListener("keydown", (e) => {
      if (e.key === "Escape" && nav.classList.contains("open")) { setMenu(false); menuBtn.focus(); }
    });
    matchMedia("(min-width: 861px)").addEventListener("change", (e) => { if (e.matches) setMenu(false); });
  }

  /* Older markup on secondary pages may still carry data-reveal; it is visible by default now. */
  $$("[data-reveal]").forEach((el) => el.classList.add("in"));

  /* ---------- hero demo ---------- */
  const demo = $(".demo");
  if (demo) {
    const SCENES = [
      {
        app: "Outlook", style: "Profissional",
        said: "é, tipo, pessoal, a reunião de quinta tá confirmada às dez, né, e quem não puder me avisa",
        out: "Olá, pessoal. A reunião de quinta-feira está confirmada para as 10h. Quem não puder participar, por favor me avise.",
      },
      {
        app: "Slack", style: "Natural",
        said: "então, subi a versão nova no servidor de teste, tipo, dá uma olhada quando puder e me fala se quebrou alguma coisa",
        out: "Subi a versão nova no servidor de teste. Dá uma olhada quando puder e me fala se quebrou alguma coisa.",
      },
      {
        app: "ChatGPT", style: "Prompt",
        said: "é, eu preciso de um script em python que lê uma planilha do excel, né, e manda um e-mail pra cada cliente com a fatura atrasada há mais de dez dias",
        out: "Escreva um script em Python que leia uma planilha do Excel e envie um e-mail a cada cliente com fatura atrasada há mais de 10 dias.",
      },
    ];
    const panes = $$(".pane", demo);
    const appBtns = $$(".demo-app", demo);
    const pauseBtn = $(".demo-pause", demo);
    const ov = $("[data-ov]", demo);
    const pill = $("[data-pill]", demo);
    const keys = $("[data-keys]", demo);
    const live = $("[data-live]", demo);
    const time = $("[data-time]", demo);
    const fill = (sel, v) => $$(sel, demo).forEach((el) => (el.textContent = v));

    let gen = 0;
    let paused = false;
    let onScreen = false;
    let started = false;
    const ABORT = Symbol("abort");

    /** Waits `ms` of running time: the clock stops while paused, off screen or in a hidden tab. */
    async function wait(ms, my) {
      let left = ms;
      while (left > 0) {
        if (my !== gen) throw ABORT;
        if (paused || !onScreen || document.hidden) { await sleep(120); continue; }
        const step = Math.min(left, 60);
        await sleep(step);
        left -= step;
      }
      if (my !== gen) throw ABORT;
    }

    const words = (s) => s.split(" ");
    function setOut(pane, text, animate) {
      const out = $(".out", pane);
      out.textContent = "";
      if (!text) return [];
      const spans = words(text).map((w, i, a) => {
        const s = document.createElement("span");
        s.className = "w" + (animate ? " new" : "");
        s.textContent = w + (i < a.length - 1 ? " " : "");
        out.append(s);
        return s;
      });
      return spans;
    }
    function select(i) {
      panes.forEach((p, k) => p.classList.toggle("on", k === i));
      appBtns.forEach((b, k) => b.setAttribute("aria-pressed", String(k === i)));
      fill("[data-style]", SCENES[i].style);
      fill("[data-style2]", SCENES[i].style);
      fill("[data-app]", SCENES[i].app);
      fill("[data-app2]", SCENES[i].app);
    }
    function resetChrome() {
      ov.classList.remove("show", "listening");
      pill.classList.remove("show", "done");
      keys.classList.remove("down");
      live.textContent = "";
      time.textContent = "00:00";
    }
    /** The finished state of a scene, for reduced motion and for a paused demo. */
    function still(i) {
      resetChrome();
      select(i);
      panes.forEach((p, k) => setOut(p, k === i ? SCENES[k].out : "", false));
      pill.classList.add("show", "done");
    }
    function renderLive(list, firm) {
      live.textContent = "";
      const tail = firm ? 0 : Math.min(3, list.length);
      const head = list.slice(0, list.length - tail).join(" ");
      if (head) live.append(head + (tail ? " " : ""));
      if (tail) {
        const t = document.createElement("span");
        t.className = "tail";
        t.textContent = list.slice(-tail).join(" ");
        live.append(t);
      }
    }
    async function press(my) {
      keys.classList.add("down");
      await wait(240, my);
      keys.classList.remove("down");
    }

    const wordMs = (w) => 110 + w.length * 16;
    /* The markup paints scene 0 already listening, with its first FIRST_WORDS words in the overlay,
       so the first frame shows the mechanism instead of an empty compose window. */
    const FIRST_WORDS = 6;

    async function scene(i, my, from = 0) {
      const sc = SCENES[i];
      const pane = panes[i];
      const said = words(sc.said);
      let spoken = 0; // ms of speech so far, drives the overlay's timer
      if (from) {
        select(i);
        ov.classList.add("show", "listening");
        spoken = said.slice(0, from).reduce((t, w) => t + wordMs(w), 0);
      } else {
        resetChrome();
        select(i);
        panes.forEach((p) => setOut(p, "", false));
        await wait(900, my);
        await press(my);
        ov.classList.add("show", "listening");
        await wait(450, my);
      }

      for (let k = from + 1; k <= said.length; k++) {
        renderLive(said.slice(0, k), false);
        const ms = wordMs(said[k - 1]);
        await wait(ms, my);
        spoken += ms;
        time.textContent = "00:" + String(Math.min(59, Math.floor(spoken / 1000))).padStart(2, "0");
      }
      renderLive(said, true);
      await wait(700, my);
      await press(my);
      ov.classList.remove("listening");
      ov.classList.remove("show");
      pill.classList.add("show");
      await wait(1100, my);

      pill.classList.add("done");
      const spans = setOut(pane, sc.out, true);
      await wait(30, my);
      spans.forEach((s, k) => setTimeout(() => s.classList.remove("new"), k * 22));
      await wait(3600, my);
      pill.classList.remove("show");
      await wait(900, my);
    }

    async function loop(from, resume = 0) {
      const my = ++gen;
      let i = from;
      try {
        for (;;) {
          await scene(i, my, resume);
          resume = 0;
          i = (i + 1) % SCENES.length;
        }
      } catch (e) {
        if (e !== ABORT) throw e;
      }
    }

    appBtns.forEach((b, k) =>
      b.addEventListener("click", () => {
        if (reduce || paused) { gen++; still(k); return; }
        started = true;
        loop(k);
      }),
    );

    if (reduce) {
      pauseBtn.hidden = true;
      still(0);
    } else {
      pauseBtn.addEventListener("click", () => {
        paused = !paused;
        pauseBtn.setAttribute("aria-pressed", String(paused));
        pauseBtn.setAttribute("aria-label", paused ? "Continuar demonstração" : "Pausar demonstração");
        demo.classList.toggle("paused", paused);
      });
      new IntersectionObserver((es) => {
        for (const e of es) {
          onScreen = e.isIntersecting;
          if (onScreen && !started) { started = true; loop(0, FIRST_WORDS); }
        }
      }, { threshold: 0.2 }).observe($(".sheet", demo));
    }
  }

  /* ---------- styles switcher ---------- */
  const styler = $(".styler");
  if (styler) {
    const OUT = {
      natural: {
        text: "Pessoal, a reunião de quinta vai ser às 10h. Quem não puder, me avisa.",
        desc: "Corrige e limpa a fala, mantendo o seu jeito de falar.",
      },
      professional: {
        text: "Olá a todos, a reunião de quinta-feira será às 10h. Quem não puder participar, por favor me avise.",
        desc: "Formal e organizado, pronto para e-mail.",
      },
      prompt: {
        text: "Reunião de quinta às 10h. Quem não puder participar deve avisar.",
        desc: "Enxuto para mandar a uma IA: sem rodeios, mesmo contexto, menos tokens.",
      },
      raw: {
        text: "pessoal a reunião de quinta vai ser às dez e quem não puder me avisa",
        desc: "Só as palavras: sem pontuação e sem IA.",
      },
      verbatim: {
        text: "É, tipo, pessoal, a reunião de quinta vai ser às dez, né, e quem não puder me avisa.",
        desc: "Exatamente o que foi transcrito, sem passar por IA.",
      },
    };
    const tabs = $$(".tab", styler);
    const panel = $("#sty-panel");
    const out = $("[data-out]", styler);
    const desc = $("[data-desc]", styler);
    const picks = $$("[data-pick]");

    const setStyle = (key, focus) => {
      const tab = tabs.find((t) => t.dataset.style === key);
      if (!tab) return;
      tabs.forEach((t) => {
        const on = t === tab;
        t.setAttribute("aria-selected", String(on));
        t.tabIndex = on ? 0 : -1;
      });
      panel.setAttribute("aria-labelledby", tab.id);
      picks.forEach((p) => p.setAttribute("aria-pressed", String(p.dataset.pick === key)));
      out.textContent = OUT[key].text;
      desc.textContent = OUT[key].desc;
      if (!reduce) { out.classList.remove("swap"); void out.offsetWidth; out.classList.add("swap"); }
      if (focus) tab.focus();
    };
    tabs.forEach((t, i) => {
      t.addEventListener("click", () => setStyle(t.dataset.style));
      t.addEventListener("keydown", (e) => {
        const d = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
        if (d) { e.preventDefault(); setStyle(tabs[(i + d + tabs.length) % tabs.length].dataset.style, true); }
        if (e.key === "Home") { e.preventDefault(); setStyle(tabs[0].dataset.style, true); }
        if (e.key === "End") { e.preventDefault(); setStyle(tabs[tabs.length - 1].dataset.style, true); }
      });
    });
    picks.forEach((p) => p.addEventListener("click", () => setStyle(p.dataset.pick)));
    picks.forEach((p) => p.setAttribute("aria-pressed", String(p.dataset.pick === "professional")));
  }
})();
