// Mirrors companion-core's serde shapes (see crates/companion-core/src/config.rs).

export type Mods = "none" | "shift" | "ctrl" | "alt" | "ctrl_shift";
export interface TransportCode {
  key: string;
  mods?: Mods;
}

export type MediaKey =
  | "volume_up"
  | "volume_down"
  | "mute"
  | "play_pause"
  | "next_track"
  | "previous_track"
  | "brightness_up"
  | "brightness_down";

export type ScrollDirection = "up" | "down" | "left" | "right";

export type Action =
  | { type: "noop" }
  | { type: "keys"; chord: string }
  | { type: "sequence"; chords: string[] }
  | { type: "media"; key: MediaKey }
  | { type: "scroll"; direction: ScrollDirection; lines?: number }
  | { type: "launch"; program: string; args?: string[] }
  | { type: "command"; command: string };

export type Accel = "none" | "light" | "medium" | "aggressive";

export interface Binding {
  action: Action;
  accel?: Accel;
  /** Plain-English label ("Increase Brush Size"); the action holds the keys. */
  name?: string;
}

export interface AppMatch {
  windows_exe: string[];
  macos_bundle: string[];
  window_title: string[];
}

export interface Profile {
  name: string;
  /** Unstarred profiles keep their bindings but never match. */
  enabled?: boolean;
  match: AppMatch;
  bindings: Record<string, Binding>;
}

/** One shortcut an application exposes (presets/apps.json). */
export interface AppAction {
  id: string;
  name: string;
  context: string;
  windows?: Action;
  mac?: Action;
}

/** An application or website the catalog knows about (presets/apps.json). */
export interface AppEntry {
  id: string;
  name: string;
  kind: "app" | "site";
  match: AppMatch;
  defaults: Record<string, Binding>;
  actions: AppAction[];
  source?: string;
}

export interface Config {
  schema_version: number;
  engine: { start_at_login: boolean; log_level: string };
  transport: Record<string, TransportCode>;
  default_profile: Profile;
  profiles: Profile[];
}

export interface CatalogEntry {
  id: string;
  name: string;
  category: string;
  windows?: Action;
  mac?: Action;
}

export interface EngineMsg {
  type:
    | "hello"
    | "status"
    | "event"
    | "config_applied"
    | "config_rejected"
    | "learned"
    | "connected"
    | "disconnected";
  version?: string;
  config?: string;
  profile?: string;
  paused?: boolean;
  event?: string;
  action?: string;
  repeat?: number;
  error?: string;
  /** `learned`: the key and modifier namespace the module sent. */
  key?: string;
  mods?: Mods;
}

export const MODULES = ["TUNE", "LEFT_TOUCH", "RIGHT_TOUCH"] as const;
export const GESTURES = ["CW", "CCW", "PRESS", "TAP", "DOUBLE_TAP", "SWIPE_LEFT", "SWIPE_RIGHT", "SWIPE_UP", "SWIPE_DOWN"] as const;
export const FUNCTION_KEYS = ["F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24"] as const;
export const MODS: Mods[] = ["none", "shift", "ctrl", "alt", "ctrl_shift"];

export function moduleLabel(m: string): string {
  return MODULE_LABEL[m] ?? m;
}
export function gestureLabel(g: string): string {
  return GESTURE_LABEL[g] ?? g;
}
/** Dial rotation only exists on the Tune. */
export function gesturesFor(module: string): readonly string[] {
  return module === "TUNE" ? GESTURES : GESTURES.filter((g) => g !== "CW" && g !== "CCW");
}

export interface WindowInfo {
  exe: string;
  title: string;
}

const MODULE_LABEL: Record<string, string> = {
  TUNE: "Tune",
  LEFT_TOUCH: "Left Touch",
  RIGHT_TOUCH: "Right Touch",
};
const GESTURE_LABEL: Record<string, string> = {
  CW: "Clockwise",
  CCW: "Counterclockwise",
  PRESS: "Press",
  TAP: "Tap",
  DOUBLE_TAP: "Double tap",
  SWIPE_LEFT: "Swipe left",
  SWIPE_RIGHT: "Swipe right",
  SWIPE_UP: "Swipe up",
  SWIPE_DOWN: "Swipe down",
};

/** `TUNE_SWIPE_LEFT` -> ["Tune", "Swipe left"] */
export function eventLabel(id: string): [string, string] {
  for (const m of Object.keys(MODULE_LABEL)) {
    if (id.startsWith(m + "_")) {
      const g = id.slice(m.length + 1);
      return [MODULE_LABEL[m], GESTURE_LABEL[g] ?? g];
    }
  }
  return [id, ""];
}

/** Stable display order: module, then dial, press, taps, swipes. */
const GESTURE_ORDER = Object.keys(GESTURE_LABEL);
const MODULE_ORDER = Object.keys(MODULE_LABEL);
export function eventSortKey(id: string): number {
  const [m, g] = eventLabel(id);
  const mi = MODULE_ORDER.findIndex((k) => MODULE_LABEL[k] === m);
  const gi = GESTURE_ORDER.findIndex((k) => GESTURE_LABEL[k] === g);
  return (mi < 0 ? 9 : mi) * 100 + (gi < 0 ? 99 : gi);
}

const MEDIA_LABEL: Record<MediaKey, string> = {
  volume_up: "Volume up",
  volume_down: "Volume down",
  mute: "Mute",
  play_pause: "Play / pause",
  next_track: "Next track",
  previous_track: "Previous track",
  brightness_up: "Brightness up",
  brightness_down: "Brightness down",
};

export function describeAction(a: Action | undefined): string {
  if (!a) return "";
  switch (a.type) {
    case "noop":
      return "Do nothing";
    case "keys":
      return a.chord;
    case "sequence":
      return a.chords.join(", ");
    case "media":
      return MEDIA_LABEL[a.key] ?? a.key;
    case "scroll":
      return `Scroll ${a.direction}${(a.lines ?? 1) > 1 ? ` ×${a.lines}` : ""}`;
    case "launch":
      return `Launch ${a.program}${a.args?.length ? " " + a.args.join(" ") : ""}`;
    case "command":
      return `Run: ${a.command}`;
  }
}

export function transportLabel(t: TransportCode): string {
  const m = t.mods && t.mods !== "none" ? t.mods.replace("ctrl_shift", "Ctrl+Shift").replace(/^\w/, (c) => c.toUpperCase()) + "+" : "";
  return m + t.key;
}
