# Naya Companion — Scoping Document

## 1. Project Summary

**Naya Companion** is a lightweight, always-running desktop utility for Windows and macOS that gives the Naya Create's **Tune** and **Touch** modules application-aware behavior without requiring repeated keyboard firmware flashes.

The keyboard firmware is configured **once** so a small set of Tune/Touch events emit otherwise-unused function keys, primarily **F17–F24**. Naya Companion intercepts those events, determines which application is currently active, and translates the event into a configurable action appropriate for that application.

Example:

```text
Tune clockwise -> F24
Tune counterclockwise -> F23
Tune press -> F22

F24 while Chrome is active    -> Next tab
F24 while Photoshop is active -> Increase brush size
F24 while Resolve is active   -> Next frame
F24 on desktop/default        -> Volume up
```

The companion application should be small, offline-first, low-overhead, and installable from a normal **GitHub Release installer package**.

---

## 2. Primary Goals

1. **Flash the keyboard once**
   - Configure Tune/Touch gestures to emit a stable set of F17–F24 events.
   - Avoid reflashing the keyboard for ordinary behavior changes.

2. **Move contextual behavior to the host**
   - Determine the foreground application.
   - Map the same hardware event differently depending on the active app.

3. **Provide a lightweight customization UI**
   - Users can select an application.
   - Users can assign actions to Tune/Touch events.
   - Changes apply immediately without reconnecting or reflashing the keyboard.

4. **Remain lightweight enough to run 24/7**
   - Near-zero idle CPU usage.
   - Small memory footprint.
   - No Electron/Chromium runtime.
   - No cloud dependency.

5. **Support both Tune and Touch**
   - Tune: clockwise, counterclockwise, press, and any exposed touch gestures.
   - Touch: taps/swipes/other discrete gestures exposed by the current Naya firmware.
   - Support systems with **two Touch modules**.

6. **Remain useful even if Naya firmware never becomes open**
   - The host application should depend only on standard HID events the shipping firmware can already emit.

---

## 3. Non-Goals for Initial Release

The first release does **not** need to:

- Replace Naya firmware.
- Modify the MCUboot bootloader.
- Control Tune haptic detent strength or Hapticore motor behavior.
- Recover raw X/Y touch coordinates if Naya firmware does not expose them.
- Implement a full NayaFlow replacement.
- Provide cloud synchronization.
- Require accounts, telemetry, or online services.
- Implement application-specific APIs where ordinary keyboard/mouse shortcuts are sufficient.
- Support every possible HID usage on day one.

---

## 4. Recommended Technology Stack

### Always-Running Engine

**Rust** is the preferred implementation language.

Reasons:

- Native executable.
- Low memory usage.
- Near-zero idle CPU.
- No garbage-collected runtime.
- Good Windows and macOS platform API access.
- Straightforward event timing for rotary acceleration.
- Easy packaging as a signed/native desktop application later.
- Can share most business logic across Windows and macOS.

Target idle resource budget:

| Resource | Target |
|---|---:|
| Idle CPU | ~0% |
| RAM | 5–20 MB preferred |
| Installed size | <30 MB preferred |
| Network usage | None |
| Admin/root required | No |

### Configuration UI

Preferred options:

1. **Tauri** frontend with Rust backend, or
2. A small native Rust GUI toolkit if it provides a sufficiently polished UI.

The UI should **not remain resident** after the configuration window closes.

Architecture:

```text
naya-companion      <- always-running background engine
naya-companion-ui   <- opened only when configuration is needed
```

A single binary may provide both roles if implementation is simpler, provided closing the main window leaves only the lightweight background process active.

---

## 5. Input Transport

### Initial Transport

The Naya Create firmware emits unused function keys.

Suggested base event range:

```text
F17
F18
F19
F20
F21
F22
F23
F24
```

The application should treat these as **transport codes**, not semantic actions.

Example:

```text
F24 != "Volume Up"
F24 == "Tune Clockwise"
```

The semantic meaning is stored separately.

### Suggested Tune Mapping

Initial Tune mapping:

| Physical Event | HID Transport |
|---|---|
| Counterclockwise | F23 |
| Clockwise | F24 |
| 2 finger tap | F22 |
| 1 finger tap tap | F21 |
| Swipe left | F20 |
| Swipe right | F19 |
| Swipe up | F18 |
| Swipe down | F17 |

Exact assignments should remain configurable because the Naya firmware's exposed gesture set may vary.

### Multiple Modules

Eight plain F-keys are not sufficient for every event from:

- Tune
- Left Touch
- Right Touch

