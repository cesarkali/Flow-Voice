# FlowVoice website: design system

Not published: `.vercelignore` keeps this file, PRODUCT.md, `assets/apps/CREDITS.md` and `assets/apps/preview.html` off the site (the files stay in the repo).

## Direction: "the writing surface and the red caret"

The page is a clean sheet you write on, and the only colour that speaks is the app's red text cursor. Light, quiet ground; near-black ink for type and buttons; the real FlowVoice overlay (a dark capsule) floating over that light surface, exactly how it looks over Outlook or Word on a real desktop. Red (`#FF2E00`) is reserved for what is red in the product: the caret, the recording wave, the stop/recording signal. Never for buttons, never for decoration.

Why light: the audience writes e-mail, chats and prompts at a desk in daylight; the product lives on top of white document surfaces. The previous dark + neon-glow look read as the generic AI-landing default, and the owner rejected it.

### Direction contract (home, `index.html`)

- **THESIS**: Show the mechanism, not features. The page refuses the category default of dark SaaS hero + stats strip + feature-card grid + long FAQ.
- **OWN-WORLD**: Off-white ground `#F6F6F4`, white sheets with a soft long shadow, ink `#111113`, one red caret. Schibsted Grotesk (tight, heavy display; plain body). Hairline-separated lists instead of cards. One dark ink field at the end (closing CTA + footer), the colour of the logo tile.
- **STORY**: In one viewport the visitor sees speech become clean text at the cursor, in three real apps (real icons), the product's name and the download button. Then: five styles, each with its own real scene, + per-app mapping; Command mode; free/offline/private with the real Providers screen; six details; three questions; download.
- **FIRST VIEWPORT**: Two-line headline across the full width, ending in a blinking red caret. Below: left column the signature (app icon + "FlowVoice", see `.sig`), lede, black "Baixar para Windows" pill, GitHub ghost pill, one meta line ("Windows 10 e 11 · ~3 MB · sem pedir administrador"). Right (7/11 of the width): app switcher (Outlook, Slack, ChatGPT, pause) above a white sheet showing that app's compose surface; the overlay capsule rises at the bottom, streams live words (uncertain tail in grey), collapses into "Reescrevendo com Groq…", then "✓ Colado · Estilo · App" while the clean text settles at the caret. The markup paints the first frame already listening (overlay up, first six words of scene 0, timer 00:01) and `site.js` resumes from that word, so the first paint never shows an empty compose window; reduced motion replaces it with the finished state.
- **FORM**: Code-led build; signature interaction = the dictation loop. seed: none (unavailable).
- **FINISH**: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.

## Tokens (`styles.css` `:root`)

Names are shared with `pages.css`; keep them when restyling secondary pages.

| Token | Value | Use |
|---|---|---|
| `--bg` | `#f6f6f4` | page ground |
| `--bg-2` | `#eeeeea` | rare alternate ground |
| `--surface` | `#ffffff` | sheets, cards that hold content, key caps |
| `--surface-2` | `#f1f1ee` | hover fills, inline code |
| `--text` | `#111113` | headings, body emphasis |
| `--text-2` | `#4d4d54` | body copy (≈7.9:1 on bg) |
| `--text-3` | `#66666e` | labels, meta (≈5.2:1 on bg) |
| `--line` / `--line-2` | `#e2e2dd` / `#cfcfc9` | hairlines / control borders |
| `--accent` | `#ff2e00` | fills only: caret, wave, record |
| `--accent-ink` | `#c42200` | red text and links on light ground (≈5.8:1) |
| `--accent-fg` | = `--accent-ink` | alias for older markup |
| `--accent-soft` / `--accent-line` | `#ffe7e0` / `#ffc4b5` | tinted callouts (secondary pages) |
| `--mark` | `#ffd9ce` | `::selection`, highlighted text |
| `--ink`, `--ink-2`, `--ink-3` | `#111113`, `#1c1c1f`, `#2a2a2e` | dark field, overlay, primary button + hover |
| `--ink-text` / `--ink-text-2` | `#f3f3f1` / `#a9a9b0` | text on the dark field |
| `--ink-line` | `rgba(255,255,255,.1)` | hairlines on dark |
| `--font` | Schibsted Grotesk 400–800 (Google Fonts, one request) → Segoe UI | everything |
| `--mono` | Cascadia Mono → Consolas → ui-monospace (system, no download) | key caps, overlay timer/mode label, code only |
| `--r` / `--r-lg` | 12px / 20px | control / sheet radius; buttons and pills are fully round |
| `--wrap` / `--gutter` | 1200px / clamp(16px, 4vw, 40px) | content width / side gutter |
| `--ease` | `cubic-bezier(.16,1,.3,1)` | every transition |
| `--shadow` | 1px contact + 28px/56px soft drop | sheets only |

