#!/usr/bin/env python3
"""Generate the bundled presets from the NayaOS reference data.

Outputs (checked in, regenerate when the reference data changes):
  presets/actions.json         action catalog for the Phase 2 picker
  presets/default-config.toml  first-run config embedded in the engine

Sources (reference/, a snapshot of NayaOS docs/reference; see reference/ATTRIBUTION.md):
  action-chords.json   NayaFlow's action vocabulary with per-platform chords
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
# Snapshot of the NayaOS reference data lives in ./reference; when this folder
# sits inside the NayaOS monorepo, prefer the live copy there.
_MONOREPO_REF = ROOT.parent / "docs" / "reference"
REF = _MONOREPO_REF if (_MONOREPO_REF / "action-chords.json").exists() else ROOT / "reference"
OUT = ROOT / "presets"

# Naya keycode token -> our chord token (see companion-core/src/keys.rs)
TOKENS = {
    "LCTRL": "Ctrl", "RCTRL": "Ctrl", "LSHIFT": "Shift", "RSHIFT": "Shift",
    "LALT": "Alt", "RALT": "Alt", "LGUI": "Win", "RGUI": "Win",
    "EQUAL": "=", "MINUS": "-", "LEFT_BRACKET": "[", "RIGHT_BRACKET": "]",
    "SEMICOLON": ";", "SINGLE_QUOTE": "'", "COMMA": ",", "PERIOD": ".",
    "SLASH": "/", "BACKSLASH": "\\", "GRAVE": "`",
    "PG_UP": "PageUp", "PG_DN": "PageDown", "PAGE_UP": "PageUp", "PAGE_DOWN": "PageDown",
    "ESC": "Esc", "ESCAPE": "Esc", "ENTER": "Enter", "RETURN": "Enter", "TAB": "Tab",
    "SPACE": "Space", "BACKSPACE": "Backspace", "DELETE": "Delete", "DEL": "Delete",
    "INSERT": "Insert", "HOME": "Home", "END": "End",
    "LEFT": "Left", "RIGHT": "Right", "UP": "Up", "DOWN": "Down",
}

CATEGORY = {
    "browser": "Browser", "os": "System", "text": "Text editing", "vscode": "VS Code",
    "file-manager": "Files", "terminal": "Terminal",
}


def translate(chord: str) -> str | None:
    out = []
    for tok in (t.strip() for t in chord.split("+")):
        if tok in TOKENS:
            out.append(TOKENS[tok])
        elif re.fullmatch(r"[A-Z]", tok):
            out.append(tok)
        elif m := re.fullmatch(r"NUMBER_(\d)", tok):
            out.append(m.group(1))
        elif re.fullmatch(r"F([1-9]|1\d|2[0-4])", tok):
            out.append(tok)
        else:
            return None  # KP_*, CLICK, consumer keys: not a plain chord
    mods = [t for t in out if t in ("Ctrl", "Shift", "Alt", "Win")]
    keys_ = [t for t in out if t not in ("Ctrl", "Shift", "Alt", "Win")]
    if len(keys_) != 1:
        return None
    order = {"Ctrl": 0, "Shift": 1, "Alt": 2, "Win": 3}
    return "+".join(sorted(set(mods), key=order.get) + keys_)


def build_catalog() -> list[dict]:
    data = json.loads((REF / "action-chords.json").read_text(encoding="utf-8"))["actions"]
    catalog: list[dict] = []
    seen: set[str] = set()
    for a in data:
        ctx = a.get("context")
        if ctx not in CATEGORY or "win" not in a:
            continue
        win = translate(a["win"]["chord"])
        if not win:
            continue
        mac = translate(a["mac"]["chord"]) if "mac" in a else None
        ident = f"{ctx.replace('-', '_')}.{a['action'].lower()}"
        if ident in seen:
            continue
        seen.add(ident)
        entry = {
            "id": ident,
            "name": a["name"],
            "category": CATEGORY[ctx],
            "windows": {"type": "keys", "chord": win},
        }
        if mac:
            entry["mac"] = {"type": "keys", "chord": mac}
        catalog.append(entry)

    for key, name in [
        ("volume_up", "Volume up"), ("volume_down", "Volume down"), ("mute", "Mute"),
        ("play_pause", "Play / pause"), ("next_track", "Next track"),
        ("previous_track", "Previous track"),
    ]:
        act = {"type": "media", "key": key}
        catalog.append({"id": f"media.{key}", "name": name, "category": "Media",
                        "windows": act, "mac": act})
    for d, name in [("up", "Scroll up"), ("down", "Scroll down"),
                    ("left", "Scroll left"), ("right", "Scroll right")]:
        act = {"type": "scroll", "direction": d, "lines": 1}
        catalog.append({"id": f"scroll.{d}", "name": name, "category": "Mouse / Scroll",
                        "windows": act, "mac": act})
    catalog.sort(key=lambda e: (e["category"], e["name"]))
    return catalog


# ---- default profiles -------------------------------------------------------
# Each binding: (event, action, accel).

def keys(chord: str) -> dict:
    return {"type": "keys", "chord": chord}


def media(key: str) -> dict:
    return {"type": "media", "key": key}


def scroll(direction: str) -> dict:
    return {"type": "scroll", "direction": direction, "lines": 1}


DEFAULT_PROFILE = {
    "name": "Default",
    "bindings": [
        ("TUNE_CW", media("volume_up"), "light"),
        ("TUNE_CCW", media("volume_down"), "light"),
        ("TUNE_PRESS", media("mute"), None),
        ("TUNE_SWIPE_LEFT", media("previous_track"), None),
        ("TUNE_SWIPE_RIGHT", media("next_track"), None),
        ("TUNE_SWIPE_UP", media("play_pause"), None),
        ("TUNE_SWIPE_DOWN", keys("Win+Tab"), None),          # Task view
    ],
}

BROWSER_EXE = ["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "vivaldi.exe"]
BROWSER_BUNDLE = ["com.google.Chrome", "com.microsoft.edgemac", "org.mozilla.firefox",
                  "com.brave.Browser", "com.apple.Safari"]

# Order matters only for ties; a profile with a window_title rule always beats
# an exe-only profile for the same app, so YouTube can sit after Browser.
PROFILES = [
    {
        "name": "Browser",
        "windows_exe": BROWSER_EXE,
        "macos_bundle": BROWSER_BUNDLE,
        "bindings": [
            ("TUNE_CW", keys("Ctrl+Tab"), None),
            ("TUNE_CCW", keys("Ctrl+Shift+Tab"), None),
            ("TUNE_PRESS", keys("Ctrl+T"), None),
            ("TUNE_SWIPE_LEFT", keys("Alt+Left"), None),      # Back
            ("TUNE_SWIPE_RIGHT", keys("Alt+Right"), None),    # Forward
            ("TUNE_SWIPE_UP", keys("Ctrl+="), None),          # Zoom in
            ("TUNE_SWIPE_DOWN", keys("Ctrl+-"), None),        # Zoom out
        ],
    },
    {
        "name": "YouTube",
        "windows_exe": BROWSER_EXE,
        "macos_bundle": BROWSER_BUNDLE,
        "window_title": ["YouTube"],
        "bindings": [
            ("TUNE_CW", keys("Right"), "medium"),             # Seek +5 s (fast turn: more)
            ("TUNE_CCW", keys("Left"), "medium"),             # Seek -5 s
            ("TUNE_PRESS", keys("K"), None),                  # Play / pause
            ("TUNE_SWIPE_LEFT", keys("Shift+P"), None),       # Previous video
            ("TUNE_SWIPE_RIGHT", keys("Shift+N"), None),      # Next video
            ("TUNE_SWIPE_UP", keys("Shift+."), None),         # Playback speed up
            ("TUNE_SWIPE_DOWN", keys("Shift+,"), None),       # Playback speed down
        ],
    },
    {
        "name": "Terminal",
        # Windows Terminal, PowerShell 5/7, cmd, and legacy console windows.
        "windows_exe": ["WindowsTerminal.exe", "powershell.exe", "pwsh.exe", "cmd.exe",
                        "conhost.exe", "OpenConsole.exe"],
        "macos_bundle": ["com.apple.Terminal", "com.googlecode.iterm2"],
        "bindings": [
            ("TUNE_CW", keys("Down"), None),                  # Newer command in history
            ("TUNE_CCW", keys("Up"), None),                   # Older command in history
            ("TUNE_PRESS", keys("Esc"), None),                # Clear the current line
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Shift+Tab"), None),  # Previous tab (Windows Terminal)
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Tab"), None),       # Next tab
            ("TUNE_SWIPE_UP", keys("Ctrl+="), None),          # Font bigger
            ("TUNE_SWIPE_DOWN", keys("Ctrl+-"), None),        # Font smaller
        ],
    },
    {
        "name": "Photoshop",
        "windows_exe": ["Photoshop.exe"],
        "macos_bundle": ["com.adobe.Photoshop"],
        "bindings": [
            ("TUNE_CW", keys("]"), "medium"),                 # Increase Brush Size
            ("TUNE_CCW", keys("["), "medium"),                # Decrease Brush Size
            ("TUNE_PRESS", keys("B"), None),                  # Brush Tool
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Z"), None),        # Undo
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Shift+Z"), None), # Redo
            ("TUNE_SWIPE_UP", keys("Shift+]"), None),         # Increase Brush Hardness
            ("TUNE_SWIPE_DOWN", keys("Shift+["), None),       # Decrease Brush Hardness
        ],
    },
    {
        "name": "Lightroom",
        "windows_exe": ["Lightroom.exe", "LightroomClassic.exe"],
        "macos_bundle": ["com.adobe.LightroomClassicCC7", "com.adobe.lightroomCC"],
        "bindings": [
            ("TUNE_CW", keys("Right"), None),                 # Next Photo in Filmstrip
            ("TUNE_CCW", keys("Left"), None),                 # Previous Photo in Filmstrip
            ("TUNE_PRESS", keys("Z"), None),                  # Toggle Zoom View
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Z"), None),        # Undo
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Y"), None),       # Redo
            ("TUNE_SWIPE_UP", keys("P"), None),               # Flag as pick
            ("TUNE_SWIPE_DOWN", keys("X"), None),             # Flag as reject
        ],
    },
    {
        "name": "Premiere Pro",
        "windows_exe": ["Adobe Premiere Pro.exe"],
        "macos_bundle": ["com.adobe.PremierePro.CC"],
        "bindings": [
            ("TUNE_CW", keys("Right"), "medium"),             # Step forward one frame
            ("TUNE_CCW", keys("Left"), "medium"),             # Step back one frame
            ("TUNE_PRESS", keys("Space"), None),              # Play / stop
            ("TUNE_SWIPE_LEFT", keys("Shift+Left"), None),    # Step back five frames
            ("TUNE_SWIPE_RIGHT", keys("Shift+Right"), None),  # Step forward five frames
            ("TUNE_SWIPE_UP", keys("="), None),               # Zoom in timeline
            ("TUNE_SWIPE_DOWN", keys("-"), None),             # Zoom out timeline
        ],
    },
    {
        "name": "DaVinci Resolve",
        "windows_exe": ["Resolve.exe"],
        "macos_bundle": ["com.blackmagic-design.DaVinciResolve"],
        "bindings": [
            ("TUNE_CW", keys("Right"), "medium"),
            ("TUNE_CCW", keys("Left"), "medium"),
            ("TUNE_PRESS", keys("Space"), None),
            ("TUNE_SWIPE_LEFT", keys("Shift+Left"), None),    # Back one second
            ("TUNE_SWIPE_RIGHT", keys("Shift+Right"), None),  # Forward one second
            ("TUNE_SWIPE_UP", keys("Ctrl+="), None),          # Zoom in timeline
            ("TUNE_SWIPE_DOWN", keys("Ctrl+-"), None),        # Zoom out timeline
        ],
    },
    {
        "name": "Fusion 360",
        "windows_exe": ["Fusion360.exe"],
        "macos_bundle": ["com.autodesk.fusion360"],
        "bindings": [
            ("TUNE_CW", scroll("up"), "light"),               # Zoom in
            ("TUNE_CCW", scroll("down"), "light"),            # Zoom out
            ("TUNE_PRESS", keys("F6"), None),                 # Fit view
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Z"), None),        # Undo
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Y"), None),       # Redo
        ],
    },
    {
        "name": "Blender",
        "windows_exe": ["blender.exe"],
        "macos_bundle": ["org.blenderfoundation.blender"],
        "bindings": [
            ("TUNE_CW", keys("Right"), "medium"),             # Next frame
            ("TUNE_CCW", keys("Left"), "medium"),             # Previous frame
            ("TUNE_PRESS", keys("Space"), None),              # Play animation
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Z"), None),        # Undo
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Shift+Z"), None), # Redo
            ("TUNE_SWIPE_UP", keys("Up"), None),              # Jump to next keyframe
            ("TUNE_SWIPE_DOWN", keys("Down"), None),          # Jump to previous keyframe
        ],
    },
    {
        "name": "VS Code",
        "windows_exe": ["Code.exe", "Code - Insiders.exe"],
        "macos_bundle": ["com.microsoft.VSCode"],
        "bindings": [
            ("TUNE_CW", keys("Ctrl+PageDown"), None),         # Next editor tab
            ("TUNE_CCW", keys("Ctrl+PageUp"), None),          # Previous editor tab
            ("TUNE_PRESS", keys("Ctrl+Shift+P"), None),       # Command palette
            ("TUNE_SWIPE_LEFT", keys("Alt+Left"), None),      # Go back
            ("TUNE_SWIPE_RIGHT", keys("Alt+Right"), None),    # Go forward
            ("TUNE_SWIPE_UP", keys("Shift+F8"), None),        # Previous problem
            ("TUNE_SWIPE_DOWN", keys("F8"), None),            # Next problem
        ],
    },
    {
        "name": "Discord",
        "windows_exe": ["Discord.exe"],
        "macos_bundle": ["com.hnc.Discord"],
        "bindings": [
            ("TUNE_CW", keys("Alt+Down"), None),              # Next channel
            ("TUNE_CCW", keys("Alt+Up"), None),               # Previous channel
            ("TUNE_PRESS", keys("Ctrl+Shift+M"), None),       # Toggle mute
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Alt+Up"), None),   # Previous server
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Alt+Down"), None),  # Next server
            ("TUNE_SWIPE_UP", keys("Ctrl+Shift+D"), None),    # Toggle deafen
        ],
    },
    {
        "name": "Spotify",
        "windows_exe": ["Spotify.exe"],
        "macos_bundle": ["com.spotify.client"],
        "bindings": [
            ("TUNE_CW", keys("Ctrl+Up"), "light"),            # App volume up
            ("TUNE_CCW", keys("Ctrl+Down"), "light"),         # App volume down
            ("TUNE_PRESS", keys("Space"), None),              # Play / pause
            ("TUNE_SWIPE_LEFT", keys("Ctrl+Left"), None),     # Previous track
            ("TUNE_SWIPE_RIGHT", keys("Ctrl+Right"), None),   # Next track
            ("TUNE_SWIPE_UP", keys("Alt+Shift+B"), None),     # Save to Liked Songs
        ],
    },
]


def toml_str(s: str) -> str:
    return json.dumps(s)  # JSON string escaping is valid TOML basic-string escaping


def toml_inline(d: dict) -> str:
    parts = []
    for k, v in d.items():
        if isinstance(v, str):
            parts.append(f"{k} = {toml_str(v)}")
        elif isinstance(v, bool):
            parts.append(f"{k} = {'true' if v else 'false'}")
        elif isinstance(v, int):
            parts.append(f"{k} = {v}")
        elif isinstance(v, list):
            parts.append(f"{k} = [" + ", ".join(toml_str(x) for x in v) + "]")
        else:
            raise TypeError(k)
    return "{ " + ", ".join(parts) + " }"


def emit_bindings(prefix: str, bindings) -> list[str]:
    lines = []
    for event, action, accel in bindings:
        lines.append(f"[{prefix}.bindings.{event}]")
        lines.append(f"action = {toml_inline(action)}")
        if accel:
            lines.append(f"accel = {toml_str(accel)}")
        lines.append("")
    return lines


HEADER = """# Naya Companion configuration
# Generated by tools/gen_presets.py -- edit freely; the engine reloads on save.
#
# transport:  which F-key the Tune/Touch firmware emits for each gesture.
#             Only keys listed here are intercepted; all other keys pass through.
# profiles:   per-application bindings, matched on the foreground executable
#             (Windows) or bundle id (macOS), optionally narrowed by a
#             window_title substring (websites). Unbound events fall back to
#             default_profile.
# accel:      none | light | medium | aggressive -- repeat count grows with dial speed.

