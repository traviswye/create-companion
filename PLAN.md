# Create Companion — Scaffold & Build Plan

Companion to `create-companion-scope.md`. That document says *what*; this one says *how we start*,
in what order, and which existing NayaOS assets we reuse. Windows first (the dev machine is
Windows-only), macOS behind a platform trait from day one.

---

## 0. Ground truth from the parent repo (what we already have)

| Asset | Where | Why it matters here |
|---|---|---|
| Tune on-device field map | `docs/module-gestures.md`, `openflow/backend/openflow_backend/device/module_field_map.json` | Tells us which device fields become the F-key transport. Dial rotate is a **keypress pair** at fields `0x22/0x23` (stock: `C_VOL_UP/C_VOL_DOWN`) → write `F24/F23` there and the dial emits discrete CW/CCW keys. `tap:tune:1_finger` = `0x08`, 2-finger swipes `0x10–0x13`, 3-finger swipes `0x1a/0x1b`, taps `0x0e/0x16`. Keypress records carry **modifiers**, so `Shift+F19` namespaces are flashable. |
| Gesture write path | `flash.module_gesture_write()` in the OpenFlow backend | The "flash once" step is already implemented. The companion never talks to the device; OpenFlow does. |
| Gesture enum (83 entries) | `docs/reference/naya-gesture-enum.json` | Source for semantic event names. Touch and Tune expose the same gesture family (tap/double_tap/swipe × 1–4 fingers, pinch/spread). |
| Stock module bindings | `docs/reference/naya-default-module-bindings.json` | Seed for the DEFAULT profile (volume/mute/media/brightness), so out-of-box behavior matches what users had. |
| App shortcut dictionary (5,311 shortcuts, 20 apps, per OS) | `docs/reference/app-shortcuts.json` | Ready-made **preset actions** for Photoshop, Lightroom, After Effects, Illustrator, Blender, Maya, all JetBrains IDEs, Sublime, Unity, Houdini, Nuke. |
| Naya action vocabulary (610 names, per-platform chords) | `docs/reference/action-chords.json` | OS/browser/vscode/media/text actions with validated Win/Mac chords → the "Recommended / System / Navigation / Media" picker categories. |

**Check before Phase 0:** confirm the OpenFlow keycode table encodes `F17–F24` (HID usage `0x6C–0x73`). If it doesn't, adding those entries is a small change in the encoder and is a prerequisite for flashing the transport.

**Constraint:** Track has no on-device keypress fields, so it cannot emit transport keys. Tune is fully supported; Touch stores keypress fields (slot-1 evidence) but its live field map is still unconfirmed. Plan for Tune first, Touch second.

---

## 1. Toolchain to install (dev machine)

- **Rust stable** via `rustup` (MSVC toolchain). Installed 2026-09-03 (cargo 1.98.1).
- **Node 25 / npm** — already present, used only for the Tauri UI.
- `cargo install tauri-cli --version "^2"` (Phase 2).
- Windows: Visual Studio 2022 Build Tools (C++ workload) for the MSVC linker. Installed 2026-09-03. WebView2 runtime ships with Win 11.
- Optional: `cargo-nextest` (faster tests), `cargo-deny` (license/advisory checks in release CI).

---

## 2. Repository layout

