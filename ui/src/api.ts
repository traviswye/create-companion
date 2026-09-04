import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppEntry, CatalogEntry, Config, EngineMsg, WindowInfo } from "./types";

export interface Loaded {
  path: string;
  config: Config;
  created: boolean;
}

export const api = {
  loadConfig: () => invoke<Loaded>("load_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  defaultConfig: () => invoke<Config>("default_config"),
  actionCatalog: async () => {
    const v = await invoke<{ actions: CatalogEntry[] }>("action_catalog");
    return v.actions;
  },
  appCatalog: async () => {
    const v = await invoke<{ apps: AppEntry[] }>("app_catalog");
    return v.apps;
  },
  runningWindows: () => invoke<WindowInfo[]>("running_windows"),
  validateChord: (chord: string) => invoke<string>("validate_chord", { chord }),
  openConfigFolder: () => invoke<void>("open_config_folder"),
  engineState: () => invoke<{ connected: boolean; hello?: EngineMsg; status?: EngineMsg }>("engine_state"),
  engineSend: (command: Record<string, unknown>) => invoke<void>("engine_send", { command }),
  writeTextFile: (path: string, contents: string) => invoke<void>("write_text_file", { path, contents }),
  onEngine: (cb: (m: EngineMsg) => void): Promise<UnlistenFn> =>
    listen<EngineMsg>("engine", (e) => cb(e.payload)),
};
