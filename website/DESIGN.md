# FlowVoice website: design system

Not published: `.vercelignore` keeps this file, PRODUCT.md, `assets/apps/CREDITS.md` and `assets/apps/preview.html` off the site (the files stay in the repo).

## Direction: "the writing surface and the red caret"

The page is a clean sheet you write on, and the only colour that speaks is the app's red text cursor. Light, quiet ground; near-black ink for type and buttons; the real FlowVoice overlay (a dark capsule) floating over that light surface, exactly how it looks over Outlook or Word on a real desktop. Red (`#FF2E00`) is reserved for what is red in the product: the caret, the recording wave, the stop/recording signal. Never for buttons, never for decoration.

Why light: the audience writes e-mail, chats and prompts at a desk in daylight; the product lives on top of white document surfaces. The previous dark + neon-glow look read as the generic AI-landing default, and the owner rejected it.

### Direction contract (home, `index.html`)

- **THESIS**: Show the mechanism, not features. The page refuses the category default of dark SaaS hero + stats strip + feature-card grid + long FAQ.
- **OWN-WORLD**: Off-white ground `#F6F6F4`, white sheets with a soft long shadow, ink `#111113`, one red caret. Schibsted Grotesk (tight, heavy display; plain body). Hairline-separated lists instead of cards. One dark ink field at the end (closing CTA + footer), the colour of the logo tile.
- **STORY**: In one viewport the visitor sees speech become clean text at the cursor, in three real apps (real icons), and the download button. Then: one utterance in five styles + per-app mapping; Command mode; free/offline/private with the real Providers screen; six details; three questions; download.
- **FIRST VIEWPORT**: Two-line headline across the full width, ending in a blinking red caret. Below: left column lede, black "Baixar para Windows" pill, GitHub ghost pill, one meta line. Right (7/11 of the width): app switcher (Outlook, Slack, ChatGPT, pause) above a white sheet showing that app's compose surface; the overlay capsule rises at the bottom, streams live words (uncertain tail in grey), collapses into "Reescrevendo com Groq…", then "✓ Colado · Estilo · App" while the clean text settles at the caret. The markup paints the first frame already listening (overlay up, first six words of scene 0, timer 00:01) and `site.js` resumes from that word, so the first paint never shows an empty compose window; reduced motion replaces it with the finished state.
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

Breakpoints: hero stacks at 960px (the demo stays beside the lede on small laptops); styles and free sections stack at 860px; specs go 2-col at 1080px and 1-col at 720px.

Type scale: hero h1 `clamp(2.55rem, 6.6vw, 5.75rem)`, weight 700, tracking −0.04em, leading .98. Section h2 `clamp(2rem, 3.7vw, 3.15rem)`, −0.035em. Small h2 (`.h-small`) `clamp(1.6rem, 2.5vw, 2.15rem)`. Body 17px/1.55 (16.5px under 720px). More space above headings than below; sections `clamp(64px, 8.5vw, 116px)` vertical with a hairline between them.

## Shared components (all pages)

- `.top` sticky header (solid `--bg`, hairline appears with `.scrolled`), `.wrap`, `.brand` (28px `assets/favicon.svg` + wordmark; optional `.ver`), `.nav#nav` (collapses under 860px into a drop panel toggled by `.menu-btn`), `.top-cta`.
  - Under 480px the header is 60px tall with tighter gaps and `.brand .ver` is hidden, so wordmark and "Baixar" never touch at 320px (applies to every page; markup unchanged).
  - Home only: `<header data-hero-cta>` keeps the header "Baixar" hidden on phones until the hero's own download button scrolls away (`site.js` toggles `.past-hero`). Without JS it is always shown.
- Buttons: `.btn` + `.btn-primary` (ink pill) / `.btn-ghost` (hairline pill) / `.btn-light` and `.btn-ghost-dark` (on the dark field); sizes `.btn-sm` 36px, default 44px, `.btn-lg` 52px. `.cta-row` wraps them.
- `.foot-site`: dark ink footer. New markup: `.foot-row` (about + link list) and `.foot-bottom`. The older `.foot-grid` / `.foot-about` markup on secondary pages is still styled.
- `kbd`: white cap with a 2px bottom edge, mono. `code`: mono on `--surface-2`.
- `.label`: small 13px semibold `--text-3` field label. Never as an eyebrow above a heading.
- `[data-reveal]` is retired (always visible); `site.js` still adds `.in` for old markup.
- Browser surfaces: `::selection` uses `--mark`; focus ring 2px `--accent-ink` (light ground) or `--ink-text` (dark field), offset 3px, following each control's own radius (pills stay round; only class-less inline links and `summary` get a 4px corner); scrollbar thumb `#c9c9c3`.

## Home-only components

- Hero demo `.demo`: `.demo-app` toggle pills (`aria-pressed`), `.demo-pause`, `.sheet` with `.pane[data-pane]`, `.keys`, overlay `.ov` (`.show`, `.listening`), `.pill` (`.show`, `.done`). Driven by `site.js`; stops while off screen, in a hidden tab or paused; reduced motion shows each app's finished state without autoplay.
- `.styler` tablist (roving tabindex, arrows/Home/End) + `.map` per-app rows (buttons that select the style).
- `.cmd-steps` (three hairline rows), `.facts` (dl), `.specs` (3-col hairline list), `.qa` details.
- `.final` dark closing field with the full logo.

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