```
openflowCompanion/
├── Cargo.toml                    # workspace
├── rust-toolchain.toml           # pin stable
├── crates/
│   ├── companion-core/           # NO OS deps. Pure logic, 100% unit-testable.
│   │   └── src/
│   │       ├── transport.rs      # TransportCode {key: F17..F24, mods} <-> SemanticEvent decoder table
│   │       ├── event.rs          # SemanticEvent (TUNE_CW, LEFT_TOUCH_SWIPE_LEFT, ...), Module, Gesture
│   │       ├── profile.rs        # Profile, AppMatch {windows_exe, macos_bundle}, resolver (app -> profile -> default)
│   │       ├── action.rs         # Action: KeyChord, KeySequence, Media, Scroll, Launch, Command, Noop
│   │       ├── accel.rs          # rotary velocity -> multiplier (None/Light/Medium/Aggressive/Custom)
│   │       ├── state.rs          # press/hold/double-tap/long-press state machine (Phase 3)
│   │       ├── config.rs         # serde + toml schema, versioned, migrations, validation, conflict detection
│   │       └── presets.rs        # embedded preset catalog loader (include_str! of generated JSON)
│   ├── companion-platform/       # trait definitions + per-OS impls behind cfg()
│   │   └── src/
│   │       ├── lib.rs            # traits: InputHook, ForegroundWatcher, ActionSink, Autostart
│   │       ├── windows/          # WH_KEYBOARD_LL hook, SetWinEventHook(EVENT_SYSTEM_FOREGROUND),
│   │       │                     #   QueryFullProcessImageNameW, SendInput (keys + wheel + consumer keys)
│   │       └── macos/            # CGEventTap, NSWorkspace didActivateApplication, CGEventPost (Phase 5)
│   ├── companion-engine/         # binary: create-companion (the 24/7 process)
│   │   └── src/
│   │       ├── main.rs           # single-instance guard, hidden startup, wires everything
│   │       ├── pipeline.rs       # hook -> decoder -> resolver -> executor, on one worker thread
│   │       ├── config_watcher.rs # notify file watcher, atomic swap of Arc<Config>
│   │       ├── tray.rs           # tray-icon + muda menu: Active / Open Config / Pause / Reload / Quit
│   │       ├── ipc.rs            # named pipe (Win) / unix socket (mac): status, detect-input stream, reload
│   │       └── diagnostics.rs    # ring buffer of last N events, opt-in tracing to file
│   └── companion-presets-gen/    # build-time: turns ../presets/*.json into the embedded catalog
├── presets/                      # SOURCE OF TRUTH, hand-editable JSON/TOML
│   ├── apps.json                 # id, display name, windows_exe[], macos_bundle[], icon
│   ├── actions/                  # per-category action catalogs (system, browser, media, navigation, ...)
│   └── profiles/                 # default.toml, chrome.toml, photoshop.toml, resolve.toml, ...
├── tools/
│   └── gen_presets.py            # pulls from ../docs/reference/*.json -> presets/actions/*.json
├── ui/                           # Tauri 2 app create-companion-ui (Phase 2). Not resident.
│   ├── src-tauri/                # thin Rust shell: connects to engine IPC, no business logic
│   └── src/                      # Vite + React (or Svelte): profile list, mapping editor, action picker
├── installer/
│   ├── windows/                  # NSIS or WiX (Tauri bundler can also produce the engine installer)
│   └── macos/                    # dmg config, entitlements, Info.plist (LSUIElement=1)
├── .github/workflows/
│   ├── ci.yml                    # fmt, clippy -D warnings, test (win + mac matrix), cargo-deny
│   └── release.yml               # on tag v*: build both OSes, package, checksums, GitHub Release
├── docs/
│   ├── config-schema.md          # generated from the serde schema
│   └── flash-once.md             # step-by-step: set the transport on the device with OpenFlow
├── create-companion-scope.md
└── PLAN.md
```

Rationale for the split:
- `companion-core` has zero OS dependencies, so every decision (decode, resolve, accelerate, conflict-check) is a plain unit test that runs on any CI runner.
- `companion-platform` is the only crate that touches Win32/Cocoa. macOS lands as a second module without touching core or engine.
- The UI is a separate process that dies on close. The engine exposes IPC; the UI is just a client. This is the scope doc's "no resident Electron/Chromium" requirement.

---

## 3. Key crates (Windows first)

| Need | Crate | Notes |
|---|---|---|
| Win32 API | `windows` | `WH_KEYBOARD_LL`, `SetWinEventHook`, `SendInput`, `GetForegroundWindow`, `QueryFullProcessImageNameW` |
| Tray + menu | `tray-icon` + `muda` | Tauri-team crates, work standalone with a plain Win32 message loop |
| Config | `serde`, `toml`, `schemars` | TOML per the scope doc; schemars gives a JSON schema for the UI and docs |
| Hot reload | `notify` (debounced) | Watch the config file; swap `Arc<Config>` atomically |
| IPC | `interprocess` | Named pipe on Windows, unix socket on macOS, same API |
| Autostart | `auto-launch` | HKCU Run key on Windows, LaunchAgent on macOS; no elevation |
| Logging | `tracing` + `tracing-appender` | Off by default; tray toggles diagnostics |
| Errors | `thiserror` / `anyhow` | |
| macOS (Phase 5) | `core-graphics`, `objc2-app-kit`, `core-foundation` | CGEventTap needs Input Monitoring permission |

---

## 4. Non-obvious design decisions (settle these in Phase 0/1)