### Theme roles (light values resolve to the tokens above, so the light page is pixel-identical)

| Token | Light | Use |
|---|---|---|
| `--btn-bg` / `--btn-bg-hover` / `--btn-fg` | `--ink` / `--ink-3` / `--ink-text` | primary pill, selected style tab, `.pg-tag.rec`, pressed key caps, skip link |
| `--field` / `--field-edge` | `--ink` / transparent | closing section + footer / their top hairline |
| `--ov-ring` | none | hairline edge on overlay `.ov`, `.pill`, `.cmd-say` |
| `--logo-ring` / `--logo-radius` | none / 0 | hairline edge on the dark logo tile (header, closing) |
| `--shadow-sm`, `--shadow-menu`, `--shadow-shot` | ink-tinted soft drops | styler/provider sheets, mobile menu, Providers screenshot |
| `--scroll-thumb` | `#c9c9c3` | scrollbar |
| `--ov-bg` | `--ink` | demo overlay capsule + result pill (dark: one step darker than the sheet) |
| `--prose` | `#2b2b30` | running text on secondary pages (`.pg --pg-ink`) |

## Dark theme: the same paper, at night

Not a redesign and not "dark + neon": a near-black ground (close to true black, owner feedback 2026-10-31: the first dark read "greyish"), sheets lifted by small lightness steps (never glow), off-white ink, hairlines stay hairlines, `#FF2E00` still only on the caret, wave and record signal. The demo overlay and result pill use `--ov-bg` `#0c0c0d`, a step *darker* than the sheet they float on, plus a 1px white-14% edge, so the app's capsule still separates; the closing field is a slightly lifted surface with a top hairline instead of a black slab; the primary pill inverts to off-white on near-black (ink is still "the colour of the text"). App icons sit on dark tiles unchanged.

Defined twice in `styles.css`, kept identical: `:root[data-theme="dark"]` (explicit choice) and `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) }` (OS preference). `--ink*`, `--accent`, `--on-accent`, fonts, radii and spacing do not change.

| Token | Dark | Contrast / note |
|---|---|---|
| `--bg` | `#0b0b0a` | page ground |
| `--bg-2` | `#0f0f0e` | |
| `--surface` | `#141413` | sheets, styler card, key caps, tiles, provider sheets |
| `--surface-2` | `#1c1c1a` | hover fills, inline code, table heads |
| `--text` | `#f1f0ec` | 17.3:1 on bg |
| `--text-2` | `#b8b7b0` | 9.8:1 |
| `--text-3` | `#979691` | 6.7:1 on bg, 6.2:1 on surface, 5.8:1 on surface-2 |
| `--prose` | `#d6d5cf` | 13.4:1 |
| `--line` / `--line-2` | `#252523` / `#373733` | hairlines / control borders, both visible on bg and surface |
| `--accent-ink` | `#ff6b4a` | red links + focus ring, 7.0:1 |
| `--accent-soft` / `--accent-line` | `#21130e` / `#4d2417` | tinted callouts |
| `--mark` | `#4a2016` | `::selection`, highlighted text |
| `--btn-bg` / `--btn-bg-hover` / `--btn-fg` | `#f1f0ec` / `#dcdbd5` / `#0b0b0a` | primary pill, 17.3:1 |
| `--field` / `--field-edge` | `#121211` / `#262624` | closing + footer: a lifted field with a top hairline (closing lede 8.0:1, footer meta 5.8:1) |
| `--ov-bg` | `#0c0c0d` (light: `--ink`) | overlay capsule + result pill on the demo sheet |
| `--ov-ring` / `--logo-ring` | `inset 0 0 0 1px rgba(255,255,255,.14)` / `0 0 0 1px rgba(255,255,255,.14)` | edges on dark-on-dark |
| `--shadow*` | black-based, deeper | |
| `--scroll-thumb` | `#373733` | |

Verified 2026-10-31 by a computed-colour walk over every visible text node on all four pages, both themes: no pair under 4.5:1 (3:1 for large text).

### Theme switch

