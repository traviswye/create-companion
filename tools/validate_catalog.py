#!/usr/bin/env python3
"""Validate catalog/*.json (the per-app shortcut sources for presets/apps.json).

Usage:
    python tools/validate_catalog.py                # every file in catalog/
    python tools/validate_catalog.py catalog/a.json catalog/b.json

Exit code 1 on any error. Warnings do not fail the run.

The chord grammar below mirrors crates/companion-core/src/keys.rs; the Rust
test in companion-core is the authoritative check and runs on the generated
presets/apps.json.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "catalog"

MODIFIERS = {"CTRL", "CONTROL", "SHIFT", "ALT", "OPTION", "OPT", "WIN", "META", "CMD", "COMMAND", "SUPER", "FN", "FUNCTION", "GLOBE"}
NAMED_KEYS = {
    "TAB", "ENTER", "RETURN", "ESC", "ESCAPE", "SPACE", "BACKSPACE", "BKSP", "DELETE", "DEL",
    "INSERT", "INS", "HOME", "END", "PAGEUP", "PGUP", "PAGEDOWN", "PGDN", "LEFT", "RIGHT", "UP",
    "DOWN", "MINUS", "EQUALS", "EQUAL", "PLUS", "COMMA", "PERIOD", "SLASH", "BACKSLASH",
    "SEMICOLON", "APOSTROPHE", "QUOTE", "GRAVE", "BACKTICK", "LEFTBRACKET", "LBRACKET",
    "RIGHTBRACKET", "RBRACKET",
}
SINGLE_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-=[]\\;',./`")
MEDIA_KEYS = {"volume_up", "volume_down", "mute", "play_pause", "next_track", "previous_track", "brightness_up", "brightness_down"}
SCROLL_DIRS = {"up", "down", "left", "right"}
KINDS = {"app", "site", "system"}
OSES = {"windows", "macos", "linux"}
EVENT_RE = re.compile(r"^(TUNE|LEFT_TOUCH|RIGHT_TOUCH)_(CW|CCW|TAP|PRESS|DOUBLE_TAP|SWIPE_LEFT|SWIPE_RIGHT|SWIPE_UP|SWIPE_DOWN|PINCH|SPREAD|SCROLL_LEFT|SCROLL_RIGHT|SCROLL_UP|SCROLL_DOWN)(_[1-4]F)?$")


def chord_error(chord: str) -> str | None:
    """Mirror of ParsedChord::from_str. Returns an error message or None."""
    s = chord.strip()
    if not s:
        return "empty chord"
    tokens = s.split("+")
    if len(tokens) >= 2 and tokens[-1] == "" and tokens[-2] == "":
        tokens = tokens[:-2] + ["PLUS"]
    key_count = 0
    for tok in tokens:
        t = tok.strip()
        if t == "":
            continue
        up = t.upper()
        if up in MODIFIERS:
            continue
        if (len(t) == 1 and up in SINGLE_CHARS) or up in NAMED_KEYS or re.fullmatch(r"F([1-9]|1\d|2[0-4])", up):
            key_count += 1
            continue
        return f"unknown key `{t}` in `{chord}`"
    if key_count == 0:
        return f"`{chord}` has only modifiers"
    if key_count > 1:
        return f"`{chord}` has more than one non-modifier key (use type \"sequence\" for multi-step chords)"
    return None


def action_errors(a: object, where: str) -> list[str]:
    errs: list[str] = []
    if not isinstance(a, dict) or "type" not in a:
        return [f"{where}: action must be an object with a `type`"]
    t = a["type"]
    if t == "keys":
        if not isinstance(a.get("chord"), str):
            errs.append(f"{where}: keys action needs a `chord` string")
        else:
            e = chord_error(a["chord"])
            if e:
                errs.append(f"{where}: {e}")
    elif t == "sequence":
        chords = a.get("chords")
        if not isinstance(chords, list) or not chords:
            errs.append(f"{where}: sequence needs a non-empty `chords` list")
        else:
            for c in chords:
                e = chord_error(c) if isinstance(c, str) else "chord must be a string"
                if e:
                    errs.append(f"{where}: {e}")
    elif t == "media":
        if a.get("key") not in MEDIA_KEYS:
            errs.append(f"{where}: media key must be one of {sorted(MEDIA_KEYS)}")
    elif t == "scroll":
        if a.get("direction") not in SCROLL_DIRS:
            errs.append(f"{where}: scroll direction must be one of {sorted(SCROLL_DIRS)}")
        if "lines" in a and (not isinstance(a["lines"], int) or a["lines"] < 1):
            errs.append(f"{where}: scroll lines must be a positive integer")
    elif t == "noop":
        pass
    else:
        errs.append(f"{where}: unsupported action type `{t}` (keys | sequence | media | scroll | noop)")
    return errs


def validate_file(path: Path) -> tuple[list[str], list[str], int]:
    errs: list[str] = []
    warns: list[str] = []
    try:
        d = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        return [f"{path.name}: not valid JSON: {e}"], [], 0
    if not isinstance(d, dict):
        return [f"{path.name}: top level must be an object"], [], 0

    ident = d.get("id")
    if ident != path.stem:
        errs.append(f"{path.name}: `id` must equal the file name ({path.stem!r}), got {ident!r}")
    if not isinstance(d.get("name"), str) or not d["name"].strip():
        errs.append(f"{path.name}: `name` is required")
    kind = d.get("kind")
    if kind not in KINDS:
        errs.append(f"{path.name}: `kind` must be one of {sorted(KINDS)}")
    if not isinstance(d.get("category"), str) or not d["category"].strip():
        warns.append(f"{path.name}: no `category`")

    match = d.get("match") or {}
    if kind == "system":
        os_ = d.get("os")
        if not (isinstance(os_, list) and len(os_) == 1 and os_[0] in OSES):
            errs.append(f"{path.name}: system entries need `os` = [one of {sorted(OSES)}]")
    else:
        if not isinstance(match, dict):
            errs.append(f"{path.name}: `match` must be an object")
        else:
            for k in ("windows_exe", "macos_bundle", "window_title"):
                if k in match and not (isinstance(match[k], list) and all(isinstance(x, str) for x in match[k])):
                    errs.append(f"{path.name}: match.{k} must be a list of strings")
            has_app = bool(match.get("windows_exe")) or bool(match.get("macos_bundle"))
            has_title = bool(match.get("window_title"))
            if kind == "app" and not has_app:
                errs.append(f"{path.name}: an app needs match.windows_exe and/or match.macos_bundle")
            if kind == "site" and not has_title:
                errs.append(f"{path.name}: a site needs match.window_title")

    sources = d.get("sources")
    if not isinstance(sources, list):
        errs.append(f"{path.name}: `sources` (list of URLs) is required")
    elif not sources:
        warns.append(f"{path.name}: `sources` is empty")
    else:
        for s in sources:
            if not (isinstance(s, str) and s.startswith("http")):
                errs.append(f"{path.name}: source {s!r} is not a URL")

    actions = d.get("actions")
    if not isinstance(actions, list):
        errs.append(f"{path.name}: `actions` must be a list")
        actions = []
    seen: set[tuple[str, str]] = set()
    for i, a in enumerate(actions):
        where = f"{path.name} actions[{i}]"
        if not isinstance(a, dict):
            errs.append(f"{where}: must be an object")
            continue
        name = a.get("name")
        if not isinstance(name, str) or not name.strip():
            errs.append(f"{where}: `name` is required")
            name = ""
        ctx = a.get("context", "")
        if not isinstance(ctx, str):
            errs.append(f"{where}: `context` must be a string")
            ctx = ""
        key = (name.strip().lower(), ctx.strip().lower())
        if key in seen:
            errs.append(f"{where}: duplicate action {name!r} in context {ctx!r}")
        seen.add(key)
        plats = [p for p in ("windows", "mac", "linux") if p in a]
        if not plats:
            errs.append(f"{where}: needs at least one of windows / mac / linux")
        for p in plats:
            errs.extend(action_errors(a[p], f"{where}.{p}"))

    defaults = d.get("defaults", {})
    if not isinstance(defaults, dict):
        errs.append(f"{path.name}: `defaults` must be an object")
    else:
        for ev, b in defaults.items():
            if not EVENT_RE.match(ev):
                errs.append(f"{path.name}: defaults key {ev!r} is not a semantic event name")
            if not isinstance(b, dict) or "action" not in b:
                errs.append(f"{path.name}: defaults[{ev}] needs an `action`")
                continue
            errs.extend(action_errors(b["action"], f"{path.name} defaults[{ev}]"))
            if "mac" in b:
                errs.extend(action_errors(b["mac"], f"{path.name} defaults[{ev}].mac"))
            if "accel" in b and b["accel"] not in ("none", "light", "medium", "aggressive"):
                errs.append(f"{path.name}: defaults[{ev}].accel must be none|light|medium|aggressive")

    if len(actions) < 5 and kind != "system":
        warns.append(f"{path.name}: only {len(actions)} actions; is the list complete?")
    return errs, warns, len(actions)


def main(argv: list[str]) -> int:
    files = [Path(a) for a in argv] if argv else sorted(CATALOG.glob("*.json"))
    total_errs = 0
    total_actions = 0
    for f in files:
        errs, warns, n = validate_file(f)
        total_actions += n
        for w in warns:
            print(f"WARN  {w}")
        for e in errs:
            print(f"ERROR {e}")
        total_errs += len(errs)
        print(f"{'FAIL' if errs else 'OK  '}  {f.name}: {n} actions")
    print(f"\n{len(files)} files, {total_actions} actions, {total_errs} errors")
    return 1 if total_errs else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