1. **Hook callback does almost nothing.** The low-level hook has a hard latency budget (Windows silently unhooks slow callbacks). Inside the callback: check `LLKHF_INJECTED` (ignore our own SendInput), look the vk up in a pre-built `[bool; 256]` reserved table, read modifier state with `GetAsyncKeyState`, push `(vk, mods, timestamp, down/up)` to a channel, return `1` to swallow. All decoding, resolving and executing happens on the worker thread.

2. **Only reserved keys are swallowed.** The reserved table is built from the config's transport section. An F-key not assigned as transport passes through untouched (scope §17 requirement).

3. **Modifier namespaces leak the bare modifier.** With `Shift+F19`, the hook sees `Shift↓ F19↓ F19↑ Shift↑`; we can only decide to swallow at `F19↓`. A lone Shift tap is harmless in nearly every app, but two follow-ons matter:
   - **Defer execution until the transport modifier is released** (or send a synthetic modifier-up first). Otherwise an action like `Ctrl+Tab` fired while the firmware's Ctrl is still down can misfire, and a plain `T` becomes `Ctrl+T`.
   - Prefer unmodified keys for the highest-frequency events (dial CW/CCW) and reserve modifier namespaces for Touch. Decision to make: whether to widen the unmodified pool to `F13–F24` (12 keys) before reaching for modifiers.

4. **Foreground detection is event-driven.** `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` fires on focus change; we resolve the exe name once per change and cache `Arc<Profile>`. Zero polling, so idle CPU stays at ~0%.

5. **Consumer keys via SendInput.** Volume/mute/media map to `VK_VOLUME_UP` etc.; no audio API needed for Phase 1. Horizontal scroll uses `MOUSEEVENTF_HWHEEL`.

6. **Config is versioned from the first commit.** `schema_version = 1` at the top and a migration hook in `config.rs`, so Phase 3/4 additions (acceleration, hold-chords, second Touch) never break an existing file.

7. **Semantic event IDs are stable strings**, not enum ordinals, in the config file (`TUNE_CW`, `LEFT_TOUCH_SWIPE_LEFT`). Transport can later change to vendor HID without touching profiles (scope §23).

8. **Single instance + hidden startup.** Named mutex on Windows; a second launch just asks the running engine (via IPC) to open the UI.

---

## 5. Phased build with concrete deliverables

### Phase 0 — Proof of concept
Goal: one Tune firmware mapping behaves differently in two apps.
- [x] Install rustup; create the workspace (`companion-core`, `companion-platform`, `companion-engine`). Core types + 15 unit tests, clippy clean.
- [x] `WH_KEYBOARD_LL` hook (`companion-platform/src/windows/hook.rs`): swallows only reserved F-keys, ignores injected input by default, minimal callback, dedicated message-pump thread.
- [x] Foreground exe detection at event time (`foreground_exe()`). Event-driven `SetWinEventHook` + cached profile moves to Phase 1.
- [x] Built-in Phase 0 config (`companion-engine/src/phase0.rs`): browsers → Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+T; default → volume up/down/mute with Light acceleration. `SendInput` executor for chords, media keys, wheel, launch, command.
- [x] Tune flashed by the user (F24 CW, F23 CCW, F22 tap). `docs/flash-once.md` still to write.
- [x] Smoke-tested 2026-09-04 with injected F24/F23/F17 (`--allow-injected`): F24 → `TUNE_CW` → VolumeUp, second detent 80 ms later → repeat=3, F23 → VolumeDown, unreserved F17 passed through untouched.
- [x] **Exit criterion (physical), 2026-09-04:** real Tune with `allow_injected=false` in Chrome: CW → Ctrl+Tab, CCW → Ctrl+Shift+Tab, tap → Ctrl+T, all logged and landing in Chrome. Volume half already proven via the Default profile. Detents arrive ~400–500 ms apart at moderate speed, ~40 ms in fast bursts.