- `.theme-btn` in `.top-cta`, before "Baixar": a 66x36 hairline pill (same height as the header's `btn-sm`), sun on the left and moon on the right both drawn in the track (`--text-3`), and a 28px knob in `--btn-bg` that slides onto the active theme (420ms `--ease`) with that icon in `--btn-fg`. 60px wide under 480px; under 360px it folds to a 36px round button showing the active theme's icon. `aria-label="Tema escuro"` + `aria-pressed`, real `<button>`, focus ring follows the pill. Shown only with JS (`.js`).
- Default follows `prefers-color-scheme`. A click sets `data-theme` on `<html>` and stores `fv-theme` = `light|dark` in localStorage (try/catch). With no stored pick it tracks OS changes live.
- No flash: the tiny inline script in `<head>` (which also adds `.js`) applies a stored pick before first paint.
- `<meta name="theme-color">` exists twice with `media` (light `#f6f6f4`, dark `#0b0b0a`); `<meta name="color-scheme" content="light dark">`. Both are rewritten to the chosen theme when a pick is stored (head script on load, `site.js` on toggle). `:root` carries `color-scheme: light|dark` for native controls and scrollbars.
- `og.png` and the Providers screenshot are the same in both themes.

## Motion

One authored moment per interaction; nothing animates on first paint except the hero demo.

- **Theme change** (the switch's job): the new theme is revealed by a circle growing from the switch's centre, View Transitions API (`document.startViewTransition`), `clip-path: circle()` on `::view-transition-new(root)`, 520ms `cubic-bezier(.22,1,.36,1)`. `site.js` adds `.theme-vt` to `<html>` for its duration, which switches off every `view-transition-name` so the whole page is one snapshot under the clip and turns off the default crossfade. Without the API: `.theme-fade` gives every element a 260ms colour/background/border crossfade for 300ms. With `prefers-reduced-motion: reduce`: instant. Only a click animates; first load and OS changes never do.
- **Page to page** (index, guia, novidades, privacidade): cross-document View Transitions, `@view-transition { navigation: auto; }` inside `@media (prefers-reduced-motion: no-preference)`. The header is its own group (`view-transition-name: site-header`) so it stays put while the page underneath leaves (160ms fade + 6px up, ease-in) and the next one settles in (380ms `--ease`, 80ms delay, from 14px below). A named title group was tried and dropped: two different titles crossfading in place read as a double exposure. Same-origin only; works on any static server; browsers without support simply navigate.
- **Loader**: `site.js` injects one `.nav-progress` (fixed, top, 2px). When an internal link to another page is clicked (not same-page anchors, `target=_blank`, downloads, modified clicks) and the next page has not replaced this one after 120ms, the caret types a line across the top: an ink line (`--text`) grows from the left with the red 2x14px caret (`--accent`) at its head, 9s `cubic-bezier(.1,.8,.2,1)` towards 92% (transform only). Cleared on `pageshow` (back/forward cache). Reduced motion: a static line at 35% with the caret, fading in.
- **Styles showcase**: switching style or Prompt intensity re-runs the 420ms blur-to-sharp `settle` on the said line and the result.

Breakpoints: hero stacks at 960px (the demo stays beside the lede on small laptops); styles and free sections stack at 860px; specs go 2-col at 1080px and 1-col at 720px.

Type scale: hero h1 `clamp(2.55rem, 6.6vw, 5.75rem)`, weight 700, tracking −0.04em, leading .98. Section h2 `clamp(2rem, 3.7vw, 3.15rem)`, −0.035em. Small h2 (`.h-small`) `clamp(1.6rem, 2.5vw, 2.15rem)`. Body 17px/1.55 (16.5px under 720px). More space above headings than below; sections `clamp(64px, 8.5vw, 116px)` vertical with a hairline between them.

## Shared components (all pages)

- `.top` sticky header (solid `--bg`, hairline appears with `.scrolled`), `.wrap`, `.brand` (28px `assets/favicon.svg` + wordmark; optional `.ver`), `.nav#nav` (collapses under 860px into a drop panel toggled by `.menu-btn`), `.top-cta`.
  - Under 480px the header is 60px tall with tighter gaps and `.brand .ver` is hidden, so wordmark and "Baixar" never touch at 320px (applies to every page; markup unchanged).
  - Home only: `<header data-hero-cta>` keeps the header "Baixar" hidden on phones until the hero's own download button scrolls away (`site.js` toggles `.past-hero`). Without JS it is always shown.
- Buttons: `.btn` + `.btn-primary` (ink pill) / `.btn-ghost` (hairline pill) / `.btn-light` and `.btn-ghost-dark` (on the dark field); sizes `.btn-sm` 36px, default 44px, `.btn-lg` 52px. `.cta-row` wraps them.
- `.sig`: the signature, app icon (`assets/logo.svg`, with `--logo-ring`/`--logo-radius`) + "FlowVoice" wordmark, 700, -0.03em. Hero: 40px icon, 24px name, at the top of `.hero-side` (it signs the headline; not an eyebrow, it sits below the heading and names the product). Closing: `.sig-lg`, 56px icon, 26-32px name in `--ink-text`, centred above the closing headline (replaces the old lone 72px logo).
- `.foot-site`: dark ink footer. New markup: `.foot-row` (about + link list) and `.foot-bottom`. The older `.foot-grid` / `.foot-about` markup on secondary pages is still styled.
- `kbd`: white cap with a 2px bottom edge, mono. `code`: mono on `--surface-2`.
- `.label`: small 13px semibold `--text-3` field label. Never as an eyebrow above a heading.
- `[data-reveal]` is retired (always visible); `site.js` still adds `.in` for old markup.
- Browser surfaces: `::selection` uses `--mark`; focus ring 2px `--accent-ink` (light ground) or `--ink-text` (dark field), offset 3px, following each control's own radius (pills stay round; only class-less inline links and `summary` get a 4px corner); scrollbar thumb `--scroll-thumb`. In dark the focus ring is `--accent-ink` `#ff6b4a` (6.6:1).

## Home-only components

- Hero demo `.demo`: `.demo-app` toggle pills (`aria-pressed`), `.demo-pause`, `.sheet` with `.pane[data-pane]`, `.keys`, overlay `.ov` (`.show`, `.listening`), `.pill` (`.show`, `.done`). Driven by `site.js`; stops while off screen, in a hidden tab or paused; reduced motion shows each app's finished state without autoplay.
- `.styler` tablist (roving tabindex, arrows/Home/End) + `.map` per-app rows (buttons that select the style). Each style has its own scene in `site.js` (`STY`): a context line with the app's real icon ("Você falou · no WhatsApp"), what was said (fillers the style removes are struck through in red; none in Só transcrição, which keeps them) and the result, all pt-BR as people really speak:
  - Natural: message to mum on WhatsApp (keeps "tá", "pra"). Profissional: team e-mail in Outlook about the quarterly report. Prompt: asking Claude to fix a JavaScript cart-total function that returns NaN. Cru: a Chrome search, no punctuation (what `text::raw_style` produces). Só transcrição: interview notes, literal with every "né" and "tipo".
  - Prompt shows the app's own intensity control, `.strength` ("Intensidade: Fraco · Médio · Forte", same labels and hints as the app's Styles page and the same rules as `prompt_style_text` in `src-tauri/src/settings.rs`): Fraco keeps the sentences, Médio trims to a first-person request, Forte condenses to short bullets. The control keeps its place (`visibility: hidden`) for the other styles so the description wraps the same.
  - The per-app rows match the scenes (Outlook → Profissional, WhatsApp → Natural, Claude → Prompt). The hero demo scenes match too (Outlook = Profissional e-mail, Slack = Natural team message, ChatGPT = Prompt in first person).
  - `.stack`: the said line and the result each sit in a one-cell grid with hidden `aria-hidden` copies of every variant (added by JS), so the card keeps the tallest size and the tabs never move. There is no translate toggle on the site.
- `.cmd-steps` (three hairline rows), `.facts` (dl), `.specs` (3-col hairline list), `.qa` details.
- `.final` dark closing field with the signature (`.sig-lg`).

## Assets

- App icons: `assets/apps/*.svg` (official brand marks; see `assets/apps/CREDITS.md`), shown on white 40px tiles with a hairline.
- Screenshot: `assets/shot-providers.png` (real app, Providers screen) in the free/offline section; lazy-loaded.
- `assets/og.png` (1200x630): regenerated for the light direction. Source: an HTML composition (headline with red caret, favicon + wordmark, white Outlook sheet, the dark overlay capsule mid-dictation) rendered by headless Edge at exactly 1200x630 via CDP; no stock or generated imagery.
- Favicons and `logo.svg` unchanged.

## Rules

- Red is a signal, not a theme. No red buttons, no red glows, no gradient text.
- No eyebrows/kickers, no section numbers, no icon-card grids, no stat strips.
- Every named app shows its real icon; never letter tiles.
- Copy: short, concrete, pt-BR; product facts only (see PRODUCT.md).
- Windows only (owner decision 2026-10-31): no page mentions Linux, Ubuntu, Debian or `.deb`, and nothing promises them "em breve". The FAQ question is "Em quais sistemas funciona?" → Windows 10 e 11.
- Links: every external link (GitHub, releases, provider consoles, caliberda.com.br, Pix, Sponsors, Vercel policy) has `target="_blank" rel="noopener noreferrer"` on all four pages; internal links and the `FlowVoiceSetup.exe` download (`data-dl`) stay in the same tab.
