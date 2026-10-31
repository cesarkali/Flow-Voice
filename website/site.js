/* FlowVoice site: hero demo, styles showcase, small feature loops, reveal, nav. No dependencies. */
(() => {
  "use strict";
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  /** Resolves once `el` is on screen and the tab is visible (loops idle otherwise). */
  function whenVisible(el) {
    if (el._vis && !document.hidden) return Promise.resolve();
    return new Promise((res) => {
      const check = () => {
        if (el._vis && !document.hidden) { el._wake = null; res(); }
      };
      el._wake = check;
      document.addEventListener("visibilitychange", check, { once: true });
    });
  }
  const viz = new IntersectionObserver((es) => {
    for (const e of es) {
      e.target._vis = e.isIntersecting;
      if (e.isIntersecting && e.target._wake) e.target._wake();
    }
  }, { threshold: 0.15 });
  const watch = (el) => { el._vis = false; viz.observe(el); };

  /* ---------- header + mobile nav ---------- */
  // Shared by every page (index, guia, privacidade, novidades): each block checks its elements.
  const top = $(".top");
  if (top) {
    const onScroll = () => top.classList.toggle("scrolled", scrollY > 8);
    addEventListener("scroll", onScroll, { passive: true });
    onScroll();
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
  addEventListener("keydown", (e) => { if (e.key === "Escape" && nav.classList.contains("open")) { setMenu(false); menuBtn.focus(); } });
  }

  /* ---------- reveal ---------- */
  const rev = new IntersectionObserver((es) => {
    for (const e of es) if (e.isIntersecting) { e.target.classList.add("in"); rev.unobserve(e.target); }
  }, { rootMargin: "0px 0px -8% 0px", threshold: 0.08 });
  $$("[data-reveal]").forEach((el) => (reduce ? el.classList.add("in") : rev.observe(el)));

  /* ---------- hero demo ---------- */
  const SCENES = [
    {
      scene: "mail", ico: ["#0f6cbd", "O"], title: "Nova mensagem · Outlook",
      style: "Profissional", app: "Outlook", active: 1,
      chunks: ["oi pessoal, é só pra avisar", " que a reunião de amanhã", " vai ser às dez e não às nove,", " e quem puder traz", " os números do trimestre"],
      result: "Olá, pessoal. Apenas um aviso: a reunião de amanhã será às 10h, e não às 9h. Quem puder, por favor, traga os números do trimestre.",
    },
    {
      scene: "chat", ico: ["#1b1b1d", "AI"], title: "Chat de IA · Navegador",
      style: "Prompt", app: "Navegador", active: 2,
      chunks: ["então, tipo, eu queria que você", " revisasse esse código", " e visse se tem algum vazamento", " de memória, né, e aí", " me explica bem curtinho"],
      result: "Revise este código e verifique se há vazamento de memória. Explique de forma curta.",
    },
  ];
  const CHIPS = ["Natural", "Profissional", "Prompt", "Traduzir"];

  const demo = $(".demo");
  if (demo) {
    const keys = $(".keys", demo);
    const pListen = $(".p-listen", demo), pProc = $(".p-proc", demo), pCard = $(".p-card", demo);
    const live = $("[data-live]", demo), time = $("[data-time]", demo), sty = $("[data-sty]", demo);
    const cStyle = $("[data-cstyle]", demo), cMeta = $("[data-cmeta]", demo), cText = $("[data-ctext]", demo), chips = $("[data-chips]", demo);
    const title = $("[data-title]", demo), ico = $("[data-ico]", demo);
    const show = (p) => [pListen, pProc, pCard].forEach((x) => x.classList.toggle("on", x === p));

    const setScene = (s) => {
      $$(".scene", demo).forEach((el) => el.classList.toggle("on", el.dataset.scene === s.scene));
      title.textContent = s.title;
      ico.style.background = s.ico[0];
      ico.textContent = s.ico[1];
      sty.innerHTML = `${s.style}<span class="app-n"> · ${s.app}</span>`;
      cStyle.textContent = s.style;
      cMeta.textContent = `· ${s.app} · Groq`;
      chips.innerHTML = CHIPS.map((c, i) => `<span class="chip${i === s.active ? " on" : ""}">${c}</span>`).join("");
      cText.textContent = s.result;
      live.textContent = "";
      time.textContent = "0:00";
      setTyped(s, "");
    };
    const setTyped = (s, txt) => {
      const sc = $(`[data-scene="${s.scene}"]`, demo);
      $(".typed", sc).textContent = txt;
      const ph = $(".ph", sc);
      if (ph) ph.hidden = !!txt;
    };
    const press = async () => { keys.classList.add("down"); await sleep(260); keys.classList.remove("down"); };

    if (reduce) {
      const s = SCENES[0];
      setScene(s);
      setTyped(s, s.result);
      show(pCard);
    } else {
      watch(demo);
      (async () => {
        for (let n = 0; ; n++) {
          const s = SCENES[n % SCENES.length];
          await whenVisible(demo);
          setScene(s);
          show(null);
          await sleep(1100);
          await press();
          show(pListen);
          const t0 = performance.now();
          const tick = setInterval(() => {
            const sec = Math.floor((performance.now() - t0) / 1000);
            time.textContent = `0:${String(sec).padStart(2, "0")}`;
          }, 250);
          await sleep(500);
          for (const c of s.chunks) {
            $$(".ck", live).forEach((k) => k.classList.remove("un", "new"));
            const sp = document.createElement("span");
            sp.className = "ck un new";
            sp.textContent = c;
            live.append(sp);
            await sleep(850);
          }
          $$(".ck", live).forEach((k) => k.classList.remove("un"));
          await sleep(700);
          clearInterval(tick);
          await press();
          show(pProc);
          await sleep(1300);
          show(pCard);
          setTyped(s, s.result);
          const typed = $(`[data-scene="${s.scene}"] .typed`, demo);
          typed.classList.add("flash");
          requestAnimationFrame(() => requestAnimationFrame(() => typed.classList.remove("flash")));
          await sleep(4600);
          show(null);
          await sleep(900);
        }
      })();
    }
  }

  /* ---------- styles showcase ---------- */
  const OUT = {
    natural: {
      name: "Natural",
      desc: "Corrige e limpa a fala mantendo seu jeito de falar.",
      text: "Eu preciso de um script em Python que leia uma planilha do Excel e mande um e-mail para cada cliente com a fatura atrasada, mas só se o atraso passar de dez dias.",
      en: "I need a Python script that reads an Excel spreadsheet and sends an email to every client with an overdue invoice, but only if it is more than ten days late.",
    },
    professional: {
      name: "Profissional",
      desc: "Texto formal e organizado, pronto para e-mail e trabalho.",
      text: "Preciso de um script em Python que leia uma planilha do Excel e envie um e-mail a cada cliente com fatura em atraso há mais de dez dias.",
      en: "I need a Python script that reads an Excel spreadsheet and emails each client whose invoice is more than ten days overdue.",
    },
    prompt: {
      name: "Prompt",
      desc: "Enxuga a fala para mandar a uma IA: menos tokens, mesmo contexto. A intensidade define o quanto ele condensa.",
      text: {
        light: "Crie um script em Python que leia uma planilha do Excel e envie um e-mail para cada cliente com fatura atrasada, apenas quando o atraso passar de dez dias.",
        medium: "Script em Python:\n- Ler a planilha do Excel com os clientes\n- Enviar um e-mail a cada cliente com fatura atrasada\n- Condição: atraso maior que 10 dias",
        strong: "Python: ler Excel → e-mail a clientes com fatura atrasada > 10 dias.",
      },
      en: {
        light: "Write a Python script that reads an Excel spreadsheet and emails each client with an overdue invoice, only when it is more than ten days late.",
        medium: "Python script:\n- Read the Excel spreadsheet of clients\n- Email each client with an overdue invoice\n- Condition: more than 10 days late",
        strong: "Python: read Excel → email clients with invoices > 10 days overdue.",
      },
    },
    raw: {
      name: "Cru",
      desc: "Só as palavras: sem pontuação, sem quebras de linha, sem IA.",
      text: "eu preciso de um script em python que lê uma planilha do excel e aí manda um email pra cada cliente que tá com a fatura atrasada mas só se passou de dez dias",
    },
    verbatim: {
      name: "Só transcrição",
      desc: "A transcrição exatamente como saiu, sem passar por IA.",
      text: "É, tipo assim, eu preciso de um script em Python que lê uma planilha do Excel, né, e aí manda um e-mail pra cada cliente que tá com a fatura atrasada, mas só se passou de dez dias, sabe?",
    },
    topics: {
      name: "Tópicos",
      desc: "Um estilo criado por você, com uma instrução simples: “Transforme em tópicos curtos, um por linha.”",
      text: "• Script em Python\n• Lê uma planilha do Excel\n• Envia e-mail aos clientes com fatura atrasada\n• Só quando o atraso passa de 10 dias",
      en: "• Python script\n• Reads an Excel spreadsheet\n• Emails clients with an overdue invoice\n• Only when it is more than 10 days late",
    },
  };
  const XLATE_DESC = "Tradução é uma ação, não um estilo: dite já traduzindo (Ctrl + Shift + Y) ou traduza o resultado com um clique no overlay. 34 idiomas.";
  const styler = $(".styler");
  if (styler) {
    const tabs = $$(".tab", styler);
    const out = $("[data-out]", styler), desc = $("[data-desc]", styler), sname = $("[data-sname]", styler);
    const seg = $("[data-seg]", styler), panel = $("#sty-panel");
    const xl = $("[data-xlate]", styler), sxl = $("[data-sxl]", styler);
    let cur = "natural", intensity = "medium", english = false;

    const pick = (v) => (typeof v === "string" ? v : v[intensity]);
    const render = (animate = true) => {
      const o = OUT[cur];
      const canXl = !!o.en;
      const showEn = english && canXl;
      const txt = pick(showEn ? o.en : o.text);
      sname.textContent = o.name;
      sxl.hidden = !showEn;
      desc.textContent = showEn ? XLATE_DESC : o.desc;
      seg.hidden = cur !== "prompt";
      xl.hidden = !canXl;
      xl.setAttribute("aria-pressed", String(showEn));
      if (!animate || reduce) { out.textContent = txt; return; }
      out.textContent = "";
      const frag = document.createDocumentFragment();
      // Split into words but keep line breaks and spaces as plain text nodes.
      txt.split(/(\s+)/).forEach((part, i) => {
        if (/^\s+$/.test(part) || !part) { frag.append(part); return; }
        const w = document.createElement("span");
        w.className = "w";
        w.style.animationDelay = `${Math.min(i * 9, 520)}ms`;
        w.textContent = part;
        frag.append(w);
      });
      out.append(frag);
    };
    const select = (tab, focus) => {
      tabs.forEach((t) => {
        const on = t === tab;
        t.setAttribute("aria-selected", String(on));
        t.tabIndex = on ? 0 : -1;
      });
      panel.setAttribute("aria-labelledby", tab.id);
      if (focus) tab.focus();
      if (cur === tab.dataset.style) return;
      cur = tab.dataset.style;
      render();
    };
    tabs.forEach((t) => t.addEventListener("click", () => select(t)));
    $(".tabs", styler).addEventListener("keydown", (e) => {
      const i = tabs.indexOf(document.activeElement);
      if (i < 0) return;
      const k = e.key;
      let j = null;
      if (k === "ArrowDown" || k === "ArrowRight") j = (i + 1) % tabs.length;
      else if (k === "ArrowUp" || k === "ArrowLeft") j = (i - 1 + tabs.length) % tabs.length;
      else if (k === "Home") j = 0;
      else if (k === "End") j = tabs.length - 1;
      if (j !== null) { e.preventDefault(); select(tabs[j], true); }
    });
    xl.addEventListener("click", () => { english = !english; render(); });
    $$("button", seg).forEach((b) =>
      b.addEventListener("click", () => {
        intensity = b.dataset.int;
        $$("button", seg).forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
        render();
      }),
    );
    render(false);
  }

  /* ---------- live preview stage ---------- */
  const lstage = $("[data-live-demo]");
  if (lstage) {
    const p = $("[data-live2]", lstage), tl = $("[data-tl]", lstage), t2 = $("[data-time2]", lstage);
    const bars = $$("i", tl);
    const CH = ["gente,", " subi a versão nova", " no servidor de teste,", " dá uma olhada", " antes do almoço", " e me avisa se quebrou algo"];
    const TICK = 1500;
    if (reduce) {
      p.innerHTML = CH.map((c) => `<span class="ck">${c}</span>`).join("");
      bars.forEach((b) => b.classList.add("done"));
      t2.textContent = "0:09";
    } else {
      watch(lstage);
      lstage.style.setProperty("--tick", TICK + "ms");
      (async () => {
        for (;;) {
          await whenVisible(lstage);
          p.textContent = "";
          bars.forEach((b) => b.classList.remove("done", "cur"));
          for (let i = 0; i < CH.length; i++) {
            const b = bars[i];
            b.classList.add("cur");
            t2.textContent = `0:${String(Math.round(((i + 1) * TICK) / 1000)).padStart(2, "0")}`;
            await sleep(TICK);
            b.classList.replace("cur", "done");
            $$(".ck", p).forEach((k) => k.classList.remove("un", "new"));
            const sp = document.createElement("span");
            sp.className = "ck un new";
            sp.textContent = CH[i];
            p.append(sp);
          }
          await sleep(900);
          $$(".ck", p).forEach((k) => k.classList.remove("un"));
          await sleep(2600);
        }
      })();
    }
  }

  /* ---------- step cycles (command mode) ---------- */
  $$("[data-cycle]").forEach((el) => {
    const steps = el.dataset.cycle.split(",");
    const ms = el.dataset.cycleMs.split(",").map(Number);
    if (reduce) { el.dataset.step = steps[steps.length - 1]; return; }
    watch(el);
    (async () => {
      for (let i = 0; ; i = (i + 1) % steps.length) {
        await whenVisible(el);
        el.dataset.step = steps[i];
        await sleep(ms[i] || 1500);
      }
    })();
  });

  /* ---------- per-app rows ---------- */
  const apps = $("[data-apps]");
  if (apps && !reduce) {
    const rows = $$(".app-row", apps);
    watch(apps);
    (async () => {
      for (let i = 0; ; i = (i + 1) % rows.length) {
        await whenVisible(apps);
        rows.forEach((r, j) => r.classList.toggle("on", j === i));
        await sleep(1900);
      }
    })();
  }

  /* ---------- gallery ---------- */
  const CAPS = [
    "O atalho principal, os provedores em uso e seu histórico, num lugar só.",
    "Estilos prontos e os seus, com instruções em português.",
    "Cadastre várias chaves por provedor e defina a ordem de uso.",
    "Escolha entre a nuvem e os modelos locais do Whisper, com download dentro do app.",
  ];
  const gtabs = $$(".gal-tabs [data-shot]");
  if (gtabs.length) {
    const imgs = $$(".gal-frame img"), cap = $("[data-gcap]");
    const pick = (b, focus) => {
      const n = +b.dataset.shot;
      gtabs.forEach((t) => { const on = t === b; t.setAttribute("aria-selected", String(on)); t.tabIndex = on ? 0 : -1; });
      imgs.forEach((im, i) => im.classList.toggle("on", i === n));
      cap.textContent = CAPS[n];
      if (focus) b.focus();
    };
    gtabs.forEach((b) => b.addEventListener("click", () => pick(b)));
    $(".gal-tabs").addEventListener("keydown", (e) => {
      const i = gtabs.indexOf(document.activeElement);
      if (i < 0) return;
      if (e.key === "ArrowRight") { e.preventDefault(); pick(gtabs[(i + 1) % gtabs.length], true); }
      if (e.key === "ArrowLeft") { e.preventDefault(); pick(gtabs[(i - 1 + gtabs.length) % gtabs.length], true); }
    });
  }
})();