### Phase 1 — Minimal engine — DONE 2026-09-04
- [x] `companion-core`: transport table, `SemanticEvent`, `Profile` + resolver, `Action` model, TOML config (`[engine]`, `[transport]`, `[default_profile]`, `[[profiles]]`) with validation, chord parser, bundled presets. 20 unit tests.
- [x] `companion-platform` (Windows): `WH_KEYBOARD_LL` hook + `EVENT_SYSTEM_FOREGROUND` watch on one message-pump thread (cached foreground, no polling), `SendInput` executor incl. hwheel + consumer keys, `release_modifiers`, autostart (HKCU Run via `auto-launch`), single-instance mutex, main-thread message loop + `Waker`.
- [x] Engine: pipeline thread (`Engine` is OS-free and unit-tested with a mock sink, 5 tests), config watcher with 250 ms debounce and atomic swap (bad config → logged, previous kept), tray (Active/Last, open file/folder, reload, pause, start-at-login, quit), single instance, hidden console in release builds, daily log file in `%LOCALAPPDATA%\CreateCompanion\logs`, `--config` / `--no-tray` / `--print-config` / `--allow-injected`.
- [x] `tools/gen_presets.py` → `presets/actions.json` (217 catalog actions from `action-chords.json`: Browser, System, Text, VS Code, Terminal, Files, Media, Scroll) and `presets/default-config.toml` (Default + Browser, Photoshop, Lightroom, Premiere, Resolve, Fusion 360, Blender, VS Code, Discord, Spotify). Embedded via `include_str!`, written to `%APPDATA%\CreateCompanion\config.toml` on first run.
- [x] Modifier-namespace rule: transport modifiers are released before the mapped chord is sent (`modifier_namespace_is_released_before_executing` test).
- [x] CI workflow at `.github/workflows/ci.yml` (inactive until the folder is its own repo).
- [x] Live-verified 2026-09-04: first-run config write, tray start, injected F24 → Ctrl+Tab in Chrome, file edit → "configuration changed, reloading" → "configuration applied", second instance refused, log file created. Release binary 1.3 MB.
- [x] 2026-09-04 follow-up: swipes flashed (F17–F20) → every profile binds the four Tune swipes; new Terminal (Windows Terminal / PowerShell / cmd) and YouTube profiles; `match.window_title` substring rule for website profiles (title read only at event time, most-specific match wins). 12 app profiles.
- Deferred: `docs/flash-once.md`; macOS CI job allowed to fail until Phase 5.

