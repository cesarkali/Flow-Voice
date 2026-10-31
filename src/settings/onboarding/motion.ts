// Motion primitives for the onboarding wizard (transform/opacity only).
import type { TransitionConfig } from "svelte/transition";

/**
 * Damped spring on t ∈ [0, 1] (ζ ≈ 0.7: one soft ~5% overshoot, settled by t = 1).
 * Used both as a Svelte easing and, sampled, as a CSS `linear()` easing.
 */
export function spring(t: number, zeta = 0.7, w = 9.5): number {
  if (t <= 0) return 0;
  if (t >= 1) return 1;
  const wd = w * Math.sqrt(1 - zeta * zeta);
  return 1 - Math.exp(-zeta * w * t) * (Math.cos(wd * t) + ((zeta * w) / wd) * Math.sin(wd * t));
}
/** A gentler spring for big moves (progress fill, ambient glow). */
export const softSpring = (t: number) => spring(t, 0.82, 8);

/** `linear(...)` CSS easing that follows `fn` (Chromium / WebView2 support it natively). */
export function cssLinear(fn: (t: number) => number, n = 36): string {
  const pts: string[] = [];
  for (let i = 0; i <= n; i++) pts.push(fn(i / n).toFixed(4));
  return `linear(${pts.join(", ")})`;
}

export const reduceMotion = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

const inCubic = (x: number) => x * x * x;

/** Incoming step: springs in from the travel direction, growing from 97%. */
export function stepIn(_n: Element, { d }: { d: number }): TransitionConfig {
  if (reduceMotion) return { duration: 0 };
  return {
    delay: 70,
    duration: 720,
    easing: spring,
    css: (k, u) => `opacity:${Math.min(1, k * 1.8)};transform:translate3d(${u * 72 * d}px,0,0) scale(${0.965 + 0.035 * k})`,
  };
}

/** Outgoing step: quick fade, drifts the other way and recedes slightly. */
export function stepOut(_n: Element, { d }: { d: number }): TransitionConfig {
  if (reduceMotion) return { duration: 0 };
  return {
    duration: 200,
    easing: inCubic,
    css: (k, u) => `opacity:${k};transform:translate3d(${-u * 44 * d}px,0,0) scale(${1 - 0.03 * u});pointer-events:none`,
  };
}
