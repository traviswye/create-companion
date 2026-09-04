# Catalog: one file per application, website, or system

`catalog/<id>.json` is the source of truth for what Create Companion knows about an
application: how to recognise it, every keyboard shortcut it documents, and the default
mappings for the Tune. `tools/gen_presets.py` merges these files (plus the ShortcutMapper
import in `reference/`) into `presets/apps.json`, which the UI embeds.

Validate before committing:

```
python tools/validate_catalog.py                 # all files
python tools/validate_catalog.py catalog/x.json  # one file
```

## File shape

```json
{
  "id": "spotify",
  "name": "Spotify",
  "kind": "app",
  "category": "Music",
  "match": {
    "windows_exe": ["Spotify.exe"],
    "macos_bundle": ["com.spotify.client"],
    "window_title": []
  },
  "sources": [
    "https://support.spotify.com/us/article/keyboard-shortcuts/"
  ],
  "actions": [
    { "name": "Play / pause", "context": "", "windows": { "type": "keys", "chord": "Space" }, "mac": { "type": "keys", "chord": "Space" } },
    { "name": "Open settings", "context": "", "windows": { "type": "sequence", "chords": ["Ctrl+K", "Ctrl+S"] } }
  ],
  "defaults": {
    "TUNE_CW":  { "name": "Volume up (app)", "action": { "type": "keys", "chord": "Ctrl+Up" }, "accel": "light" },
    "TUNE_CCW": { "name": "Volume down (app)", "action": { "type": "keys", "chord": "Ctrl+Down" }, "accel": "light" },
    "TUNE_TAP_1F": { "name": "Play / pause", "action": { "type": "keys", "chord": "Space" } }
  }
}
```

| Field | Rules |
|---|---|
| `id` | Lower-case, `[a-z0-9_]`, equals the file name. |
| `name` | Display name as the vendor writes it ("Adobe Photoshop", "Google Docs"). |
| `kind` | `app` (desktop program), `site` (website, matched by window title inside a browser), `system` (an OS: shortcuts of the desktop itself). |
| `category` | One of: Browser, Video, Music, Creative, CAD / 3D, Development, Communication, Productivity, Office, Games, System, Social, Shopping, AI, Files, Other. |
| `os` | System entries only: `["windows"]`, `["macos"]` or `["linux"]`. |
| `match.windows_exe` | Executable file names, exact spelling and case as on disk (`Code.exe`, `Adobe Premiere Pro.exe`). Required for `app`. |
| `match.macos_bundle` | Bundle identifiers (`com.microsoft.VSCode`). Include when known. |
| `match.window_title` | Substrings of the window title, case-insensitive. Required for `site` (`["YouTube"]`, `["Google Docs"]`). For a site, also copy the browser executables into `windows_exe` and bundles into `macos_bundle` so the profile only fires inside a browser: `["chrome.exe","msedge.exe","firefox.exe","brave.exe","vivaldi.exe"]` and `["com.google.Chrome","com.microsoft.edgemac","org.mozilla.firefox","com.brave.Browser","com.apple.Safari"]`. |
| `title_required` | Optional, apps only. `true` means the `window_title` narrows the desktop match (tmux inside a terminal window) instead of describing a web version. Without it, an app that has both an exe rule and a title rule is emitted twice: the desktop app (exe/bundle only) and a `<id>_web` site twin matched by title inside a browser, sharing the same actions. |
| `sources` | Every URL the shortcuts were taken from. Official documentation first. Required. |
| `actions` | The complete documented shortcut list. See below. |
| `defaults` | Optional. Suggested Tune bindings, keyed by event name (`TUNE_CW`, `TUNE_CCW`, `TUNE_TAP_1F`, `TUNE_SWIPE_LEFT/RIGHT/UP/DOWN`). Each is `{ name, action, accel? }`. Only include obvious ones (dial = the app's primary next/previous or zoom, tap = its primary toggle). |

## Actions

Each action: `{ "name", "context", "windows"?, "mac"?, "linux"? }`.

- `name`: the vendor's own label for the command ("Increase Brush Size", "Go to next tab"). No trailing period.
- `context`: `""` when the shortcut works everywhere in the app; otherwise the mode or panel it applies to ("Timeline", "Develop module", "Message list"). Two actions may share a name only if their contexts differ.
- `windows`, `mac`, `linux`: what the shortcut is on that platform. Omit a platform the vendor does not document. If Linux equals Windows, omit `linux`.

An action value is one of:

```json
{ "type": "keys",     "chord": "Ctrl+Shift+T" }
{ "type": "sequence", "chords": ["Ctrl+K", "Ctrl+S"] }   // multi-step chords, pressed one after another
{ "type": "media",    "key": "play_pause" }              // volume_up | volume_down | mute | play_pause | next_track | previous_track
{ "type": "scroll",   "direction": "up", "lines": 1 }    // up | down | left | right
```

### Chord grammar (exact)

`Modifier+Modifier+Key`, joined with `+`, one key per chord.

- Modifiers: `Ctrl`, `Shift`, `Alt`, `Win`. On macOS write `Cmd` for ⌘ and `Alt` for ⌥ (`Cmd` is accepted as a spelling of the meta key; use `Ctrl` for ⌃).
- Keys: letters `A`–`Z`, digits `0`–`9`, `F1`–`F24`, `Tab`, `Enter`, `Esc`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Left`, `Right`, `Up`, `Down`, and punctuation written literally: `-` `=` `[` `]` `\` `;` `'` `,` `.` `/` `` ` ``. For `+` as a key write `Ctrl++` or `Ctrl+Plus`.
- Not expressible, so leave out: mouse clicks and drags, numpad-specific keys, media keys as chords (use `type: media`), holding a key while dragging, shortcuts that require a two-key press without a modifier ("G then T" style is a `sequence` of `G` and `T`).

## What "complete" means

If the vendor documents 300 shortcuts, list 300. Do not curate down to the interesting ones; the picker has search. Keep the vendor's wording and grouping (use their section names as `context` when they are modes, not when they are just headings). Prefer the current version of the product; note the version in `note` if the docs are version-specific.