### Phase 2 — Configuration UI (Tauri 2) — DONE 2026-09-04
- [x] `ui/` Tauri 2 + Vite + React/TS app, `create-companion-ui.exe`. Non-resident: it is a plain window that exits on close. Workspace member but not a default member (needs `ui/dist`); build with `cd ui && npm run tauri build` or `cargo build -p create-companion-ui` after `npm run build`.
- [x] Engine IPC (`crates/companion-engine/src/ipc.rs`): named pipe `CreateCompanion.sock` broadcasting newline-delimited JSON — `hello`, `status`, `event`, `config_applied`, `config_rejected`. The UI only listens; config edits go through the file, which the engine hot-reloads.
- [x] Tray: "Open configuration..." launches the UI next to the engine exe (falls back to opening the TOML).
- [x] Screens: profile list (Default + apps, live profile marked), match rules editor (exe / title / bundle chips), mapping table for every transport event with inherited-from-Default display, acceleration selector, searchable action picker (217-entry catalog by category, keyboard shortcut recorder + typed chord validated by the core parser, media, scroll, launch / command, do-nothing, use-default), add application (running windows via `visible_windows()`, browse for exe, title-contains for websites), remove profile.
- [x] Detect Input: live `event` messages show "Detected: Tune / Clockwise in Browser → Ctrl+Tab" with a one-click "Change for <profile>" and a row flash.
- [x] Save: autosave 600 ms after the last edit, atomic write, engine reloads; status shows "Saved · applied by engine ✓" when `config_applied` arrives, or the engine's rejection message.
- Deferred: transport (which F-key per gesture) is edited in the TOML for now; macOS bundle ids are editable but untested.
- [x] 2026-09-04 review round: product renamed **Create Companion** (binaries/repo unchanged); nav has search + Active/Available tabs + star toggles (`Profile.enabled`, disabled profiles keep bindings and never match); Add application pinned; ShortcutMapper import → `presets/apps.json` (29 apps, 4,969 shortcuts) with per-app "<App> actions" tab in the picker; bindings carry a plain-English `name` and the table shows Action + Keys columns.
- [x] 2026-09-04/05 **Catalog expansion** (branch `catalog-expansion`, not merged): `catalog/<id>.json` source files + `tools/validate_catalog.py` + Rust chord gate; **160 entries, 22,622 actions** (systems for Windows/macOS/GNOME/KDE, 7 browsers, 47 sites, Office, DAWs, CAD, IDEs, creative). Four Sonnet QA rounds. See `catalog/REPORT.md` for per-entry counts, skipped targets and caveats. Web twins (`<id>_web`) are generated for apps that also run in a browser.
- [x] 2026-09-05 **Fn + per-OS filtering**: `ModifierSet {ctrl, shift, alt, meta, fn_key}` replaces the old `Modifiers` enum (serialized `"fn+shift"`, legacy `ctrl_shift` accepted); Fn is a legal action modifier (macOS Globe key, sent in Phase 5; Windows rejects it as `Unsupported`) and an accepted transport namespace flagged macOS-only and firmware-unverified. `FunctionKey::available_on_macos()` = F13–F20. UI: transport editor offers Fn, marks F21–F24 Windows-only, export skips Fn with a note; Available list and action picker show only entries/chords for the running OS. Catalog: 57 Fn chords in `mac` columns (macos.json 137 actions).
- [x] 2026-09-05 **Platform toggle**: "This computer / All platforms" switch on the Available list and in the action picker (remembered in localStorage). Default = running OS only; All platforms adds other-OS entries and chords with an OS tag, and the Default profile's picker merges the other systems' shortcuts, so a config can be authored on Windows and moved to a Mac.
- [x] 2026-09-05 **Catalog preview**: clicking an Available entry's name opens a read-only view (match rules, the default mappings Enable would create, searchable shortcut list) with an Enable button; only the star enables in one click.
- [x] 2026-09-05 **Renamed and moved**: product, crates, executables (`create-companion.exe`, `create-companion-ui.exe`), config folder `%APPDATA%\CreateCompanion`, pipe `CreateCompanion.sock`, mutex, autostart entry, manifest and Tauri identifier are all Create Companion; the engine/UI migrate a `NayaCompanion` folder and remove the old Run-key entry on first start. Repository now lives at `D:\CreateCompanion`, GitHub `traviswye/create-companion` (private); `traviswye/naya-companion` is the frozen predecessor. Naya's own vocabulary (`naya_behavior`, module-profile tokens, "Naya Create") is intentionally unchanged.
- [x] 2026-09-05 **Inputs polish**: the add row offers pairs (Dial, Pinch & spread, Scroll up & down, Scroll left & right) that create both halves with two keys at once; pinch/spread export as Naya's `pinch&spread` axis with `-`/`+` halves (pinch = `-`, ASSUMED, verify with the Touch); "(Windows only)" is a dropdown group heading, not part of the selected key; default window 1180x700; sidebar rows show only a shortcut count.
- [x] 2026-09-05 **Profile priority**: resolver ranks matches by specificity (title rule > single executable > group listing several), then config order; the Active list is drag-to-reorder for real ties. Fixes Browser (7 exes, listed first) beating Google Chrome (1 exe).
- [x] 2026-09-05 **Shell-surface profiles**: `task_view` (explorer.exe + "Task View"/"Task Switching", dial = select window, tap = switch, swipes = virtual desktops, swipe down = Esc) and `displayfusion` (6 documented default hotkeys; Alt+Tab handler defaults assumed, unverified). Foreground probe confirmed Task View takes the foreground as a titled explorer.exe window. Catalog: 162 entries.
- [ ] Queued: verify on hardware whether the module firmware can emit Apple Fn (vendor page 0xFF00) — if not, Fn transport stays macOS-keyboard-only.
- [ ] Queued: review and merge `catalog-expansion`; UI category grouping for the Available list (160 entries); Capture One and SOLIDWORKS full lists need in-product exports.
- [x] Website shortcut discovery: done in the catalog expansion (Google apps, GitHub, GitLab, Atlassian, Trello, Asana, Airtable, Canva, Reddit, X, Facebook, LinkedIn, Vimeo, SoundCloud, Twitch, Netflix, Hulu, Disney+, Prime Video, YouTube Music, TikTok, Wikipedia, Amazon, ChatGPT, Feedly, Outlook web, Dropbox web).
- [ ] ~~Queued: website shortcut discovery.~~ Sites are matched by window title today (YouTube). Build a scraper / curated list of keyboard shortcuts for common sites (YouTube, Gmail, Google Docs, Figma, Notion, Twitch, Netflix, GitHub, Jira, Slack web…) into `presets/apps.json` as `kind = "site"` entries with `window_title` rules, so they show under Available.
- [x] 2026-09-04: **Inputs editor** in the UI (pinned button next to Add application): one row per module gesture with modifier + F13–F24 selects, add/remove, duplicate-code refusal; saves to `[transport]`, engine hot-reloads, every profile gains the row. **Learn**: the UI sends `{"cmd":"learn","on":true}` over the (now two-way) pipe, the hook reports the next F13–F24 press (unreserved keys pass through), the engine answers `{"type":"learned","key","mods"}` and disarms. **Export for OpenFlow…**: JSON (`create-companion-inputs` v1) listing each gesture with key, HID usage, modifiers and a Naya-style chord (`LSHIFT + F20`); OpenFlow's module-profile import is the other half, to be shaped to its own JSON. No live link between the two apps by design.
- [x] 2026-09-04: **finger counts + real OpenFlow export.** Event names are `<MODULE>_<GESTURE>[_<n>F]` (`TUNE_TAP_1F`, `LEFT_TOUCH_SWIPE_UP_3F`); a name without a count is an any-count default and the resolver falls back to it, so older configs keep working (`TUNE_PRESS` reads as `TUNE_TAP_1F`). New gestures: pinch, spread, and the split scroll axes. Inputs rows have a Fingers select that renames the event everywhere. Export writes one `openflow.module-profile` v1 file per module (Tune, Left Touch, Right Touch -> TOUCH) with `key` / `shortcut_alias` actions and `value` + `split` pairs for the dial and axes; rows without a finger count are listed as skipped. Reference: `D:/NayaOS/docs/reference/module-profiles/`.
- [ ] Queued: import/export of a single app profile; per-profile "reset to bundled defaults"; decide whether Learn stays after real-module testing; OpenFlow-side Import for module profiles.