Therefore the transport layer should support namespacing.

Possible initial approach:

```text
Tune        -> F17–F24
Left Touch  -> Shift+F17–F24
Right Touch -> Ctrl+F17–F24
```

The background engine should intercept and consume these combinations so the transport keys/modifiers do not leak to the foreground application.

The UI should display semantic names such as:

```text
Tune / Clockwise
Left Touch / Swipe Left
Right Touch / Two-Finger Tap
```

The user should never need to care that an event is encoded internally as `Ctrl+F19`.

### Future Transport

If open firmware becomes available later, the transport may be upgraded to:

- Vendor-defined HID reports.
- A custom HID usage page.
- Direct device communication.

The application-level semantic event IDs should remain unchanged so existing user profiles continue working.

---

## 6. Semantic Event Model

Transport codes are decoded into semantic events.

Suggested internal naming:

```text
TUNE_CW
TUNE_CCW
TUNE_PRESS
TUNE_TOUCH_TAP
TUNE_SWIPE_LEFT
TUNE_SWIPE_RIGHT
TUNE_SWIPE_UP
TUNE_SWIPE_DOWN

LEFT_TOUCH_TAP
LEFT_TOUCH_DOUBLE_TAP
LEFT_TOUCH_SWIPE_LEFT
LEFT_TOUCH_SWIPE_RIGHT
LEFT_TOUCH_SWIPE_UP
LEFT_TOUCH_SWIPE_DOWN

RIGHT_TOUCH_TAP
RIGHT_TOUCH_DOUBLE_TAP
RIGHT_TOUCH_SWIPE_LEFT
RIGHT_TOUCH_SWIPE_RIGHT
RIGHT_TOUCH_SWIPE_UP
RIGHT_TOUCH_SWIPE_DOWN
```

The semantic layer should not assume a specific transport key.

---

## 7. Core Runtime Architecture

```text
Naya Create
    |
    | F17-F24 / namespaced HID events
    v
Input Capture
    |
    v
Transport Decoder
    |
    v
Semantic Event
    |
    +----------------------+
    |                      |
    v                      v
Foreground App         Event State
Detection              / Timing
    |                      |
    +----------+-----------+
               |
               v
        Profile Resolver
               |
               v
         Action Executor
               |
               v
    Keyboard / Mouse / OS
```

### Required Runtime Components

1. **Input Capture**
   - Global capture of configured transport keys.
   - Ability to suppress transport keys from reaching normal applications.
   - Preserve ordinary F17–F24 use when the user chooses not to reserve them.

2. **Transport Decoder**
   - Convert F-key/modifier combinations into semantic Naya events.

3. **Foreground Application Detector**
   - Windows: active window -> process executable.
   - macOS: frontmost application -> bundle identifier.
   - Event-driven where possible rather than high-frequency polling.

4. **Profile Resolver**
   - Match active application to configured profile.
   - Fall back to a global/default profile.

5. **Action Executor**
   - Send keyboard shortcuts.
   - Send media/system controls.
   - Send mouse wheel and horizontal scroll.
   - Launch programs/scripts.
   - Execute future application-specific plugins.

6. **Config Watcher**
   - Reload configuration immediately when changed by the UI.
   - No daemon restart required.

---

## 8. Application Profiles

Each application can have its own event mappings.

Example:

```text
DEFAULT
Tune CW       -> Volume Up
Tune CCW      -> Volume Down
Tune Press    -> Mute

CHROME
Tune CW       -> Next Tab
Tune CCW      -> Previous Tab
Tune Press    -> New Tab
Touch Left    -> Browser Back
Touch Right   -> Browser Forward

PHOTOSHOP
Tune CW       -> Increase Brush Size
Tune CCW      -> Decrease Brush Size
Tune Press    -> Select Brush Tool
Touch Left    -> Undo
Touch Right   -> Redo

DAVINCI RESOLVE
Tune CW       -> Next Frame
Tune CCW      -> Previous Frame
Tune Press    -> Play/Pause
Touch Up      -> Timeline Zoom In
Touch Down    -> Timeline Zoom Out
```

Profiles should be identified by:

### Windows

```text
chrome.exe
Photoshop.exe
Resolve.exe
Fusion360.exe
Code.exe
```

### macOS

Prefer bundle identifiers:

```text
com.google.Chrome
com.adobe.Photoshop
com.blackmagic-design.DaVinciResolve
com.microsoft.VSCode
```

Human-readable application names should be displayed in the UI.

---

## 9. Initial Target Applications

The initial action presets should cover common applications without requiring custom APIs.

