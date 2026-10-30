// Scrolls a search result into view and pulses it with the accent color.
// Target lookup: element id first; otherwise the element whose own text equals the label
// (row labels, section headers, card names), widened to its row/card.
import { fold } from "./search-index";

const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

function ownText(el: Element): string {
  let s = "";
  for (const n of el.childNodes) if (n.nodeType === Node.TEXT_NODE) s += n.textContent;
  return s.replace(/\s+/g, " ").trim();
}

export function findTarget(page: HTMLElement, label: string, id?: string): HTMLElement | null {
  if (id) {
    const byId = page.querySelector<HTMLElement>(`#${CSS.escape(id)}`);
    if (byId) return byId;
  }
  const want = fold(label.replace(/\s+/g, " ").trim());
  if (!want) return null;
  // best match: exact text inside a settings row > exact elsewhere > prefix
  let exactRow: HTMLElement | null = null;
  let exact: HTMLElement | null = null;
  let prefix: HTMLElement | null = null;
  for (const el of page.querySelectorAll<HTMLElement>("*")) {
    if (el.closest("[aria-hidden='true']")) continue;
    const own = fold(ownText(el));
    if (!own) continue;
    if (own === want) {
      if (el.closest(".row")) {
        exactRow = el;
        break;
      }
      exact ??= el;
    } else if (!prefix && own.startsWith(want)) prefix = el;
  }
  const hit = exactRow ?? exact ?? prefix;
  if (!hit) return null;
  // widen to something meaningful to highlight, but never a whole page section
  const wide = hit.closest<HTMLElement>(".row, article, li, [role='radio'], button, label, dl > div, .card, section");
  if (wide && page.contains(wide) && wide.offsetHeight < 260) return wide;
  const head = hit.closest<HTMLElement>("header");
  if (head && page.contains(head) && head.offsetHeight < 120) return head;
  return hit;
}

/** Scrolls `target` to the middle of `scroller` and draws a short accent pulse around it. */
export function reveal(scroller: HTMLElement, target: HTMLElement) {
  const sr = scroller.getBoundingClientRect();
  const tr = target.getBoundingClientRect();
  const top = scroller.scrollTop + tr.top - sr.top - Math.max(24, (sr.height - tr.height) / 2 - 40);
  scroller.scrollTo({ top: Math.max(0, top), behavior: reduced() ? "auto" : "smooth" });

  // wait for the smooth scroll to settle, then place the pulse inside the scrolled content
  // (and for entrance animations that move the target, e.g. the page stagger, to finish)
  let last = -1;
  let still = 0;
  const started = performance.now();
  const moving = () =>
    document
      .getAnimations()
      .some((a) => a.playState === "running" && a.effect instanceof KeyframeEffect && a.effect.target instanceof Node && a.effect.target.contains(target) && a.effect.getComputedTiming().iterations !== Infinity);
  const step = () => {
    const now = scroller.scrollTop;
    still = now === last ? still + 1 : 0;
    last = now;
    if ((still < 3 || moving()) && performance.now() - started < 1400) return void requestAnimationFrame(step);
    pulse(scroller, target);
  };
  requestAnimationFrame(step);
  target.querySelector<HTMLElement>("input, button, [tabindex='0'], [role='switch'], [role='combobox']")?.focus({
    preventScroll: true,
  });
}

function pulse(scroller: HTMLElement, target: HTMLElement) {
  const host = scroller.firstElementChild as HTMLElement | null;
  if (!host) return;
  const hr = host.getBoundingClientRect();
  const tr = target.getBoundingClientRect();
  const pad = 6;
  const ring = document.createElement("div");
  ring.className = "fv-hit";
  ring.setAttribute("aria-hidden", "true");
  const s = ring.style;
  s.left = `${tr.left - hr.left - pad - 6}px`;
  s.top = `${tr.top - hr.top - pad}px`;
  s.width = `${tr.width + pad * 2 + 6}px`;
  s.height = `${tr.height + pad * 2}px`;
  host.appendChild(ring);
  if (reduced()) {
    setTimeout(() => ring.remove(), 1400);
    return;
  }
  const a = ring.animate(
    [
      { opacity: 0, transform: "scale(1.03)" },
      { opacity: 1, transform: "scale(1)", offset: 0.16 },
      { opacity: 1, transform: "scale(1)", offset: 0.62 },
      { opacity: 0, transform: "scale(1)" },
    ],
    { duration: 1500, easing: "cubic-bezier(0.2, 0.7, 0.2, 1)" },
  );
  a.onfinish = () => ring.remove();
}
