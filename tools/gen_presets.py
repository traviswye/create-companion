#!/usr/bin/env python3
"""Generate the bundled presets.

Inputs:
  catalog/<id>.json          one file per app / site / system (see catalog/README.md)
  reference/action-chords.json   NayaFlow's action vocabulary -> generic catalog
  reference/app-shortcuts.json   ShortcutMapper import (20 apps, MIT) -> merged into apps

Outputs (checked in, regenerate when inputs change):
  presets/actions.json         generic action catalog (browser / system / text / ...)
  presets/apps.json            per-application catalog: match rules, shortcuts, defaults
  presets/default-config.toml  first-run config embedded in the engine
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
CATALOG = ROOT / "catalog"
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

BROWSER_EXE = ["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "vivaldi.exe"]
BROWSER_BUNDLE = ["com.google.Chrome", "com.microsoft.edgemac", "org.mozilla.firefox",
                  "com.brave.Browser", "com.apple.Safari"]

# Profiles that ship enabled in the first-run config, in this order.
DEFAULT_CONFIG_PROFILES = [
    "browser", "youtube", "terminal", "photoshop", "lightroom", "premiere", "resolve",
    "fusion360", "blender", "vscode", "discord", "spotify",
]

DEFAULT_PROFILE = {
    "name": "Default",
    "bindings": {
        "TUNE_CW": {"name": "Volume up", "action": {"type": "media", "key": "volume_up"}, "accel": "light"},
        "TUNE_CCW": {"name": "Volume down", "action": {"type": "media", "key": "volume_down"}, "accel": "light"},
        "TUNE_TAP_1F": {"name": "Mute", "action": {"type": "media", "key": "mute"}},
        "TUNE_SWIPE_LEFT": {"name": "Previous track", "action": {"type": "media", "key": "previous_track"}},
        "TUNE_SWIPE_RIGHT": {"name": "Next track", "action": {"type": "media", "key": "next_track"}},
        "TUNE_SWIPE_UP": {"name": "Play / pause", "action": {"type": "media", "key": "play_pause"}},
        "TUNE_SWIPE_DOWN": {"name": "Task view", "action": {"type": "keys", "chord": "Win+Tab"}},
    },
}

# ShortcutMapper app name -> (id, display name, exe list, bundle list). An id
# that also has a catalog file gets the ShortcutMapper rows appended to it.
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
SM_CATEGORY = {
    "after_effects": "Video", "illustrator": "Creative", "3dsmax": "CAD / 3D", "maya": "CAD / 3D",
    "ets2": "Games", "appcode": "Development", "clion": "Development", "intellij": "Development",
    "phpstorm": "Development", "pycharm": "Development", "rubymine": "Development",
    "webstorm": "Development", "houdini": "CAD / 3D", "sketchup": "CAD / 3D",
    "sublime": "Development", "nuke": "Video", "unity": "Development",
}


def translate(chord: str, mac: bool = False) -> str | None:
    """Naya token chord -> our chord grammar. On macOS LGUI is the Command key."""
    out = []
    for tok in (t.strip() for t in chord.split("+")):
        if mac and tok in ("LGUI", "RGUI"):
            out.append("Cmd")
        elif tok in TOKENS:
            out.append(TOKENS[tok])
        elif re.fullmatch(r"[A-Z]", tok):
            out.append(tok)
        elif m := re.fullmatch(r"NUMBER_(\d)", tok):
            out.append(m.group(1))
        elif re.fullmatch(r"F([1-9]|1\d|2[0-4])", tok):
            out.append(tok)
        else:
            return None  # KP_*, CLICK, consumer keys: not a plain chord
    mod_names = ("Ctrl", "Shift", "Alt", "Win", "Cmd")
    mods = [t for t in out if t in mod_names]
    keys_ = [t for t in out if t not in mod_names]
    if len(keys_) != 1:
        return None
    order = {"Ctrl": 0, "Shift": 1, "Alt": 2, "Win": 3, "Cmd": 3}
    return "+".join(sorted(set(mods), key=order.get) + keys_)


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")


def context_rank(ctx: str) -> int:
    c = ctx.lower()
    if c in ("", "global context", "main ui", "houdini", "screen", "window", "global"):
        return 0
    return 1


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
        mac = translate(a["mac"]["chord"], mac=True) if "mac" in a else None
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

def load_catalog_files() -> dict[str, dict]:
    files: dict[str, dict] = {}
    for f in sorted(CATALOG.glob("*.json")):
        d = json.loads(f.read_text(encoding="utf-8"))
        if d.get("id") != f.stem:
            raise SystemExit(f"{f.name}: id {d.get('id')!r} does not match the file name")
        files[f.stem] = d
    return files


class ActionSink:
    """Collects an app's actions with stable, unique ids."""

    def __init__(self, app_id: str):
        self.app_id = app_id
        self.rows: list[dict] = []
        self.ids: set[str] = set()

    def add(self, name: str, context: str, windows=None, mac=None, linux=None) -> None:
        base = f"{self.app_id}.{slug(name)}"
        ident = base
        if ident in self.ids and context:
            ident = f"{base}_{slug(context)}"
        n = 2
        while ident in self.ids:
            ident = f"{base}_{n}"
            n += 1
        self.ids.add(ident)
        row = {"id": ident, "name": name, "context": context}
        if windows is not None:
            row["windows"] = windows
        if mac is not None:
            row["mac"] = mac
        if linux is not None:
            row["linux"] = linux
        self.rows.append(row)


