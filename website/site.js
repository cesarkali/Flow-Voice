/* FlowVoice site: header, mobile menu, the overlay demos (hero + overlay section), styles switcher. No dependencies.
   Header and menu are shared by every page (index, guia, privacidade, novidades). */
(() => {
  "use strict";
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  /* The page's language and its texts for the scripts: i18n/<lang>.js, written by i18n/build.py from
     i18n/src/demo/<lang>.json. Pages in a language folder sit one level down, so assets resolve from here. */
  const LANG = (document.documentElement.lang || "pt").slice(0, 2);
  const T = (window.FV_I18N && window.FV_I18N[LANG]) || null;
  const SITE = new URL(".", document.currentScript ? document.currentScript.src : location.href).href;

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
    const openLabel = menuBtn.getAttribute("aria-label");
    const closeLabel = T ? T.ui.menuClose : openLabel;
    const setMenu = (open) => {
      menuBtn.setAttribute("aria-expanded", String(open));
      menuBtn.setAttribute("aria-label", open ? closeLabel : openLabel);
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

  /* ---------- language picker: a <details> menu (works without JS); this closes it on Escape or a click
     outside, and remembers that the visitor knows where the languages are ---------- */
  const langPick = $(".lang");
  if (langPick) {
    const close = (focus) => { if (!langPick.open) return; langPick.open = false; if (focus) $("summary", langPick).focus(); };
    document.addEventListener("click", (e) => { if (!langPick.contains(e.target)) close(false); });
    langPick.addEventListener("keydown", (e) => { if (e.key === "Escape") close(true); });
  }
  $$("a[data-lang]").forEach((a) => a.addEventListener("click", () => { try { localStorage.setItem("fv-lang-hint", "1"); } catch {} }));

  /* ---------- on a Portuguese page, a browser set to another of the site's languages gets a quiet offer
     to switch (never a redirect), until it is dismissed or a language is picked ---------- */
  if (T && T.hints && langPick) {
    let seen = false;
    try { seen = localStorage.getItem("fv-lang-hint") === "1"; } catch {}
    const want = (navigator.languages || [navigator.language || ""]).map((l) => l.toLowerCase().slice(0, 2)).find((l) => l === LANG || T.hints[l]);
    const to = want && want !== LANG && $(`a[data-lang="${want}"]`, langPick);
    if (!seen && to) {
      const h = T.hints[want];
      const box = document.createElement("div");
      box.className = "lang-hint";
      box.lang = to.lang;
      box.setAttribute("role", "region");
      box.setAttribute("aria-label", h.label);
      box.innerHTML = '<p></p><a class="btn btn-primary btn-sm"></a><button type="button" class="lang-hint-x"><svg viewBox="0 0 24 24" aria-hidden="true"><path fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" d="M6.5 6.5l11 11M17.5 6.5l-11 11"/></svg></button>';
      $("p", box).textContent = h.text;
      const go = $("a", box);
      go.textContent = h.go;
      go.href = to.href;
      go.hreflang = to.hreflang;
      const x = $("button", box);
      x.setAttribute("aria-label", h.close);
      const done = () => { try { localStorage.setItem("fv-lang-hint", "1"); } catch {} };
      go.addEventListener("click", done);
      x.addEventListener("click", () => { done(); box.remove(); });
      document.body.append(box);
      requestAnimationFrame(() => requestAnimationFrame(() => box.classList.add("in")));
    }
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

  /* ---------- the overlay (.fvo): the app's capsule, rebuilt ----------
     Mirrors src/overlay/Overlay.svelte: one shell whose width/height/radius morph between phases (CSS
     transitions on the sizes set here), content layers that cross-fade on top, one set of 12 bars that
     listen (driven here like the app's level meter) and then become the processing wave (CSS). */
  const BARS = 12;
  const ENV = Array.from({ length: BARS }, (_, i) => 0.42 + 0.58 * Math.pow(Math.sin((Math.PI * (i + 0.5)) / BARS), 1.3));
  const LAG = Array.from({ length: BARS }, (_, i) => Math.round(Math.abs(i - (BARS - 1) / 2) * (23 / BARS)));
  /* words are joined with a space, or with nothing in scripts written without spaces (Japanese, Chinese) */
  const SEP = T ? T.sep : " ";

  /** Pausable clock for a looping demo: waits stop while paused, off screen or in a hidden tab. */
  function runner(watch) {
    const r = { gen: 0, paused: false, onScreen: false, started: false, ABORT: Symbol("abort") };
    r.canRun = () => !r.paused && r.onScreen && !document.hidden;
    r.wait = async (ms, my) => {
      let left = ms;
      while (left > 0) {
        if (my !== r.gen) throw r.ABORT;
        if (!r.canRun()) { await sleep(120); continue; }
        const step = Math.min(left, 60);
        await sleep(step);
        left -= step;
      }
      if (my !== r.gen) throw r.ABORT;
    };
    r.watch = (onFirst) => new IntersectionObserver((es) => {
      for (const e of es) {
        r.onScreen = e.isIntersecting;
        if (r.onScreen && !r.started) { r.started = true; onFirst(); }
      }
    }, { threshold: 0.2 }).observe(watch);
    return r;
  }

  function capsule(root, host, canRun) {
    const shell = $("[data-shell]", root);
    const wave = $(".fvo-wave", root);
    const bars = $$(".fvo-wave i", root);
    const L = { listen: $('[data-layer="listen"]', root), proc: $('[data-layer="proc"]', root), card: $('[data-layer="card"]', root) };
    const live = $("[data-live]", root);
    const text = $("[data-result]", root);
    const head = $("[data-head]", root);
    const hm = $("[data-hm]", root);
    const proc = $("[data-proc]", root);
    const primary = $("[data-primary]", root);
    const life = $(".fvo-life", root);
    const chips = $$("[data-chip]", root);
    let view = "listen";
    let words = [];
    const c = { speaking: false };

    function fit(snap) {
      const avail = Math.max(240, host.clientWidth - 24);
      const compact = avail < 560;
      root.classList.toggle("compact", compact);
      root.classList.toggle("narrow", avail < 380);
      const S = compact ? { cap: 368, live: 440, card: 440 } : { cap: 388, live: 500, card: 600 };
      const el = L[view];
      const hasText = words.length > 0 || root.classList.contains("has-text");
      let w;
      if (view === "proc") w = el.offsetWidth + 2;
      else {
        w = Math.min(avail, view === "card" ? S.card : hasText ? S.live : S.cap);
        el.style.width = `${w}px`;
      }
      const h = el.offsetHeight + 2;
      const r = view === "card" ? 18 : view === "listen" && hasText ? 22 : 28;
      if (snap) shell.classList.add("snap");
      shell.style.width = `${w}px`;
      shell.style.height = `${h}px`;
      shell.style.borderRadius = `${r}px`;
      if (snap) { void shell.offsetWidth; shell.classList.remove("snap"); }
    }

    /* bars: the app's meter (attack/release, ripple from the centre, a slow wobble) fed by a fake voice */
    let raf = 0, amp = 0, hpos = 0, last = 0;
    const hist = new Float32Array(32);
    const still = () => bars.forEach((b, i) => (b.style.transform = `scaleY(${(0.1 + 0.9 * 0.55 * ENV[i]).toFixed(3)})`));
    function loop(now) {
      raf = requestAnimationFrame(loop);
      if (!canRun()) { last = 0; return; }
      const dt = last ? Math.min(4, (now - last) / 16.7) : 1;
      last = now;
      const target = c.speaking ? 0.42 + 0.42 * Math.abs(Math.sin(now / 130)) * (0.6 + 0.4 * Math.sin(now / 370)) + Math.random() * 0.12 : 0.03 + Math.random() * 0.03;
      amp += (target - amp) * (target > amp ? 1 - Math.pow(0.3, dt) : 1 - Math.pow(0.88, dt));
      hpos = (hpos + 1) % 32;
      hist[hpos] = amp;
      for (let i = 0; i < BARS; i++) {
        const v = hist[(hpos - LAG[i] + 32) % 32];
        const wob = 0.62 + 0.38 * Math.sin(now / (150 + i * 19) + i * 2.3) * Math.sin(now / (410 - i * 7) + i);
        bars[i].style.transform = `scaleY(${(0.1 + 0.9 * Math.min(1, v * ENV[i] * (0.55 + wob * 0.65))).toFixed(3)})`;
      }
    }
    const startBars = () => { if (reduce) still(); else if (!raf) raf = requestAnimationFrame(loop); };
    const stopBars = () => { cancelAnimationFrame(raf); raf = 0; };

    c.view = (v, snap = false) => {
      view = v;
      for (const k in L) L[k].classList.toggle("on", k === v);
      wave.className = `fvo-wave ${v === "listen" ? "m-listen" : v === "proc" && !reduce ? "m-proc" : "m-off"}`;
      if (v === "listen") startBars(); else stopBars();
      fit(snap);
    };
    c.show = (v) => { c.view(v, true); shell.classList.remove("hidden"); };
    c.hide = () => { shell.classList.add("hidden"); c.speaking = false; stopBars(); };
    c.refit = () => fit(false);

    /* live text: one span per word; lighter (.un) until confirmed, then solid and never changed again */
    c.clear = () => { live.textContent = ""; words = []; root.classList.remove("has-text"); };
    c.push = (w, firm = false) => {
      const s = document.createElement("span");
      s.className = `ck${firm ? "" : " un"}${reduce ? "" : " add"}`;
      s.textContent = (words.length ? SEP : "") + w;
      live.append(s);
      words.push({ el: s, firm });
      if (words.length === 1) root.classList.add("has-text");
      if (view === "listen") fit(false);
    };
    c.firm = (n, fixes = {}) => {
      for (let i = 0; i < Math.min(n, words.length); i++) {
        const wd = words[i];
        if (wd.firm) continue;
        if (fixes[i] != null) {
          wd.el.textContent = (i ? SEP : "") + fixes[i];
          wd.el.classList.remove("add", "fix");
          void wd.el.offsetWidth;
          if (!reduce) wd.el.classList.add("fix");
        }
        wd.firm = true;
        wd.el.classList.remove("un");
      }
      if (view === "listen") fit(false);
    };
    c.count = () => words.length;
    c.firmCount = () => words.filter((w) => w.firm).length;
    c.proc = (label) => { proc.textContent = label; if (view === "proc") fit(false); };

    /* result card; style is the key (natural, professional…), its name comes from the page's language */
    c.card = ({ title = T.ui.pasted, style, app = "", out, busy = false, primaryOn = false, ms = 6000 }) => {
      head.textContent = title;
      hm.innerHTML = busy ? '<span class="fvo-rw"></span>' : `<span data-style></span>${app ? '<span class="fvo-app"> · <span data-app></span></span>' : ""}`;
      if (busy) $(".fvo-rw", hm).textContent = T.ui.rewriting;
      else {
        $("[data-style]", hm).textContent = T.styles[style];
        if (app) $("[data-app]", hm).textContent = app;
      }
      $(".fvo-via", root).hidden = busy;
      L.card.classList.toggle("busy", busy);
      primary.hidden = !primaryOn;
      if (out != null && text.textContent !== out) {
        text.textContent = out;
        if (!reduce) { text.classList.remove("reveal"); void text.offsetWidth; text.classList.add("reveal"); }
      }
      chips.forEach((ch) => { ch.classList.toggle("on", !busy && ch.dataset.chip === style); ch.classList.remove("loading"); });
      L.card.style.setProperty("--life", `${ms}ms`);
      if (view === "card") fit(false);
    };
    c.loading = (style) => chips.forEach((ch) => ch.classList.toggle("loading", ch.dataset.chip === style));
    c.restartLife = () => { life.style.animation = "none"; void life.offsetWidth; life.style.animation = ""; };
    c.look = (look, light) => {
      shell.classList.toggle("glass", look !== "solid");
      shell.classList.toggle("translucent", look === "translucent");
      shell.classList.toggle("glight", look !== "solid" && light);
    };

    let rt = 0;
    addEventListener("resize", () => { cancelAnimationFrame(rt); rt = requestAnimationFrame(() => fit(true)); });
    text.addEventListener("animationend", () => text.classList.remove("reveal"));
    return c;
  }

  /* ---------- hero demo ---------- */
  const demo = $(".demo");
  if (demo && T) {
    const SCENES = T.hero;
    const panes = $$(".pane", demo);
    const appBtns = $$(".demo-app", demo);
    const pauseBtn = $(".demo-pause", demo);
    const keys = $("[data-keys]", demo);
    const time = $("[data-time]", demo);
    const sheet = $(".sheet", demo);
    const R = runner(sheet);
    const cap = capsule($("[data-fvo]", demo), sheet, R.canRun);
    const fill = (sel, v) => $$(sel, demo).forEach((el) => (el.textContent = v));

    // what was said comes as space-separated words in every language (SEP joins them on screen)
    const words = (s) => s.split(" ");
    // the pasted text appears word by word, or two characters at a time where words have no spaces
    const pieces = (t) => (SEP ? t.split(" ").map((w, i, a) => w + (i < a.length - 1 ? " " : "")) : t.match(/[\s\S]{1,2}/gu) || []);
    function setOut(pane, text, animate) {
      const out = $(".out", pane);
      out.textContent = "";
      if (!text) return [];
      return pieces(text).map((w) => {
        const s = document.createElement("span");
        s.className = "w" + (animate ? " new" : "");
        s.textContent = w;
        out.append(s);
        return s;
      });
    }
    function select(i) {
      panes.forEach((p, k) => p.classList.toggle("on", k === i));
      appBtns.forEach((b, k) => b.setAttribute("aria-pressed", String(k === i)));
      fill("[data-style]", T.styles[SCENES[i].style]);
      fill("[data-app]", SCENES[i].app);
    }
    const doneCard = (i) => cap.card({ style: SCENES[i].style, app: SCENES[i].app, out: SCENES[i].out, ms: 5200 });
    /** The finished state of a scene, for reduced motion and for a paused demo. */
    function still(i) {
      select(i);
      keys.classList.remove("down");
      panes.forEach((p, k) => setOut(p, k === i ? SCENES[k].out : "", false));
      doneCard(i);
      cap.show("card");
    }
    async function press(my) {
      keys.classList.add("down");
      await R.wait(240, my);
      keys.classList.remove("down");
    }

    const wordMs = (w) => 110 + w.length * 16;
    /* The markup paints scene 0 already listening, with its first FIRST_WORDS words in the overlay
       (the last two still lighter), so the first frame shows the mechanism instead of an empty window. */
    const FIRST_WORDS = 6;

    async function scene(i, my, from = 0) {
      const sc = SCENES[i];
      const pane = panes[i];
      const said = words(sc.said);
      let spoken = 0; // ms of speech so far, drives the overlay's timer
      cap.clear();
      if (from) {
        select(i);
        said.slice(0, from).forEach((w, k) => cap.push(w, k < from - 2));
        cap.show("listen");
        spoken = said.slice(0, from).reduce((t, w) => t + wordMs(w), 0);
      } else {
        cap.hide();
        select(i);
        panes.forEach((p) => setOut(p, "", false));
        time.textContent = "00:00";
        await R.wait(900, my);
        await press(my);
        cap.show("listen");
        await R.wait(450, my);
      }
      cap.speaking = true;
      let nextFirm = spoken + 1100;
      for (let k = from; k < said.length; k++) {
        cap.push(said[k]);
        const ms = wordMs(said[k]);
        await R.wait(ms, my);
        spoken += ms;
        time.textContent = "00:" + String(Math.min(59, Math.floor(spoken / 1000))).padStart(2, "0");
        // every ~1.2 s a new answer agrees with the last one on all but the newest words
        if (spoken >= nextFirm) { cap.firm(cap.count() - 2 - (k % 2)); nextFirm = spoken + 1100 + (k % 3) * 150; }
      }
      cap.firm(cap.count());
      cap.speaking = false;
      await R.wait(700, my);
      await press(my);
      cap.proc(T.ui.transcribing);
      cap.view("proc");
      await R.wait(650, my);
      cap.proc(T.ui.refining);
      await R.wait(950, my);

      doneCard(i);
      cap.view("card");
      const spans = setOut(pane, sc.out, true);
      await R.wait(30, my);
      spans.forEach((s, k) => setTimeout(() => s.classList.remove("new"), k * 22));
      await R.wait(5200, my);
      cap.hide();
      await R.wait(700, my);
    }

    async function loop(from, resume = 0) {
      const my = ++R.gen;
      let i = from;
      try {
        for (;;) {
          await scene(i, my, resume);
          resume = 0;
          i = (i + 1) % SCENES.length;
        }
      } catch (e) {
        if (e !== R.ABORT) throw e;
      }
    }

    appBtns.forEach((b, k) =>
      b.addEventListener("click", () => {
        if (reduce || R.paused) { R.gen++; still(k); return; }
        R.started = true;
        loop(k);
      }),
    );

    if (reduce) {
      pauseBtn.hidden = true;
      still(0);
    } else {
      const pauseLabel = pauseBtn.getAttribute("aria-label");
      pauseBtn.addEventListener("click", () => {
        R.paused = !R.paused;
        pauseBtn.setAttribute("aria-pressed", String(R.paused));
        pauseBtn.setAttribute("aria-label", R.paused ? T.ui.play : pauseLabel);
        demo.classList.toggle("paused", R.paused);
      });
      // first frame: the static markup, measured to its real size
      select(0);
      cap.clear();
      words(SCENES[0].said).slice(0, FIRST_WORDS).forEach((w, k) => cap.push(w, k < FIRST_WORDS - 2));
      cap.view("listen", true);
      R.watch(() => loop(0, FIRST_WORDS));
    }
  }

  /* ---------- the overlay section: live preview, one-click restyle, and the capsule's look ---------- */
  const ovd = $(".ovdemo");
  if (ovd && T) {
    const desk = $(".desk", ovd);
    const R = runner(desk);
    const cap = capsule($("[data-fvo]", ovd), desk, R.canRun);
    const looks = $$("[data-look]", ovd);
    const lightBtn = $(".ov-light", ovd);
    const pauseBtn = $(".demo-pause", ovd);
    const time = $("[data-time]", ovd);
    let look = "glass";
    let light = false;
    const applyLook = () => {
      looks.forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.look === look)));
      lightBtn.disabled = look === "solid";
      lightBtn.setAttribute("aria-pressed", String(light && look !== "solid"));
      cap.look(look, light);
    };
    looks.forEach((b) => b.addEventListener("click", () => { look = b.dataset.look; applyLook(); }));
    lightBtn.addEventListener("click", () => { light = !light; applyLook(); });
    applyLook();

    // what the cloud first hears, and the words it settles on once two answers agree
    const SAID = T.overlay.said.split(" ");
    const FIX = T.overlay.fix;
    const NAT = T.overlay.natural;
    const PRO = T.overlay.professional;

    async function run(my) {
      cap.hide();
      cap.clear();
      await R.wait(800, my);
      cap.show("listen");
      cap.speaking = true;
      await R.wait(600, my);
      let t = 0;
      let nextFirm = 1300;
      time.textContent = "00:00";
      for (let k = 0; k < SAID.length; k++) {
        cap.push(SAID[k]);
        const ms = 150 + SAID[k].length * 22;
        await R.wait(ms, my);
        t += ms;
        time.textContent = "00:" + String(Math.floor(t / 1000)).padStart(2, "0");
        if (t >= nextFirm) { cap.firm(cap.count() - 3, FIX); nextFirm = t + 1300; }
      }
      cap.speaking = false;
      await R.wait(500, my);
      cap.firm(cap.count(), FIX);
      await R.wait(1100, my);
      cap.proc(T.ui.transcribing);
      cap.view("proc");
      await R.wait(650, my);
      cap.proc(T.ui.refining);
      await R.wait(950, my);
      cap.card({ style: "natural", out: NAT, ms: 9000 });
      cap.view("card");
      await R.wait(1900, my);
      cap.loading("professional");
      cap.card({ busy: true, style: "natural", ms: 9000 });
      cap.loading("professional");
      await R.wait(1300, my);
      cap.card({ title: T.ui.pastedPrev, style: "professional", out: PRO, primaryOn: true, ms: 5000 });
      cap.restartLife();
      await R.wait(5000, my);
      cap.hide();
      await R.wait(600, my);
    }
    async function loop() {
      const my = ++R.gen;
      try { for (;;) await run(my); } catch (e) { if (e !== R.ABORT) throw e; }
    }
    if (reduce) {
      pauseBtn.hidden = true;
      SAID.slice(0, 15).forEach((w) => cap.push(w));
      cap.firm(Math.min(11, SAID.length), FIX);
      cap.show("listen");
    } else {
      const pauseLabel = pauseBtn.getAttribute("aria-label");
      pauseBtn.addEventListener("click", () => {
        R.paused = !R.paused;
        pauseBtn.setAttribute("aria-pressed", String(R.paused));
        pauseBtn.setAttribute("aria-label", R.paused ? T.ui.play : pauseLabel);
        ovd.classList.toggle("paused", R.paused);
      });
      cap.hide();
      R.watch(loop);
    }
  }

  /* ---------- styles switcher ----------
     Each style has its own scene: what was said, where, and what lands at the cursor. Prompt has the app's
     three intensities. The said line and the result sit in stacks with hidden copies of every variant, so the
     card keeps one height and the tabs never move. */
  const styler = $(".styler");
  if (styler && T) {
    const icon = (app) => (app ? `<img src="${SITE}assets/apps/${app}.svg" alt="" width="16" height="16">` : "");
    const STY = T.styler;
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
      ctx.innerHTML = icon(STY[key].icon);
      ctx.append(STY[key].where);
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
    render(currentKey, false);
  }
})();