### Phase 3 — Tune enhancements
- [ ] `accel.rs` curves + per-mapping multiplier; repeat-count executor.
- [ ] `state.rs`: press/hold, tap/double-tap/long-press, hold+rotate chords (needs key-up from transport; verify the firmware sends up events for the tap field).
- [ ] Config schema v2 with migration.

### Phase 4 — Two Touch modules
- [ ] Left/Right namespaces in the transport table; conflict detection (same transport code assigned twice → validation error surfaced in the UI).
- [ ] Touch field map confirmation via `tools/naya_module_probe.py` (parent repo) on a docked Touch; extend `flash-once.md`.
- [ ] Two-Touch profile templates.

### Phase 5 — macOS
- [ ] `companion-platform/macos`: CGEventTap, NSWorkspace activation notifications, CGEventPost, bundle-ID matching.
- [ ] Menu-bar app (`LSUIElement`), permission onboarding UI (Input Monitoring / Accessibility), login item.
- [ ] Universal build + dmg in `release.yml`; unsigned during dev, notarization job gated on secrets.

### Phase 6 — Integrations
- Plugin trait in `action.rs`; first plugins: run script, OBS WebSocket, MIDI. Out of MVP.

---

## 6. Config file (v1 shape, matches scope §14)

```toml
schema_version = 1

[transport]
reserve = ["F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24"]

[transport.tune]
clockwise = "F24"
counterclockwise = "F23"
press = "F22"          # tap:tune:1_finger
swipe_left = "F20"
swipe_right = "F19"
swipe_up = "F18"
swipe_down = "F17"

[transport.left_touch]
modifier = "shift"     # Shift+F17..F24
swipe_left = "F20"
# ...

[profiles.default]
TUNE_CW  = { action = "system.volume_up", accel = "light" }
TUNE_CCW = { action = "system.volume_down", accel = "light" }
TUNE_PRESS = "system.mute"

[profiles.chrome]
match = { windows_exe = ["chrome.exe", "msedge.exe"], macos_bundle = ["com.google.Chrome"] }
TUNE_CW  = "browser.next_tab"
TUNE_CCW = "browser.previous_tab"
TUNE_PRESS = "browser.new_tab"
LEFT_TOUCH_SWIPE_LEFT = "browser.back"

[profiles.photoshop]
match = { windows_exe = ["Photoshop.exe"], macos_bundle = ["com.adobe.Photoshop"] }
TUNE_CW  = { action = "photoshop.increase_brush_size", accel = "medium" }
TUNE_PRESS = { keys = "B" }                       # inline custom shortcut
```

Action references resolve against the embedded catalog; `{ keys = "Ctrl+Shift+T" }` is the escape hatch for anything not in it.

---

## 7. First commands to run

```powershell
# toolchain
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc

# workspace
cd D:\NayaOS\openflowCompanion
cargo new --lib crates/companion-core
cargo new --lib crates/companion-platform
cargo new --bin crates/companion-engine --name create-companion
# then write the root Cargo.toml [workspace] and rust-toolchain.toml
```

Split out of the NayaOS monorepo into its own repository on 2026-09-04 (`traviswye/create-companion`), so tag-driven release workflows and CI run on their own.