def build_apps(generic: list[dict], files: dict[str, dict]) -> list[dict]:
    apps: dict[str, dict] = {}

    for aid, c in files.items():
        sink = ActionSink(aid)
        entry = {
            "id": aid,
            "name": c["name"],
            "kind": c.get("kind", "app"),
            "category": c.get("category", ""),
            "match": {
                "windows_exe": c.get("match", {}).get("windows_exe", []),
                "macos_bundle": c.get("match", {}).get("macos_bundle", []),
                "window_title": c.get("match", {}).get("window_title", []),
            },
            "sources": c.get("sources", []),
            "defaults": c.get("defaults", {}),
            "actions": [],
        }
        if c.get("os"):
            entry["os"] = c["os"]
        if c.get("title_required"):
            entry["title_required"] = True
        # Documented actions first (they carry the correct per-platform chords);
        # then any default binding whose name has no action yet, on the
        # platforms the app matches on (a Mac-only app has no Windows column).
        for a in c.get("actions", []):
            if any(r["name"].lower() == a["name"].lower() and r["context"].lower() == a.get("context", "").lower() for r in sink.rows):
                continue
            sink.add(a["name"], a.get("context", ""), windows=a.get("windows"), mac=a.get("mac"), linux=a.get("linux"))
        on_windows = bool(entry["match"]["windows_exe"]) or entry["kind"] == "site" or "windows" in (c.get("os") or [])
        on_mac = bool(entry["match"]["macos_bundle"]) or entry["kind"] == "site" or "macos" in (c.get("os") or [])
        if not on_windows and not on_mac:
            on_windows = on_mac = True
        for _ev, b in entry["defaults"].items():
            if b.get("name") and not any(r["name"].lower() == b["name"].lower() for r in sink.rows):
                sink.add(b["name"], "", windows=b["action"] if on_windows else None, mac=b["action"] if on_mac else None)
        if cat := c.get("actions_from_category"):
            for g in generic:
                if g["category"] == cat and not any(r["name"].lower() == g["name"].lower() for r in sink.rows):
                    sink.add(g["name"], "", windows=g["windows"], mac=g.get("mac"))
        entry["actions"] = sink.rows
        entry["_sink"] = sink
        apps[aid] = entry

    sm = json.loads((REF / "app-shortcuts.json").read_text(encoding="utf-8"))["apps"]
    imported = 0
    for sm_name, (aid, display, exes, bundles) in SHORTCUTMAPPER_APPS.items():
        src = sm.get(sm_name)
        if not src:
            continue
        if aid not in apps:
            apps[aid] = {
                "id": aid, "name": display, "kind": "app", "category": SM_CATEGORY.get(aid, ""),
                "match": {"windows_exe": exes, "macos_bundle": bundles, "window_title": []},
                "sources": ["https://github.com/waldobronchart/ShortcutMapper"],
                "defaults": {},
                "actions": [],
                "_sink": ActionSink(aid),
            }
        entry = apps[aid]
        sink: ActionSink = entry["_sink"]
        existing = {(r["name"].lower(), r["context"].lower()) for r in sink.rows}
        rows = []
        for action_name, a in src["actions"].items():
            win = translate(a["windows"]["chord"]) if "windows" in a else None
            mac = translate(a["mac"]["chord"], mac=True) if "mac" in a else None
            if not win and not mac:
                continue
            ctx = a.get("context", "")
            if (action_name.lower(), ctx.lower()) in existing:
                continue
            rows.append((action_name, ctx, win, mac))
            imported += 1
        rows.sort(key=lambda r: (context_rank(r[1]), r[1], r[0]))
        for action_name, ctx, win, mac in rows:
            sink.add(action_name, ctx,
                     windows={"type": "keys", "chord": win} if win else None,
                     mac={"type": "keys", "chord": mac} if mac else None)
        entry["actions"] = sink.rows
        if "https://github.com/waldobronchart/ShortcutMapper" not in entry["sources"]:
            entry["sources"].append("https://github.com/waldobronchart/ShortcutMapper")
        entry["source"] = "ShortcutMapper"

    for e in apps.values():
        e.pop("_sink", None)

    # An app that also runs in a browser: split into the desktop entry (matched
    # on exe / bundle) and a site twin (matched on window title inside a browser)
    # sharing the same actions and defaults. A desktop-exe rule combined with a
    # title rule would otherwise never match inside Chrome.
    twins = {}
    for aid, e in list(apps.items()):
        if e.get("title_required"):
            continue  # the title narrows the desktop match (tmux inside a terminal); no twin
        if e["kind"] == "app" and e["match"]["window_title"] and (e["match"]["windows_exe"] or e["match"]["macos_bundle"]):
            twin = json.loads(json.dumps(e))
            twin["id"] = f"{aid}_web"
            twin["name"] = f"{e['name']} (web)"
            twin["kind"] = "site"
            twin["match"] = {"windows_exe": list(BROWSER_EXE), "macos_bundle": list(BROWSER_BUNDLE), "window_title": e["match"]["window_title"]}
            for r in twin["actions"]:
                r["id"] = r["id"].replace(f"{aid}.", f"{aid}_web.", 1)
            twin["twin_of"] = aid
            twins[twin["id"]] = twin
            e["match"]["window_title"] = []
    apps.update(twins)

    first = [apps[i] for i in DEFAULT_CONFIG_PROFILES if i in apps]
    rest = sorted((a for k, a in apps.items() if k not in DEFAULT_CONFIG_PROFILES), key=lambda a: a["name"].lower())
    total = sum(len(a["actions"]) for a in apps.values())
    print(f"apps.json: {len(apps)} apps, {total} actions ({imported} from ShortcutMapper)")
    return first + rest