schema_version = 1

[engine]
start_at_login = false
log_level = "info"

[transport]
TUNE_CW = { key = "F24" }
TUNE_CCW = { key = "F23" }
TUNE_PRESS = { key = "F22" }
TUNE_SWIPE_LEFT = { key = "F20" }
TUNE_SWIPE_RIGHT = { key = "F19" }
TUNE_SWIPE_UP = { key = "F18" }
TUNE_SWIPE_DOWN = { key = "F17" }
# Touch modules use a modifier namespace on the same keys, e.g.:
# LEFT_TOUCH_SWIPE_LEFT = { key = "F20", mods = "shift" }
"""


def build_default_config() -> str:
    L = HEADER.splitlines() + [
        "",
        "[default_profile]",
        f"name = {toml_str(DEFAULT_PROFILE['name'])}",
        "",
    ]
    L += emit_bindings("default_profile", DEFAULT_PROFILE["bindings"])
    for p in PROFILES:
        L.append("[[profiles]]")
        L.append(f"name = {toml_str(p['name'])}")
        m = {"windows_exe": p["windows_exe"], "macos_bundle": p["macos_bundle"]}
        if p.get("window_title"):
            m["window_title"] = p["window_title"]
        L.append("match = " + toml_inline(m))
        L.append("")
        L += emit_bindings("profiles", p["bindings"])
    return "\n".join(L).rstrip() + "\n"


def main() -> int:
    OUT.mkdir(exist_ok=True)
    catalog = build_catalog()
    (OUT / "actions.json").write_text(
        json.dumps({"_meta": {"source": "tools/gen_presets.py from docs/reference/action-chords.json",
                              "count": len(catalog)},
                    "actions": catalog}, indent=1) + "\n", encoding="utf-8")
    cfg = build_default_config()
    tomllib.loads(cfg)  # syntax check
    (OUT / "default-config.toml").write_text(cfg, encoding="utf-8", newline="\n")
    print(f"actions.json: {len(catalog)} actions; default-config.toml: {len(PROFILES)} app profiles")
    return 0


if __name__ == "__main__":
    sys.exit(main())
