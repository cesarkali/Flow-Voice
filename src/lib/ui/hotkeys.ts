// Accelerator helpers: KeyboardEvent -> Tauri accelerator string, and
// accelerator -> human keycap labels.

export const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "OSLeft",
  "OSRight",
]);

const CODE_MAP: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  IntlBackslash: "\\",
  PrintScreen: "PrintScreen",
  ScrollLock: "ScrollLock",
  Pause: "Pause",
  CapsLock: "CapsLock",
  NumpadAdd: "NumAdd",
  NumpadSubtract: "NumSubtract",
  NumpadMultiply: "NumMultiply",
  NumpadDivide: "NumDivide",
  NumpadDecimal: "NumDecimal",
  NumpadEnter: "NumEnter",
};

export function isModifierCode(code: string) {
  return MODIFIER_CODES.has(code);
}

export function modifiersOf(e: KeyboardEvent): string[] {
  const m: string[] = [];
  if (isMac ? e.metaKey : e.ctrlKey) m.push("CommandOrControl");
  if (isMac && e.ctrlKey) m.push("Control");
  if (e.altKey) m.push("Alt");
  if (e.shiftKey) m.push("Shift");
  if (!isMac && e.metaKey) m.push("Super");
  return m;
}

export function keyOf(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (/^Numpad[0-9]$/.test(code)) return "Num" + code.slice(6);
  return CODE_MAP[code] ?? null;
}

/** Returns the accelerator, or null if the event is only modifiers / unsupported. */
export function acceleratorFromEvent(e: KeyboardEvent): { accel: string | null; mods: string[]; needsModifier: boolean } {
  const mods = modifiersOf(e);
  if (isModifierCode(e.code)) return { accel: null, mods, needsModifier: false };
  const k = keyOf(e.code);
  if (!k) return { accel: null, mods, needsModifier: false };
  const isF = /^F\d+$/.test(k) || ["Pause", "ScrollLock", "PrintScreen", "Insert"].includes(k);
  if (!mods.length && !isF) return { accel: null, mods, needsModifier: true };
  return { accel: [...mods, k].join("+"), mods, needsModifier: false };
}

const LABELS: Record<string, string> = {
  CommandOrControl: isMac ? "⌘" : "Ctrl",
  CmdOrCtrl: isMac ? "⌘" : "Ctrl",
  Control: isMac ? "⌃" : "Ctrl",
  Ctrl: "Ctrl",
  Alt: isMac ? "⌥" : "Alt",
  Option: "⌥",
  Shift: isMac ? "⇧" : "Shift",
  Super: isMac ? "⌘" : "Win",
  Meta: isMac ? "⌘" : "Win",
  Command: "⌘",
  Up: "↑",
  Down: "↓",
  Left: "←",
  Right: "→",
  Space: "Space",
  Enter: "Enter",
  Escape: "Esc",
};

export function keycapsOf(accel: string): string[] {
  if (!accel) return [];
  // Split on "+" but keep a literal "+" key (e.g. "Ctrl++").
  const parts = accel.split(/\+(?!$)/);
  return parts.map((p) => LABELS[p] ?? (p.length === 1 ? p.toUpperCase() : p));
}