# ---- default config ---------------------------------------------------------

def toml_str(s: str) -> str:
    return json.dumps(s)  # JSON string escaping is valid TOML basic-string escaping


def toml_value(v) -> str:
    if isinstance(v, str):
        return toml_str(v)
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(toml_value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k} = {toml_value(x)}" for k, x in v.items()) + " }"
    raise TypeError(type(v))


def emit_bindings(prefix: str, bindings: dict) -> list[str]:
    lines = []
    for event, b in bindings.items():
        lines.append(f"[{prefix}.bindings.{event}]")
        if b.get("name"):
            lines.append(f"name = {toml_str(b['name'])}")
        lines.append(f"action = {toml_value(b['action'])}")
        if b.get("accel"):
            lines.append(f"accel = {toml_str(b['accel'])}")
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


def build_default_config(files: dict[str, dict]) -> str:
    L = HEADER.splitlines() + [
        "",
        "[default_profile]",
        f"name = {toml_str(DEFAULT_PROFILE['name'])}",
        "",
    ]
    L += emit_bindings("default_profile", DEFAULT_PROFILE["bindings"])
    for aid in DEFAULT_CONFIG_PROFILES:
        c = files[aid]
        L.append("[[profiles]]")
        L.append(f"name = {toml_str(c['name'])}")
        L.append("enabled = true")
        m = {"windows_exe": c["match"].get("windows_exe", []), "macos_bundle": c["match"].get("macos_bundle", [])}
        if c["match"].get("window_title"):
            m["window_title"] = c["match"]["window_title"]
        L.append("match = " + toml_value(m))
        L.append("")
        L += emit_bindings("profiles", c.get("defaults", {}))
    return "\n".join(L).rstrip() + "\n"


def main() -> int:
    OUT.mkdir(exist_ok=True)
    generic = build_catalog()
    (OUT / "actions.json").write_text(
        json.dumps({"_meta": {"source": "tools/gen_presets.py from reference/action-chords.json",
                              "count": len(generic)},
                    "actions": generic}, indent=1) + "\n", encoding="utf-8")
    files = load_catalog_files()
    missing = [i for i in DEFAULT_CONFIG_PROFILES if i not in files]
    if missing:
        raise SystemExit(f"catalog files missing for default-config profiles: {missing}")
    apps = build_apps(generic, files)
    (OUT / "apps.json").write_text(
        json.dumps({"_meta": {"source": "tools/gen_presets.py: catalog/*.json + reference/app-shortcuts.json (ShortcutMapper, MIT)",
                              "count": len(apps)},
                    "apps": apps}, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")
    cfg = build_default_config(files)
    tomllib.loads(cfg)  # syntax check
    (OUT / "default-config.toml").write_text(cfg, encoding="utf-8", newline="\n")
    print(f"actions.json: {len(generic)} actions; default-config.toml: {len(DEFAULT_CONFIG_PROFILES)} app profiles")
    return 0


if __name__ == "__main__":
    sys.exit(main())
