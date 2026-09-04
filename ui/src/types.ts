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
  kind: "app" | "site" | "system";
  category?: string;
  /** System entries only: which OS's shortcuts these are. */
  os?: ("windows" | "macos" | "linux")[];
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

export const FUNCTION_KEYS = ["F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24"] as const;
export const MODS: Mods[] = ["none", "shift", "ctrl", "alt", "ctrl_shift"];
export { MODULES, GESTURES, moduleLabel, gestureLabel, gesturesFor, eventLabel, eventSortKey, parseEvent, makeEvent, fingerOptions, takesFingers, fingersLabel, nayaBehavior } from "./events";

export interface WindowInfo {
  exe: string;
  title: string;
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
