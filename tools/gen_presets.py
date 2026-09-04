#!/usr/bin/env python3
"""Generate the bundled presets from the reference data.

Outputs (checked in, regenerate when the reference data or the tables below change):
  presets/actions.json         generic action catalog (browser / system / text / ...)
  presets/apps.json            per-application catalog: match rules, the app's own
                               shortcuts (from ShortcutMapper), and default bindings
  presets/default-config.toml  first-run config embedded in the engine

Sources (reference/, a snapshot of NayaOS docs/reference; see reference/ATTRIBUTION.md):
  action-chords.json   NayaFlow's action vocabulary with per-platform chords
  app-shortcuts.json   ShortcutMapper import: 5,311 shortcuts across 20 apps (MIT)
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


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")


# ---- generic catalog --------------------------------------------------------

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


# ---- applications -----------------------------------------------------------
# Hand-authored apps. Bindings: (event, name, action, accel).

def keys(chord: str) -> dict:
    return {"type": "keys", "chord": chord}


def media(key: str) -> dict:
    return {"type": "media", "key": key}


def scroll(direction: str) -> dict:
    return {"type": "scroll", "direction": direction, "lines": 1}


DEFAULT_PROFILE = {
    "name": "Default",
    "bindings": [
        ("TUNE_CW", "Volume up", media("volume_up"), "light"),
        ("TUNE_CCW", "Volume down", media("volume_down"), "light"),
        ("TUNE_TAP_1F", "Mute", media("mute"), None),
        ("TUNE_SWIPE_LEFT", "Previous track", media("previous_track"), None),
        ("TUNE_SWIPE_RIGHT", "Next track", media("next_track"), None),
        ("TUNE_SWIPE_UP", "Play / pause", media("play_pause"), None),
        ("TUNE_SWIPE_DOWN", "Task view", keys("Win+Tab"), None),
    ],
}

BROWSER_EXE = ["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "vivaldi.exe"]
BROWSER_BUNDLE = ["com.google.Chrome", "com.microsoft.edgemac", "org.mozilla.firefox",
                  "com.brave.Browser", "com.apple.Safari"]

# id, name, kind, match, bindings. Order = order in the default config.
PROFILES = [
    {
        "id": "browser", "name": "Browser", "kind": "app",
        "windows_exe": BROWSER_EXE, "macos_bundle": BROWSER_BUNDLE,
        "bindings": [
            ("TUNE_CW", "Next tab", keys("Ctrl+Tab"), None),
            ("TUNE_CCW", "Previous tab", keys("Ctrl+Shift+Tab"), None),
            ("TUNE_TAP_1F", "New tab", keys("Ctrl+T"), None),
            ("TUNE_SWIPE_LEFT", "Back", keys("Alt+Left"), None),
            ("TUNE_SWIPE_RIGHT", "Forward", keys("Alt+Right"), None),
            ("TUNE_SWIPE_UP", "Zoom in", keys("Ctrl+="), None),
            ("TUNE_SWIPE_DOWN", "Zoom out", keys("Ctrl+-"), None),
        ],
    },
    {
        "id": "youtube", "name": "YouTube", "kind": "site",
        "windows_exe": BROWSER_EXE, "macos_bundle": BROWSER_BUNDLE, "window_title": ["YouTube"],
        "bindings": [
            ("TUNE_CW", "Seek forward 5 s", keys("Right"), "medium"),
            ("TUNE_CCW", "Seek back 5 s", keys("Left"), "medium"),
            ("TUNE_TAP_1F", "Play / pause", keys("K"), None),
            ("TUNE_SWIPE_LEFT", "Previous video", keys("Shift+P"), None),
            ("TUNE_SWIPE_RIGHT", "Next video", keys("Shift+N"), None),
            ("TUNE_SWIPE_UP", "Playback speed up", keys("Shift+."), None),
            ("TUNE_SWIPE_DOWN", "Playback speed down", keys("Shift+,"), None),
        ],
        "actions": [
            ("Seek forward 10 s", keys("L")), ("Seek back 10 s", keys("J")),
            ("Mute", keys("M")), ("Fullscreen", keys("F")), ("Captions", keys("C")),
            ("Theater mode", keys("T")), ("Miniplayer", keys("I")),
            ("Frame forward (paused)", keys(".")), ("Frame back (paused)", keys(",")),
        ],
    },
    {
        "id": "terminal", "name": "Terminal", "kind": "app",
        "windows_exe": ["WindowsTerminal.exe", "powershell.exe", "pwsh.exe", "cmd.exe",
                        "conhost.exe", "OpenConsole.exe"],
        "macos_bundle": ["com.apple.Terminal", "com.googlecode.iterm2"],
        "bindings": [
            ("TUNE_CW", "Newer command (history)", keys("Down"), None),
            ("TUNE_CCW", "Older command (history)", keys("Up"), None),
            ("TUNE_TAP_1F", "Clear line", keys("Esc"), None),
            ("TUNE_SWIPE_LEFT", "Previous tab", keys("Ctrl+Shift+Tab"), None),
            ("TUNE_SWIPE_RIGHT", "Next tab", keys("Ctrl+Tab"), None),
            ("TUNE_SWIPE_UP", "Font bigger", keys("Ctrl+="), None),
            ("TUNE_SWIPE_DOWN", "Font smaller", keys("Ctrl+-"), None),
        ],
        "actions": [
            ("New tab", keys("Ctrl+Shift+T")), ("Close tab", keys("Ctrl+Shift+W")),
            ("Find", keys("Ctrl+Shift+F")), ("Command palette", keys("Ctrl+Shift+P")),
            ("Split pane", keys("Alt+Shift+D")), ("Cancel command", keys("Ctrl+C")),
        ],
    },
    {
        "id": "photoshop", "name": "Adobe Photoshop", "kind": "app",
        "windows_exe": ["Photoshop.exe"], "macos_bundle": ["com.adobe.Photoshop"],
        "bindings": [
            ("TUNE_CW", "Increase Brush Size", keys("]"), "medium"),
            ("TUNE_CCW", "Decrease Brush Size", keys("["), "medium"),
            ("TUNE_TAP_1F", "Brush Tool", keys("B"), None),
            ("TUNE_SWIPE_LEFT", "Undo", keys("Ctrl+Z"), None),
            ("TUNE_SWIPE_RIGHT", "Redo", keys("Ctrl+Shift+Z"), None),
            ("TUNE_SWIPE_UP", "Increase Brush Hardness", keys("Shift+]"), None),
            ("TUNE_SWIPE_DOWN", "Decrease Brush Hardness", keys("Shift+["), None),
        ],
    },
    {
        "id": "lightroom", "name": "Adobe Lightroom", "kind": "app",
        "windows_exe": ["Lightroom.exe", "LightroomClassic.exe"],
        "macos_bundle": ["com.adobe.LightroomClassicCC7", "com.adobe.lightroomCC"],
        "bindings": [
            ("TUNE_CW", "Next Photo in Filmstrip", keys("Right"), None),
            ("TUNE_CCW", "Previous Photo in Filmstrip", keys("Left"), None),
            ("TUNE_TAP_1F", "Toggle Zoom View", keys("Z"), None),
            ("TUNE_SWIPE_LEFT", "Undo", keys("Ctrl+Z"), None),
            ("TUNE_SWIPE_RIGHT", "Redo", keys("Ctrl+Y"), None),
            ("TUNE_SWIPE_UP", "Flag as pick", keys("P"), None),
            ("TUNE_SWIPE_DOWN", "Flag as reject", keys("X"), None),
        ],
    },
    {
        "id": "premiere", "name": "Adobe Premiere Pro", "kind": "app",
        "windows_exe": ["Adobe Premiere Pro.exe"], "macos_bundle": ["com.adobe.PremierePro.CC"],
        "bindings": [
            ("TUNE_CW", "Step forward one frame", keys("Right"), "medium"),
            ("TUNE_CCW", "Step back one frame", keys("Left"), "medium"),
            ("TUNE_TAP_1F", "Play / stop", keys("Space"), None),
            ("TUNE_SWIPE_LEFT", "Step back five frames", keys("Shift+Left"), None),
            ("TUNE_SWIPE_RIGHT", "Step forward five frames", keys("Shift+Right"), None),
            ("TUNE_SWIPE_UP", "Zoom in timeline", keys("="), None),
            ("TUNE_SWIPE_DOWN", "Zoom out timeline", keys("-"), None),
        ],
        "actions": [
            ("Add marker", keys("M")), ("Ripple delete", keys("Shift+Delete")),
            ("Razor at playhead", keys("Ctrl+K")), ("Go to in point", keys("Shift+I")),
            ("Go to out point", keys("Shift+O")), ("Undo", keys("Ctrl+Z")), ("Redo", keys("Ctrl+Shift+Z")),
        ],
    },
    {
        "id": "resolve", "name": "DaVinci Resolve", "kind": "app",
        "windows_exe": ["Resolve.exe"], "macos_bundle": ["com.blackmagic-design.DaVinciResolve"],
        "bindings": [
            ("TUNE_CW", "Next frame", keys("Right"), "medium"),
            ("TUNE_CCW", "Previous frame", keys("Left"), "medium"),
            ("TUNE_TAP_1F", "Play / stop", keys("Space"), None),
            ("TUNE_SWIPE_LEFT", "Back one second", keys("Shift+Left"), None),
            ("TUNE_SWIPE_RIGHT", "Forward one second", keys("Shift+Right"), None),
            ("TUNE_SWIPE_UP", "Zoom in timeline", keys("Ctrl+="), None),
            ("TUNE_SWIPE_DOWN", "Zoom out timeline", keys("Ctrl+-"), None),
        ],
        "actions": [
            ("Add marker", keys("M")), ("Split clip", keys("Ctrl+\\")), ("Mark in", keys("I")),
            ("Mark out", keys("O")), ("Undo", keys("Ctrl+Z")), ("Redo", keys("Ctrl+Shift+Z")),
            ("Next edit", keys("Down")), ("Previous edit", keys("Up")),
        ],
    },
    {
        "id": "fusion360", "name": "Autodesk Fusion 360", "kind": "app",
        "windows_exe": ["Fusion360.exe"], "macos_bundle": ["com.autodesk.fusion360"],
        "bindings": [
            ("TUNE_CW", "Zoom in", scroll("up"), "light"),
            ("TUNE_CCW", "Zoom out", scroll("down"), "light"),
            ("TUNE_TAP_1F", "Fit view", keys("F6"), None),
            ("TUNE_SWIPE_LEFT", "Undo", keys("Ctrl+Z"), None),
            ("TUNE_SWIPE_RIGHT", "Redo", keys("Ctrl+Y"), None),
        ],
        "actions": [
            ("Sketch", keys("S")), ("Extrude", keys("E")), ("Measure", keys("I")),
            ("Hide / show", keys("V")), ("Display mode", keys("Ctrl+Alt+V")),
        ],
    },
    {
        "id": "blender", "name": "Blender", "kind": "app",
        "windows_exe": ["blender.exe"], "macos_bundle": ["org.blenderfoundation.blender"],
        "bindings": [
            ("TUNE_CW", "Next frame", keys("Right"), "medium"),
            ("TUNE_CCW", "Previous frame", keys("Left"), "medium"),
            ("TUNE_TAP_1F", "Play animation", keys("Space"), None),
            ("TUNE_SWIPE_LEFT", "Undo", keys("Ctrl+Z"), None),
            ("TUNE_SWIPE_RIGHT", "Redo", keys("Ctrl+Shift+Z"), None),
            ("TUNE_SWIPE_UP", "Jump to next keyframe", keys("Up"), None),
            ("TUNE_SWIPE_DOWN", "Jump to previous keyframe", keys("Down"), None),
        ],
    },
    {
        "id": "vscode", "name": "VS Code", "kind": "app",
        "windows_exe": ["Code.exe", "Code - Insiders.exe"], "macos_bundle": ["com.microsoft.VSCode"],
        "bindings": [
            ("TUNE_CW", "Next editor tab", keys("Ctrl+PageDown"), None),
            ("TUNE_CCW", "Previous editor tab", keys("Ctrl+PageUp"), None),
            ("TUNE_TAP_1F", "Command palette", keys("Ctrl+Shift+P"), None),
            ("TUNE_SWIPE_LEFT", "Go back", keys("Alt+Left"), None),
            ("TUNE_SWIPE_RIGHT", "Go forward", keys("Alt+Right"), None),
            ("TUNE_SWIPE_UP", "Previous problem", keys("Shift+F8"), None),
            ("TUNE_SWIPE_DOWN", "Next problem", keys("F8"), None),
        ],
        "actions_from_category": "VS Code",
    },
    {
        "id": "discord", "name": "Discord", "kind": "app",
        "windows_exe": ["Discord.exe"], "macos_bundle": ["com.hnc.Discord"],
        "bindings": [
            ("TUNE_CW", "Next channel", keys("Alt+Down"), None),
            ("TUNE_CCW", "Previous channel", keys("Alt+Up"), None),
            ("TUNE_TAP_1F", "Toggle mute", keys("Ctrl+Shift+M"), None),
            ("TUNE_SWIPE_LEFT", "Previous server", keys("Ctrl+Alt+Up"), None),
            ("TUNE_SWIPE_RIGHT", "Next server", keys("Ctrl+Alt+Down"), None),
            ("TUNE_SWIPE_UP", "Toggle deafen", keys("Ctrl+Shift+D"), None),
        ],
        "actions": [
            ("Mark server read", keys("Shift+Esc")), ("Unread channel", keys("Alt+Shift+Down")),
            ("Search", keys("Ctrl+K")), ("Answer call", keys("Ctrl+Enter")),
        ],
    },
    {
        "id": "spotify", "name": "Spotify", "kind": "app",
        "windows_exe": ["Spotify.exe"], "macos_bundle": ["com.spotify.client"],
        "bindings": [
            ("TUNE_CW", "Volume up (app)", keys("Ctrl+Up"), "light"),
            ("TUNE_CCW", "Volume down (app)", keys("Ctrl+Down"), "light"),
            ("TUNE_TAP_1F", "Play / pause", keys("Space"), None),
            ("TUNE_SWIPE_LEFT", "Previous track", keys("Ctrl+Left"), None),
            ("TUNE_SWIPE_RIGHT", "Next track", keys("Ctrl+Right"), None),
            ("TUNE_SWIPE_UP", "Save to Liked Songs", keys("Alt+Shift+B"), None),
        ],
        "actions": [
            ("Seek forward", keys("Shift+Right")), ("Seek back", keys("Shift+Left")),
            ("Shuffle", keys("Ctrl+S")), ("Repeat", keys("Ctrl+R")), ("Search", keys("Ctrl+L")),
        ],
    },
]

# ShortcutMapper app name -> (id, display name, match rules). Ids that also
# appear in PROFILES get the shortcuts attached to that profile's app entry.
SHORTCUTMAPPER_APPS = {
    "Adobe After Effects": ("after_effects", "Adobe After Effects", ["AfterFX.exe"], ["com.adobe.AfterEffects"]),
    "Adobe Illustrator": ("illustrator", "Adobe Illustrator", ["Illustrator.exe"], ["com.adobe.illustrator"]),
    "Adobe Lightroom": ("lightroom", None, None, None),
    "Adobe Photoshop": ("photoshop", None, None, None),
    "Autodesk 3dsMax": ("3dsmax", "Autodesk 3ds Max", ["3dsmax.exe"], []),
    "Autodesk Maya": ("maya", "Autodesk Maya", ["maya.exe"], ["com.autodesk.maya"]),
    "Blender": ("blender", None, None, None),
    "Euro Truck Simulator 2": ("ets2", "Euro Truck Simulator 2", ["eurotrucks2.exe"], ["com.scssoft.eurotrucks2"]),
    "JetBrains AppCode": ("appcode", "JetBrains AppCode", [], ["com.jetbrains.AppCode"]),
    "JetBrains CLion": ("clion", "JetBrains CLion", ["clion64.exe"], ["com.jetbrains.CLion"]),
    "JetBrains IntelliJ IDEA": ("intellij", "JetBrains IntelliJ IDEA", ["idea64.exe"], ["com.jetbrains.intellij"]),
    "JetBrains PhpStorm": ("phpstorm", "JetBrains PhpStorm", ["phpstorm64.exe"], ["com.jetbrains.PhpStorm"]),
    "JetBrains PyCharm": ("pycharm", "JetBrains PyCharm", ["pycharm64.exe"], ["com.jetbrains.pycharm"]),
    "JetBrains RubyMine": ("rubymine", "JetBrains RubyMine", ["rubymine64.exe"], ["com.jetbrains.rubymine"]),
    "JetBrains WebStorm": ("webstorm", "JetBrains WebStorm", ["webstorm64.exe"], ["com.jetbrains.WebStorm"]),
    "SideFx Houdini": ("houdini", "SideFX Houdini", ["houdini.exe", "houdinifx.exe"], ["com.sidefx.houdini"]),
    "SketchUp": ("sketchup", "SketchUp", ["SketchUp.exe"], ["com.sketchup.SketchUp.2024"]),
    "Sublime Text": ("sublime", "Sublime Text", ["sublime_text.exe"], ["com.sublimetext.4"]),
    "The Foundry Nuke": ("nuke", "The Foundry Nuke", [], ["com.thefoundry.Nuke"]),
    "Unity 3D": ("unity", "Unity", ["Unity.exe"], ["com.unity3d.UnityEditor5.x"]),
}


def context_rank(ctx: str) -> int:
    c = ctx.lower()
    if c in ("global context", "main ui", "houdini", "screen", "window"):
        return 0
    return 1


def build_apps(catalog: list[dict]) -> list[dict]:
    apps: dict[str, dict] = {}
    for p in PROFILES:
        entry = {
            "id": p["id"],
            "name": p["name"],
            "kind": p["kind"],
            "match": {
                "windows_exe": p.get("windows_exe", []),
                "macos_bundle": p.get("macos_bundle", []),
                "window_title": p.get("window_title", []),
            },
            "defaults": {
                ev: {"name": name, "action": action, **({"accel": accel} if accel else {})}
                for ev, name, action, accel in p["bindings"]
            },
            "actions": [],
        }
        seen: set[str] = set()
        # The default bindings are actions too, so the picker's app tab shows them.
        for _ev, name, action, _accel in p["bindings"]:
            aid = f"{p['id']}.{slug(name)}"
            if aid not in seen:
                seen.add(aid)
                entry["actions"].append({"id": aid, "name": name, "context": "", "windows": action, "mac": action})
        for name, action in p.get("actions", []):
            aid = f"{p['id']}.{slug(name)}"
            if aid not in seen:
                seen.add(aid)
                entry["actions"].append({"id": aid, "name": name, "context": "", "windows": action, "mac": action})
        if cat := p.get("actions_from_category"):
            for c in catalog:
                if c["category"] == cat:
                    aid = f"{p['id']}.{slug(c['name'])}"
                    if aid not in seen:
                        seen.add(aid)
                        entry["actions"].append({"id": aid, "name": c["name"], "context": "", "windows": c["windows"], "mac": c.get("mac")})
        apps[p["id"]] = entry

    sm = json.loads((REF / "app-shortcuts.json").read_text(encoding="utf-8"))["apps"]
    imported = 0
    for sm_name, (aid, display, exes, bundles) in SHORTCUTMAPPER_APPS.items():
        src = sm.get(sm_name)
        if not src:
            continue
        if aid not in apps:
            apps[aid] = {
                "id": aid, "name": display, "kind": "app",
                "match": {"windows_exe": exes, "macos_bundle": bundles, "window_title": []},
                "defaults": {},
                "actions": [],
            }
        entry = apps[aid]
        seen = {a["id"] for a in entry["actions"]}
        rows = []
        for action_name, a in src["actions"].items():
            win = translate(a["windows"]["chord"]) if "windows" in a else None
            mac = translate(a["mac"]["chord"]) if "mac" in a else None
            if not win and not mac:
                continue
            ctx = a.get("context", "")
            base = f"{aid}.{slug(action_name)}"
            ident = base
            n = 2
            while ident in seen:
                ident = f"{base}_{slug(ctx) or n}"
                if ident in seen:
                    ident = f"{base}_{n}"
                    n += 1
            seen.add(ident)
            row = {"id": ident, "name": action_name, "context": ctx}
            if win:
                row["windows"] = {"type": "keys", "chord": win}
            if mac:
                row["mac"] = {"type": "keys", "chord": mac}
            rows.append(row)
            imported += 1
        rows.sort(key=lambda r: (context_rank(r["context"]), r["context"], r["name"]))
        entry["actions"].extend(rows)
        entry["source"] = "ShortcutMapper"

    ordered = [apps[p["id"]] for p in PROFILES] + sorted(
        (a for k, a in apps.items() if k not in {p["id"] for p in PROFILES}), key=lambda a: a["name"].lower()
    )
    print(f"apps.json: {len(ordered)} apps, {imported} ShortcutMapper shortcuts imported")
    return ordered


# ---- default config ---------------------------------------------------------

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
    for event, name, action, accel in bindings:
        lines.append(f"[{prefix}.bindings.{event}]")
        lines.append(f"name = {toml_str(name)}")
        lines.append(f"action = {toml_inline(action)}")
        if accel:
            lines.append(f"accel = {toml_str(accel)}")
        lines.append("")
    return lines


HEADER = """# Create Companion configuration
# Generated by tools/gen_presets.py -- edit freely; the engine reloads on save.
#
# transport:  which F-key the Tune/Touch firmware emits for each gesture.
#             Only keys listed here are intercepted; all other keys pass through.
# profiles:   per-application bindings, matched on the foreground executable
#             (Windows) or bundle id (macOS), optionally narrowed by a
#             window_title substring (websites). Unbound events fall back to
#             default_profile. enabled = false keeps a profile but never matches it.
# accel:      none | light | medium | aggressive -- repeat count grows with dial speed.

