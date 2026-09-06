# Release notes

## Unreleased

- **macOS** (in testing): menu-bar engine on a CGEvent tap, the same configuration window,
  universal Intel/Apple Silicon dmg. Needs Input Monitoring and Accessibility; unsigned for now.
  Inputs use F13–F20 (macOS has no F21–F24).

## 0.1.0 — first release (Windows)

Create Companion turns the Naya Create's Tune (and, soon, Touch) gestures into per-application
shortcuts. Flash the module once; change what its gestures do on the computer, live.

### What you get

- **Engine**: a small tray program that intercepts the F-keys a module sends, decodes the gesture
  (dial, taps and swipes by finger count, with modifier namespaces so two modules can share keys),
  picks a profile by the application in front, and sends the mapped action. Hot-reloads its
  configuration; starts at login if you ask it to.
- **Configuration window**: Active and Available profiles with search; star to enable; preview
  an app's shortcuts before enabling it; drag to set priority; per-gesture actions chosen by name
  from the catalog or recorded from the keyboard; media, scroll, launch and command actions;
  live "detected" strip showing what each gesture did.
- **Catalog**: 162 applications, websites and systems with 22,646 documented shortcuts,
  compiled from vendor documentation with sources cited per entry. Filtered to the running OS,
  with an *All platforms* switch for building a configuration to move to a Mac.
- **Profiles**: System (fallback), God Mode (wins everywhere; for window switching and other
  system-wide gestures), and one per app or site. A site title beats an app; one app beats a
  group; drag order breaks ties.
- **Inputs**: which key each gesture sends, with Learn (capture the module's actual key) and
  Export for OpenFlow (writes module-profile files to flash from).
- **Dial**: speed curves (light / medium / aggressive, tunable in the config) and a per-binding
  multiplier.
- **Two-finger swipes** on the Tune arrive as a run of keys scaled to travel; collapsed to one
  action by default, or set to *Follow swipe* per input or per binding so volume and scroll track
  the swipe.
- **Held modifiers**: a Shift or Alt you hold yourself never changes what a gesture means.
- **Installer**: per-user NSIS installer with both programs, Start Menu entry, starts the engine
  after install, removes the login entry on uninstall, keeps your configuration.

### Known limitations

- Windows only. macOS is next; Linux later.
- The installer is not code-signed yet: SmartScreen shows a warning on first run. Verify the
  `.sha256` from the Releases page.
- Touch modules are not yet measured on real hardware. The engine already handles their key
  ranges and modifier namespaces; how the Touch streams gestures will be characterised when one
  is on the bench, and defaults will follow.
- Hold, long-press and double-tap are not offered: the module firmware reports a tap only on
  finger lift, so there is nothing for the host to build them from.
- Third-party Alt+Tab replacements (DisplayFusion) capture the sticky Ctrl+Alt+Tab switcher as
  well and ignore arrow keys in that mode; disable their handler to drive Windows' switcher from
  the dial.

### Configuration

`%APPDATA%\CreateCompanion\config.toml`, schema version 2. Older files are migrated on load with a
backup kept next to them. Logs in `%LOCALAPPDATA%\CreateCompanion\logs\`.
