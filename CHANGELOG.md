# Release notes

## 0.2.0 — macOS arrives; Windows polish

### macOS (first release)

Create Companion now runs on macOS 10.15 and later, Intel and Apple Silicon, from one universal
dmg. The engine is a menu-bar program built on a CGEvent tap, with the same configuration
window as Windows.

- Inputs use F13–F20 (macOS has no F21–F24); the first-run configuration and the bundled
  profiles use Mac shortcuts (Cmd+T, Cmd+Z, Ctrl+Up for Mission Control, and so on).
- Needs two permissions in *System Settings → Privacy & Security*: **Input Monitoring** to see
  the module's keys and **Accessibility** to send shortcuts and read window titles. The engine
  asks on first start; quit and reopen it after granting them.
- **Not signed or notarized yet.** Gatekeeper blocks the first launch: right-click → Open on
  Ventura and Sonoma, or *Privacy & Security → Open Anyway* on macOS 15. See INSTALL.md.
- Start at login is a per-user LaunchAgent. Configuration lives in
  `~/Library/Application Support/CreateCompanion/`, logs in `~/Library/Logs/CreateCompanion/`.
- First release on this platform: tested on an Intel MacBook running Ventura. Reports from
  other machines are welcome in Issues.

### Windows and both platforms

- Every search field has a clear button (and Escape clears it).
- Starring an app uses the shortcuts for the platform you are on; several first-wave catalog
  entries had Mac columns that copied the Windows chords (undo/redo, new tab, back/forward,
  zoom) and are corrected.
- Issue templates for bugs, catalog requests and feature requests; Discussions enabled.
- Repository housekeeping: plan and scope documents moved to `docs/`.

### Upgrading

Windows: run the new installer over the old one; it stops the engine, replaces the programs and
restarts. macOS: drag the new app over the old one in Applications. Configuration files are kept
on both.

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