schema_version = 1

[engine]
start_at_login = false
log_level = "info"

[transport]
TUNE_CW = { key = "F24" }
TUNE_CCW = { key = "F23" }
TUNE_TAP_1F = { key = "F22" }
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
        L.append("enabled = true")
        m = {"windows_exe": p.get("windows_exe", []), "macos_bundle": p.get("macos_bundle", [])}
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
        json.dumps({"_meta": {"source": "tools/gen_presets.py from reference/action-chords.json",
                              "count": len(catalog)},
                    "actions": catalog}, indent=1) + "\n", encoding="utf-8")
    apps = build_apps(catalog)
    (OUT / "apps.json").write_text(
        json.dumps({"_meta": {"source": "tools/gen_presets.py: hand-authored profiles + reference/app-shortcuts.json (ShortcutMapper, MIT)",
                              "count": len(apps)},
                    "apps": apps}, separators=(",", ":")) + "\n", encoding="utf-8")
    cfg = build_default_config()
    tomllib.loads(cfg)  # syntax check
    (OUT / "default-config.toml").write_text(cfg, encoding="utf-8", newline="\n")
    print(f"actions.json: {len(catalog)} actions; default-config.toml: {len(PROFILES)} app profiles")
    return 0


if __name__ == "__main__":
    sys.exit(main())