### Browsers

- Chrome
- Edge
- Firefox
- Safari

Preset actions:

- Next/previous tab.
- New tab.
- Close tab.
- Browser back/forward.
- Page up/down.
- Zoom in/out.
- Vertical/horizontal scroll.

### Adobe / Creative

- Photoshop
- Lightroom
- Premiere Pro

Preset examples:

- Brush size.
- Brush hardness where shortcut-accessible.
- Undo/redo.
- Timeline/frame navigation.
- Timeline zoom.
- Playback.
- Tool switching.

### Video

- DaVinci Resolve
- Final Cut Pro
- VLC
- YouTube/browser media

Preset examples:

- Frame forward/back.
- Timeline scrub.
- Play/pause.
- Volume.
- Timeline zoom.
- Jump forward/back.

### CAD / 3D

- Fusion 360
- Blender
- SolidWorks where shortcut-compatible

Preset examples:

- Zoom.
- Timeline/history navigation.
- Tool shortcuts.
- View navigation through keyboard/mouse emulation.

### Development

- VS Code
- Visual Studio
- JetBrains IDEs

Preset examples:

- Next/previous tab.
- Navigate errors.
- Navigate search results.
- Font size.
- Undo/redo.
- Terminal history.

### Communication

- Discord
- Teams
- Zoom

Preset examples:

- Mute.
- Deafen.
- Volume.
- Push-to-talk or configured shortcut.
- Channel/navigation shortcuts where available.

### Media

- Spotify
- Apple Music
- Plex/Jellyfin clients

Preset examples:

- Volume.
- Previous/next track.
- Play/pause.
- Seek where available.

### OS / Default Profile

- Volume.
- Mute.
- Brightness.
- Media.
- App switching.
- Virtual desktops / Spaces.
- Scrolling.
- Zoom.
- Window management shortcuts.

---

## 10. Action Types

### Phase 1 Actions

The first release should support:

#### Keyboard

- Single key.
- Modifier + key.
- Arbitrary shortcut.
- Key sequence.
- Key down / key up where needed.

#### Consumer / System

- Volume up/down.
- Mute.
- Play/pause.
- Previous/next media.
- Brightness up/down where supported.

#### Application / OS

- Launch application.
- Open file/URL.
- Run command/script.
- Switch applications.
- Switch desktop/Space using OS shortcuts.

### Future Actions

- Per-application Windows audio mixer.
- MIDI output.
- OBS WebSocket actions.
- Home Assistant actions.
- Resolve scripting API.
- Adobe integrations.
- macOS Shortcuts.
- PowerShell/AppleScript actions.
- User plugins.

---

## 11. Tune Rotary Acceleration

Tune rotation should support host-side acceleration without any keyboard firmware changes.

The engine measures time between clockwise or counterclockwise events.

Example default curve:

| Event Interval | Multiplier |
|---|---:|
| >250 ms | 1x |
| 150–250 ms | 2x |
| 80–150 ms | 4x |
| <80 ms | 8x |

Each mapping can choose:

```text
None
Light
Medium
Aggressive
Custom
```

Example:

```text
Resolve:
Slow turn -> 1 frame/event
Fast turn -> 8 frames/event

Photoshop:
Slow turn -> brush +/- 1
Fast turn -> brush +/- 10

Volume:
Slow turn -> +/- 1 step
Fast turn -> +/- 5 steps
```

Acceleration should be optional because some actions, such as tab switching, are best kept one-action-per-detent.

---

## 12. Press / Hold / Compound Input

If Tune press exposes separate key-down and key-up events, the host can implement additional behaviors without firmware support.

Examples:

```text
Rotate
Hold Tune + Rotate
Tap Tune
Double Tap Tune
Long Press Tune
```

Possible application mappings:

| App | Rotate | Hold + Rotate | Press |
|---|---|---|---|
| Default | Volume | Brightness | Mute |
| Chrome | Tabs | Zoom | New Tab |
| Photoshop | Brush Size | Brush Hardness | Brush Tool |
| Resolve | Frame Scrub | Timeline Zoom | Play/Pause |

These distinctions should be detected host-side whenever the HID transport provides sufficient down/up timing information.

---

## 13. UI Scope

The UI should prioritize quick configuration rather than exposing every low-level detail.

### Main Screen

Suggested structure:

```text
Naya Companion

Active Profile: Google Chrome

[ Default ]
[ Chrome ]
[ Photoshop ]
[ Resolve ]
[ + Add Application ]

Chrome
------------------------------------------------
Tune Clockwise          Next Tab
Tune Counterclockwise   Previous Tab
Tune Press              New Tab

Left Touch Swipe Left   Browser Back
Left Touch Swipe Right  Browser Forward

Right Touch Swipe Left  Previous Tab
Right Touch Swipe Right Next Tab
------------------------------------------------
```

### Mapping Editor

Clicking a mapping opens an action picker.

Categories:

```text
Recommended
Keyboard Shortcut
Mouse / Scroll
Media
System
Navigation
Application Presets
Launch / Script
Advanced
```

Search should be the primary navigation mechanism.

Example:

```text
Search actions: [ tab ]

Next Tab
Previous Tab
New Tab
Close Tab
```

### Interactive Input Detection

Provide an optional:

> **Detect Input**

flow.

The user performs a physical action:

```text
Rotate Tune clockwise
```

The UI displays:

```text
Detected: Tune / Clockwise
```

Then the user chooses its action.

This is especially useful for Touch gesture discovery.

### Active Profile Indicator

Optional tray/menu-bar tooltip:

```text
Naya Companion
Profile: Photoshop
```

No persistent full-size UI is required.

---

## 14. Configuration Storage

Use a human-readable local configuration format.

JSON or TOML are preferred.

Conceptual schema:

```toml
[transport.tune]
clockwise = "F24"
counterclockwise = "F23"
press = "F22"
swipe_left = "F20"
swipe_right = "F19"

[profiles.default]
TUNE_CW = "system.volume_up"
TUNE_CCW = "system.volume_down"
TUNE_PRESS = "system.mute"

[profiles.chrome]
match.windows_exe = "chrome.exe"
match.macos_bundle = "com.google.Chrome"

TUNE_CW = "browser.next_tab"
TUNE_CCW = "browser.previous_tab"
TUNE_PRESS = "browser.new_tab"
LEFT_TOUCH_SWIPE_LEFT = "browser.back"
LEFT_TOUCH_SWIPE_RIGHT = "browser.forward"
```

The exact schema may evolve, but:

- Transport mapping.
- Semantic events.
- Application matching.
- User actions.

must remain separate concepts.

---

## 15. System Tray / Menu Bar

The background app should expose a minimal tray/menu-bar menu:

```text
Naya Companion
-------------------------
Active: Photoshop
Open Configuration
Pause Companion
Reload Configuration
Quit
```

Optional diagnostics:

```text
Last Input: Tune Clockwise
Last Transport: F24
Profile: Photoshop
Action: Increase Brush Size
```

Diagnostics should be easy to enable during development but should not create continuous logs by default.

---

## 16. Startup Behavior

Optional user setting:

```text
[✓] Start Naya Companion at login
```

Requirements:

- No admin elevation on normal launch.
- No visible window at startup.
- Background engine starts directly.
- Configuration UI stays closed.

---

## 17. Windows Implementation Notes

Likely requirements:

- Global low-level keyboard input interception.
- Suppression of reserved transport keys.
- Foreground-window/process detection.
- Synthetic keyboard and mouse event generation.
- System tray integration.
- Installer registration for optional startup.

Potential APIs/libraries should be evaluated during implementation rather than hard-coded into the architecture.

Important requirement:

**Do not globally swallow F17–F24 unless they are configured as Naya transport events.**

---

## 18. macOS Implementation Notes

Likely requirements:

- Event tap or equivalent global input interception.
- Accessibility/Input Monitoring permission.
- Frontmost application detection.
- Synthetic keyboard/mouse events.
- Menu-bar integration.
- Optional login item.

The UI should explain required macOS permissions clearly and only request them when necessary.

---

## 19. Packaging and GitHub Releases

The project should build installable artifacts automatically using GitHub Actions.

Suggested release assets:

### Windows

```text
NayaCompanion-Setup-x64.exe
```

Optional later:

```text
NayaCompanion-Setup-arm64.exe
NayaCompanion-portable-x64.zip
```

### macOS

```text
NayaCompanion-universal.dmg
```

or:

```text
NayaCompanion-universal.pkg
```

Initial macOS builds may be unsigned during development, but production-quality distribution should eventually use Apple signing/notarization.

### GitHub Release Workflow

On version tag:

```text
v0.1.0
```

CI should:

1. Build Windows release.
2. Build macOS release.
3. Run automated tests.
4. Package installers.
5. Generate checksums.
6. Attach installers to GitHub Release.
7. Generate release notes/changelog.

The companion should be fully usable offline after installation.

---

## 20. Privacy and Security

Naya Companion should be local-only by default.

Requirements:

- No account.
- No telemetry.
- No cloud service.
- No keyboard-content logging.
- Do not record normal keystrokes.
- Only intercept transport events configured for Naya Companion.
- Do not store foreground-window titles unless explicitly needed.
- Application identification should normally use process executable or bundle ID only.
- Scripts/commands should require explicit user configuration.

---

## 21. Development Phases

### Phase 0 — Transport Proof of Concept

Goal:

Prove the architecture before building a polished app.

Deliverables:

- Detect F23/F24 globally.
- Suppress them from applications.
- Detect foreground application.
- Map:
  - Desktop -> volume.
  - Chrome -> next/previous tab.
- Demonstrate profile switch without keyboard reflash.

Success criterion:

**One fixed Tune firmware mapping behaves differently in at least two applications.**

---

### Phase 1 — Minimal Companion Engine

Deliverables:

- Rust background process.
- Windows support first if desired.
- F17–F24 transport.
- Modifier namespaces.
- Semantic event decoder.
- Default profile.
- Per-app profile matching.
- Keyboard shortcut execution.
- Mouse scroll execution.
- Media/system actions.
- Config hot reload.
- Tray icon.
- Start-at-login option.

---

### Phase 2 — Configuration UI

Deliverables:

- Application list.
- Add/remove application profiles.
- Mapping editor.
- Searchable action picker.
- Custom shortcut recorder.
- Tune/Touch semantic event names.
- Detect-input workflow.
- Save and instant reload.

---

### Phase 3 — Tune Enhancements

Deliverables:

- Rotary velocity measurement.
- Configurable acceleration.
- Press/hold state.
- Press + rotate mappings.
- Tap/double-tap/long-press where transport permits.

---

### Phase 4 — Multiple Touch Modules

Deliverables:

- Left/right Touch namespaces.
- Gesture mapping.
- Two-Touch profile templates.
- Conflict detection for duplicate transport assignments.

---

### Phase 5 — macOS

Deliverables:

- macOS input capture.
- Bundle-ID profiles.
- Event output.
- Menu-bar application.
- Permission onboarding.
- Universal installer.

---

### Phase 6 — Advanced Integrations

Potential future plugins:

- OBS.
- MIDI.
- Resolve API.
- Adobe automation.
- Windows per-app audio.
- macOS Shortcuts.
- Home Assistant.
- User scripts.
- Vendor-defined HID if open Naya firmware becomes available.

---

## 22. Initial Acceptance Criteria

Version 1.0 should satisfy all of the following:

- User can install from a GitHub Release using a normal installer.
- Background engine starts without a full UI.
- Idle CPU usage is effectively zero.
- Transport events can be intercepted without leaking into applications.
- Tune clockwise/counterclockwise/press can be mapped independently.
- Touch gestures can be mapped if the Naya firmware exposes them as discrete host events.
- Left and right Touch can be distinguished.
- At least one default/global profile exists.
- Multiple per-application profiles can be created.
- Active application automatically selects the correct profile.
- User can choose from common actions.
- User can enter custom keyboard shortcuts.
- User can configure rotary acceleration.
- Configuration changes apply without reflashing or reconnecting the keyboard.
- No cloud dependency is required.
- Windows and macOS releases are available.

---

## 23. Core Design Principle

The long-term design should preserve a strict separation:

```text
PHYSICAL EVENT
Tune rotated clockwise
        |
        v
SEMANTIC EVENT
TUNE_CW
        |
        v
APPLICATION CONTEXT
Photoshop
        |
        v
ACTION
Increase Brush Size
```

Neither the physical transport nor the application action should be permanently coupled.

This allows the transport to evolve from:

```text
F24
```

to:

```text
Vendor HID: TUNE_CW
```

in a future open firmware without breaking user profiles.

---

## 24. Recommended MVP

The first useful build should stay intentionally small:

**Inputs**
- Tune CW.
- Tune CCW.
- Tune press.
- Four Touch swipe directions.
- Separate Left/Right Touch namespaces.

**Actions**
- Custom keyboard shortcut.
- Volume.
- Mute.
- Media.
- Vertical/horizontal scroll.
- Launch application.
- Run command.

**Profiles**
- Default.
- Chrome/Edge/Firefox/Safari.
- Photoshop.
- Premiere.
- Resolve.
- Fusion 360.
- VS Code.
- Discord.
- Spotify.

**UI**
- Add application.
- Choose event.
- Choose action.
- Record shortcut.
- Optional acceleration.
- Save.

That is enough to validate the entire architecture without recreating NayaFlow or building a large application framework.
