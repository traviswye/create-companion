// Semantic event names, mirroring companion-core/src/event.rs:
//   <MODULE>_<GESTURE>[_<n>F]   e.g. TUNE_CW, TUNE_TAP_1F, LEFT_TOUCH_SWIPE_UP_3F
// Legacy TUNE_PRESS is the one-finger tap; a name without a count is "any count".

export const MODULES = ["TUNE", "LEFT_TOUCH", "RIGHT_TOUCH"] as const;
export type ModuleId = (typeof MODULES)[number];

export const GESTURES = [
  "CW",
  "CCW",
  "TAP",
  "DOUBLE_TAP",
  "SWIPE_LEFT",
  "SWIPE_RIGHT",
  "SWIPE_UP",
  "SWIPE_DOWN",
  "PINCH",
  "SPREAD",
  "SCROLL_LEFT",
  "SCROLL_RIGHT",
  "SCROLL_UP",
  "SCROLL_DOWN",
] as const;
export type GestureId = (typeof GESTURES)[number];

const MODULE_LABEL: Record<ModuleId, string> = { TUNE: "Tune", LEFT_TOUCH: "Left Touch", RIGHT_TOUCH: "Right Touch" };
const GESTURE_LABEL: Record<GestureId, string> = {
  CW: "Clockwise",
  CCW: "Counterclockwise",
  TAP: "Tap",
  DOUBLE_TAP: "Double tap",
  SWIPE_LEFT: "Swipe left",
  SWIPE_RIGHT: "Swipe right",
  SWIPE_UP: "Swipe up",
  SWIPE_DOWN: "Swipe down",
  PINCH: "Pinch",
  SPREAD: "Spread",
  SCROLL_LEFT: "Scroll left",
  SCROLL_RIGHT: "Scroll right",
  SCROLL_UP: "Scroll up",
  SCROLL_DOWN: "Scroll down",
};

export interface ParsedEvent {
  module: ModuleId;
  gesture: GestureId;
  fingers: number | null;
}

export function parseEvent(id: string): ParsedEvent | null {
  const module = MODULES.find((m) => id.startsWith(m + "_"));
  if (!module) return null;
  let rest = id.slice(module.length + 1);
  let fingers: number | null = null;
  const m = /^(.*)_([1-4])F$/.exec(rest);
  if (m) {
    rest = m[1];
    fingers = Number(m[2]);
  }
  if (rest === "PRESS") return { module, gesture: "TAP", fingers: 1 };
  if (!(GESTURES as readonly string[]).includes(rest)) return null;
  const gesture = rest as GestureId;
  if (!takesFingers(gesture) && fingers !== null) return null;
  if (!gestureAvailable(module, gesture)) return null;
  return { module, gesture, fingers };
}

export function makeEvent(module: ModuleId, gesture: GestureId, fingers: number | null): string {
  return `${module}_${gesture}${takesFingers(gesture) && fingers ? `_${fingers}F` : ""}`;
}

export function takesFingers(g: GestureId): boolean {
  return g !== "CW" && g !== "CCW";
}

export function gestureAvailable(module: ModuleId, g: GestureId): boolean {
  return module === "TUNE" || (g !== "CW" && g !== "CCW");
}

export function gesturesFor(module: ModuleId): GestureId[] {
  return GESTURES.filter((g) => gestureAvailable(module, g));
}

/** Finger counts a gesture can be flashed for. Pinch/spread need two hands' worth. */
export function fingerOptions(g: GestureId): number[] {
  if (!takesFingers(g)) return [];
  return g === "PINCH" || g === "SPREAD" ? [2, 3, 4] : [1, 2, 3, 4];
}

export function moduleLabel(m: string): string {
  return (MODULE_LABEL as Record<string, string>)[m] ?? m;
}
export function gestureLabel(g: string): string {
  return (GESTURE_LABEL as Record<string, string>)[g] ?? g;
}
export function fingersLabel(n: number | null): string {
  return n === null ? "" : n === 1 ? "1 finger" : `${n} fingers`;
}

/** `TUNE_SWIPE_LEFT_3F` -> ["Tune", "Swipe left", "3 fingers"] */
export function eventParts(id: string): [string, string, string] {
  const p = parseEvent(id);
  if (!p) return [id, "", ""];
  return [moduleLabel(p.module), gestureLabel(p.gesture), fingersLabel(p.fingers)];
}

/** `TUNE_SWIPE_LEFT_3F` -> ["Tune", "Swipe left · 3 fingers"] */
export function eventLabel(id: string): [string, string] {
  const [m, g, f] = eventParts(id);
  return [m, f ? `${g} · ${f}` : g];
}

/** Stable display order: module, then gesture, then finger count. */
export function eventSortKey(id: string): number {
  const p = parseEvent(id);
  if (!p) return 99999;
  return MODULES.indexOf(p.module) * 1000 + GESTURES.indexOf(p.gesture) * 10 + (p.fingers ?? 0);
}

/**
 * Naya behavior string for an event, plus the half of a direction pair it is
 * (`"-"` or `"+"`), or null when a finger count is needed but missing.
 * Mirrors SemanticEvent::naya_behavior in companion-core.
 */
export function nayaBehavior(id: string): { behavior: string; half: "-" | "+" | null } | null {
  const p = parseEvent(id);
  if (!p) return null;
  const mod = p.module === "TUNE" ? "tune" : "touch";
  if (p.gesture === "CW" || p.gesture === "CCW") return { behavior: `rotate:${mod}:dial`, half: p.gesture === "CW" ? "+" : "-" };
  if (p.fingers === null) return null;
  const q = p.fingers === 1 ? "1_finger" : `${p.fingers}_fingers`;
  const table: Record<string, [string, "-" | "+" | null]> = {
    TAP: ["tap", null],
    DOUBLE_TAP: ["double_tap", null],
    SWIPE_LEFT: ["swipe_left", null],
    SWIPE_RIGHT: ["swipe_right", null],
    SWIPE_UP: ["swipe_up", null],
    SWIPE_DOWN: ["swipe_down", null],
    PINCH: ["pinch", null],
    SPREAD: ["spread", null],
    SCROLL_LEFT: ["horizontal", "-"],
    SCROLL_RIGHT: ["horizontal", "+"],
    SCROLL_UP: ["vertical", "-"],
    SCROLL_DOWN: ["vertical", "+"],
  };
  const [g, half] = table[p.gesture];
  return { behavior: `${g}:${mod}:${q}`, half };
}
