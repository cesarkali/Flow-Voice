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

  /* ---------- theme: follows the OS until the visitor picks one; the pick is remembered ----------
     The inline script in <head> applies a stored pick before first paint; this keeps the button,
     the theme-color metas and color-scheme in step. */
  const themeBtn = $(".theme-btn");
  if (themeBtn) {
    const root = document.documentElement;
    const osDark = matchMedia("(prefers-color-scheme: dark)");
    const COLORS = { light: "#f6f6f4", dark: "#0b0b0a" };
    const current = () => root.getAttribute("data-theme") || (osDark.matches ? "dark" : "light");
    const sync = () => {
      const stored = root.getAttribute("data-theme");
      const dark = current() === "dark";
      themeBtn.setAttribute("aria-pressed", String(dark));
      // with no stored pick the media-based metas already match the OS; with a pick both follow it
      $$('meta[name="theme-color"]').forEach((m) => {
        const media = m.getAttribute("media") || "";
        m.content = stored ? COLORS[stored] : media.includes("dark") ? COLORS.dark : COLORS.light;
      });
      const cs = $('meta[name="color-scheme"]');
      if (cs) cs.content = stored || "light dark";
    };
    const apply = (next) => {
      root.setAttribute("data-theme", next);
      try { localStorage.setItem("fv-theme", next); } catch {}
      sync();
    };
    /* Only a click animates (never the first paint or an OS change). The new theme is revealed by a circle
       growing from the switch; without View Transitions, a short colour crossfade; reduced motion, instant. */
    themeBtn.addEventListener("click", () => {
      const next = current() === "dark" ? "light" : "dark";
      if (matchMedia("(prefers-reduced-motion: reduce)").matches) { apply(next); return; }
      if (!document.startViewTransition) {
        root.classList.add("theme-fade");
        apply(next);
        setTimeout(() => root.classList.remove("theme-fade"), 300);
        return;
      }
      const r = themeBtn.getBoundingClientRect();
      const x = r.left + r.width / 2;
      const y = r.top + r.height / 2;
      const end = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
      root.classList.add("theme-vt");
      const vt = document.startViewTransition(() => apply(next));
      vt.ready.then(() => {
        root.animate(
          { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${end}px at ${x}px ${y}px)`] },
          { duration: 520, easing: "cubic-bezier(0.22, 1, 0.36, 1)", pseudoElement: "::view-transition-new(root)" },
        );
      }).catch(() => {});
      vt.finished.finally(() => root.classList.remove("theme-vt"));
    });
    osDark.addEventListener("change", sync);
    sync();
  }

  /* ---------- page loader: if the next page takes more than ~120 ms, a caret types a line across the top ----------
     Internal page links only (not same-page anchors, new tabs, downloads or modified clicks). */
  const bar = document.createElement("div");
  bar.className = "nav-progress";
  bar.setAttribute("aria-hidden", "true");
  document.body.append(bar);
  let barTimer = 0;
  const pagePath = (p) => p.replace(/index\.html$/, "");
  document.addEventListener("click", (e) => {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    const a = e.target.closest("a[href]");
    if (!a || (a.target && a.target !== "_self") || a.hasAttribute("download") || a.hasAttribute("data-dl")) return;
    const u = new URL(a.href, location.href);
    if (u.origin !== location.origin || pagePath(u.pathname) === pagePath(location.pathname)) return;
    clearTimeout(barTimer);
    barTimer = setTimeout(() => bar.classList.add("go"), 120);
  });
  // back/forward cache: the page comes back exactly as it was left, so clear the loader
  addEventListener("pageshow", () => { clearTimeout(barTimer); bar.classList.remove("go"); });

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
        out: "Preciso de um script em Python que leia uma planilha do Excel e envie um e-mail a cada cliente com fatura atrasada há mais de 10 dias.",
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

  /* ---------- styles switcher ----------
     Each style has its own scene: what was said, where, and what lands at the cursor. Prompt has the app's
     three intensities. The said line and the result sit in stacks with hidden copies of every variant, so the
     card keeps one height and the tabs never move. */
  const styler = $(".styler");
  if (styler) {
    const icon = (app) => `<img src="assets/apps/${app}.svg" alt="" width="16" height="16">`;
    const STY = {
      natural: {
        ctx: icon("whatsapp") + "no WhatsApp",
        said: "“<s>é,</s> mãe, vou chegar mais tarde hoje, <s>tipo,</s> o trânsito tá parado aqui na marginal, <s>né,</s> guarda um prato pra mim que eu janto quando chegar”",
        text: "Mãe, vou chegar mais tarde hoje, o trânsito tá parado aqui na Marginal. Guarda um prato pra mim que eu janto quando chegar.",
        desc: "Corrige e limpa a fala, mantendo o seu jeito de falar.",
      },
      professional: {
        ctx: icon("outlook") + "no Outlook",
        said: "“<s>é,</s> oi, pessoal, <s>então,</s> sobre o relatório do trimestre, <s>né,</s> eu preciso que cada um mande a sua parte até sexta, <s>tipo,</s> até o meio-dia, pra eu conseguir revisar tudo antes da reunião de segunda”",
        text: "Olá, pessoal. Sobre o relatório trimestral: preciso que cada um envie a sua parte até sexta-feira, ao meio-dia, para que eu consiga revisar tudo antes da reunião de segunda-feira.",
        desc: "Formal e organizado, pronto para e-mail.",
      },
      prompt: {
        ctx: icon("claude") + "no Claude",
        said: "“<s>é, então,</s> eu queria que você me ajudasse a corrigir uma função em JavaScript, <s>né,</s> ela devia somar os preços do carrinho, mas tá retornando NaN quando algum produto vem sem preço, <s>tipo,</s> e não muda o nome da função porque ela é usada em outros lugares”",
        levels: {
          light: {
            text: "Eu queria que você me ajudasse a corrigir uma função em JavaScript. Ela devia somar os preços do carrinho, mas tá retornando NaN quando algum produto vem sem preço. E não muda o nome da função, porque ela é usada em outros lugares.",
            desc: "Fraco: só tira vícios e palavras sobrando; mantém as suas frases.",
          },
          medium: {
            text: "Preciso corrigir uma função em JavaScript que deveria somar os preços do carrinho, mas retorna NaN quando algum produto vem sem preço. Não mude o nome da função: ela é usada em outros lugares.",
            desc: "Médio: enxuga com equilíbrio para mandar a uma IA, sem perder contexto.",
          },
          strong: {
            text: "Corrigir função JavaScript que soma os preços do carrinho:\n- retorna NaN quando um produto vem sem preço\n- não mudar o nome (usada em outros lugares)",
            desc: "Forte: condensa em tópicos curtos, sem perder requisitos, números e prazos.",
          },
        },
      },
      raw: {
        ctx: icon("chrome") + "na busca do Chrome",
        said: "“melhores fones bluetooth até 300 reais, <s>tipo,</s> com cancelamento de ruído, <s>né,</s> pra usar na academia”",
        text: "Melhores fones bluetooth até 300 reais com cancelamento de ruído pra usar na academia",
        desc: "Só as palavras: sem pontuação e sem IA. Bom para buscas e anotações rápidas.",
      },
      verbatim: {
        ctx: "nas notas de uma entrevista",
        said: "“é, então, a ideia é a gente lançar em março, né, mas, tipo, só se o teste com os clientes der certo”",
        text: "É, então, a ideia é a gente lançar em março, né? Mas, tipo, só se o teste com os clientes der certo.",
        desc: "Exatamente o que foi transcrito, com cada “né” e “tipo”, sem passar por IA.",
      },
    };
    let level = "medium";
    let currentKey = "professional";
    const pick = (key) => (STY[key].levels ? STY[key].levels[level] : STY[key]);

    const tabs = $$(".tab", styler);
    const panel = $("#sty-panel");
    const out = $("[data-out]", styler);
    const desc = $("[data-desc]", styler);
    const said = $("[data-said]", styler);
    const ctx = $("[data-ctx]", styler);
    const strength = $(".strength", styler);
    const levelBtns = $$("[data-level]", styler);
    const picks = $$("[data-pick]");

    /* hidden copies of every variant reserve the tallest size in each stack */
    const sizers = (stack, list, html) => list.forEach((v) => {
      const p = document.createElement("p");
      p.className = stack.firstElementChild.className + " sizer";
      p.setAttribute("aria-hidden", "true");
      if (html) p.innerHTML = v; else p.textContent = v;
      stack.append(p);
    });
    const allOut = Object.values(STY).flatMap((v) => (v.levels ? Object.values(v.levels) : [v]));
    sizers($("[data-said-stack]", styler), Object.values(STY).map((v) => v.said), true);
    sizers($("[data-out-stack]", styler), allOut.map((v) => v.text), false);

    const render = (key, animate) => {
      const v = pick(key);
      out.textContent = v.text;
      desc.textContent = v.desc;
      said.innerHTML = STY[key].said;
      ctx.innerHTML = STY[key].ctx;
      strength.classList.toggle("off", !STY[key].levels);
      levelBtns.forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.level === level)));
      if (animate && !reduce) {
        for (const el of [out, said]) { el.classList.remove("swap"); void el.offsetWidth; el.classList.add("swap"); }
      }
    };
    const setStyle = (key, focus) => {
      const tab = tabs.find((t) => t.dataset.style === key);
      if (!tab) return;
      currentKey = key;
      tabs.forEach((t) => {
        const on = t === tab;
        t.setAttribute("aria-selected", String(on));
        t.tabIndex = on ? 0 : -1;
      });
      panel.setAttribute("aria-labelledby", tab.id);
      picks.forEach((p) => p.setAttribute("aria-pressed", String(p.dataset.pick === key)));
      render(key, true);
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
    levelBtns.forEach((b) => b.addEventListener("click", () => { level = b.dataset.level; render(currentKey, true); }));
    picks.forEach((p) => p.addEventListener("click", () => setStyle(p.dataset.pick)));
    picks.forEach((p) => p.setAttribute("aria-pressed", String(p.dataset.pick === "professional")));
  }
})();
