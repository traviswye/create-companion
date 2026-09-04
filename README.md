# Naya Companion

Lightweight always-on host engine for the Naya Create's Tune and Touch modules. The keyboard is
flashed **once** so module gestures emit F17–F24; the companion swallows those keys and turns them
into per-application actions (Chrome: switch tabs, Photoshop: brush size, desktop: volume).

- Scope: `naya-companion-scope.md`
- Build plan and phase checklists: `PLAN.md`
- Part of the [NayaOS](https://github.com/traviswye/NayaOS) preservation effort for the Naya Create keyboard.

## Status

Phases 0–2 complete (Windows): engine + tray + configuration window. The Tune is flashed once
(F24 = clockwise, F23 = counterclockwise, F22 = tap, F17–F20 = swipes); everything else is
configured on the host.

## Run

```powershell
cargo run --release            # tray icon + engine; creates %APPDATA%\NayaCompanion\config.toml on first run
cargo run -- --no-tray         # console mode, Ctrl+C to quit
cargo run -- --config x.toml   # use another config file (still hot-reloaded)
cargo run -- --print-config    # dump the bundled default config
cargo run -- --allow-injected  # treat synthetic F-keys as transport (testing without the device)
```

- **Config**: `%APPDATA%\NayaCompanion\config.toml`. Edit and save; the engine reloads within
  a second. A file that fails to parse is logged and ignored, the previous config stays active.
- **Tray menu**: active profile and last action, **Open configuration...** (the UI), edit config
  file, open folder, reload, pause, start at login, quit.
- **Configuration window** (`naya-companion-ui.exe`, Tauri): pick a profile, click an action to
  change it (search the catalog, record a shortcut, media, scroll, launch), set acceleration, add an
  application from the running windows or by title for websites. Edits autosave and the engine
  applies them live. Turn the dial while it is open and the detected input is shown with a
  one-click edit. The window is not resident; close it and only the engine remains.
- **Logs**: `%LOCALAPPDATA%\NayaCompanion\logs\companion.log.<date>` (and stderr in debug
  builds). `RUST_LOG=debug` shows every decoded event; otherwise `[engine] log_level` applies.
- Only F-keys listed under `[transport]` are intercepted. Everything else passes through.

### Bundled profiles

Every profile binds the dial (CW / CCW / press) and the four Tune swipes.

| Profile | Matches | Dial | Swipes |
|---|---|---|---|
| Default | anything else | volume / mute | prev / next track, play-pause, task view |
| Browser | Chrome, Edge, Firefox, Brave, Vivaldi | tabs / new tab | back / forward, zoom |
| YouTube | a browser whose title contains "YouTube" | seek / play-pause | prev / next video, speed |
| Terminal | Windows Terminal, PowerShell, pwsh, cmd | command history / Esc | tabs, font size |
| Photoshop | Photoshop.exe | brush size / brush tool | undo / redo, hardness |
| Lightroom | Lightroom, Lightroom Classic | next / prev photo / zoom | undo / redo, pick / reject |
| Premiere Pro | Adobe Premiere Pro.exe | frame step / play | 5-frame step, timeline zoom |
| DaVinci Resolve | Resolve.exe | frame step / play | 1 s step, timeline zoom |
| Fusion 360 | Fusion360.exe | zoom / fit | undo / redo |
| Blender | blender.exe | frame step / play | undo / redo, keyframes |
| VS Code | Code.exe | editor tabs / palette | back / forward, problems |
| Discord | Discord.exe | channels / mute | servers, deafen |
| Spotify | Spotify.exe | app volume / play | prev / next track, like |

**Website profiles** use `window_title = ["..."]` in the `match` table, combined with the browser
executables. A title rule always beats an executable-only rule, so YouTube wins over Browser when a
YouTube tab is active. Titles are read from the foreground window only at the moment a dial event
arrives, only when some profile has a title rule, and are never logged or stored.

Regenerate the bundled presets with `python tools/gen_presets.py` (reads `reference/action-chords.json`, a snapshot from the NayaOS project; see `reference/ATTRIBUTION.md`).
Your live config is written once, on first run; to pick up new bundled profiles either delete
`%APPDATA%\NayaCompanion\config.toml` and restart, or copy the pieces you want from
`cargo run -- --print-config`.

## Layout

| Crate | Role |
|---|---|
| `crates/companion-core` | OS-free logic: semantic events, transport table, profiles, actions, acceleration, config, chord parsing. Fully unit-tested. |
| `crates/companion-platform` | Win32 implementation: low-level keyboard hook + foreground watch, `SendInput` executor, autostart, single instance, message loop. macOS later. |
| `crates/companion-engine` | The `naya-companion` binary: pipeline thread, config watcher, tray, logging. |
| `presets/` | Generated action catalog and the default config (`tools/gen_presets.py`). |
| `ui/` | Configuration window: Tauri 2 shell (`ui/src-tauri`) + Vite/React frontend (`ui/src`). |

## Develop

```powershell
cargo test                       # engine + core (the UI crate is not a default member)
cargo clippy --all-targets
cargo fmt --all
cd ui; npm install; npm run tauri dev     # UI with hot reload (needs the engine running for live status)
cd ui; npm run build; cd ..; cargo build --release -p naya-companion -p naya-companion-ui --features naya-companion-ui/custom-protocol
```

The engine looks for `naya-companion-ui.exe` next to itself, so build both into the same `target`
directory (as above) or install them side by side.
