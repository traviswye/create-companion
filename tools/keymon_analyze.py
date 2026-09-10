"""Read a keymon.ps1 capture and report how modules send their modifier namespaces.

Usage:
    powershell -File tools/keymon.ps1 -Seconds 40 -OnlyFKeys | Tee-Object -FilePath burst.log
    python tools/keymon_analyze.py burst.log [--window 100]

With a gesture_capture.ps1 log, events are grouped by their "### gesture:"
header (one capture per prompt); with a plain keymon log they are grouped into
bursts (runs of F13-F24 events with gaps under 300 ms). For each group it prints:
  - how many F-key presses it holds and which keys
  - the modifier pattern: PER-EVENT (a modifier press and release around each
    F-key: safe), SPANNING (one modifier press covering several F-keys: the
    timing rule misclassifies repeats after the window), or NONE
  - the largest gap between a modifier press and the F-key that followed it
  - every F-key press whose modifier had already been down longer than the
    window when it arrived, i.e. what the engine would treat as user-held
It ends with a summary table, one row per group: presses, span, keys (with the
modifier that was down), and ONCE / STREAM / NONE, so a census of a module's
gestures answers "one key or a run?" at a glance.
"""
import re
import sys
from datetime import datetime

WINDOW_MS = 100
BURST_GAP_MS = 300
MODS = {"LShift", "RShift", "Shift", "LCtrl", "RCtrl", "Ctrl", "LAlt", "RAlt", "Alt", "LWin", "RWin"}
LINE = re.compile(r"^(\d\d:\d\d:\d\d\.\d{3})\s+(?:hw\+\s*(-?\d+)ms\s+)?(DOWN|UP)\s+(\S+)\s+vk=0x([0-9A-F]+)")


SECTION = re.compile(r"^###\s+gesture:\s+(.*?)(?:\s+expect=(\S+))?\s*(\[.*\])?\s*$")


def parse(path):
    """Events, each tagged with the gesture section it belongs to (None for a plain keymon log)."""
    events = []
    section = None
    for raw in open(path, encoding="utf-8", errors="replace"):
        line = raw.strip("\ufeff \r\n")
        sm = SECTION.match(line)
        if sm:
            section = (sm.group(1) + (" " + sm.group(3) if sm.group(3) else ""), sm.group(2))
            events.append({"section": section, "marker": True})
            continue
        m = LINE.match(line)
        if not m:
            continue
        t = datetime.strptime(m.group(1), "%H:%M:%S.%f")
        ms = t.hour * 3600000 + t.minute * 60000 + t.second * 1000 + t.microsecond // 1000
        events.append({"ms": ms, "lag": int(m.group(2) or 0), "down": m.group(3) == "DOWN", "key": m.group(4), "vk": int(m.group(5), 16), "section": section})
    return events


def is_f(e):
    return 0x7C <= e["vk"] <= 0x87


def keys_seen(b):
    """The distinct keys a group delivered, each as key or mods+key with the modifiers down at that instant."""
    seen = set()
    held = {}
    for e in b:
        if e["key"] in MODS:
            held[e["key"].lstrip("LR")] = e["down"]
        elif is_f(e) and e["down"]:
            mods = "+".join(sorted(k for k, d in held.items() if d))
            seen.add(f"{mods}+{e['key']}" if mods else e["key"])
    return seen


def groups(events):
    """(label, expect, events) per gesture section, or per timing burst."""
    if any(e.get("marker") for e in events):
        out, cur, label = [], [], None
        for e in events:
            if e.get("marker"):
                if label is not None:
                    out.append((label[0], label[1], cur))
                label, cur = e["section"], []
            else:
                cur.append(e)
        if label is not None:
            out.append((label[0], label[1], cur))
        return out
    return [(f"burst {i}", None, b) for i, b in enumerate(bursts(events), 1)]


def bursts(events):
    out, cur, last = [], [], None
    for e in events:
        if last is not None and e["ms"] - last > BURST_GAP_MS and any(is_f(x) for x in cur):
            out.append(cur)
            cur = []
        cur.append(e)
        last = e["ms"]
    if any(is_f(x) for x in cur):
        out.append(cur)
    return out


def analyze(path, window):
    events = parse(path)
    if not events:
        print("no keymon lines found in", path)
        return
    print(f"{len([e for e in events if not e.get('marker')])} events, window {window} ms\n")
    rows = []  # (presses, span_ms, keys, shape, verdict, label) for the summary table
    for label, expect, b in groups(events):
        if not b:
            print(f"{label}: no events captured\n")
            rows.append((0, 0, "", "NONE", "", label))
            continue
        f_down = [e for e in b if is_f(e) and e["down"]]
        mod_down = [e for e in b if e["key"] in MODS and e["down"]]
        mod_up = [e for e in b if e["key"] in MODS and not e["down"]]
        keys = sorted({e["key"] for e in f_down})
        if not mod_down:
            pattern = "NONE"
        elif len(mod_down) >= len(f_down):
            pattern = "PER-EVENT"
        else:
            pattern = "SPANNING"
        # For each F-key press, how long had its modifier been down?
        gaps, misclassified = [], []
        down_since = {}
        for e in b:
            if e["key"] in MODS:
                slot = e["key"].lstrip("LR")
                if e["down"]:
                    down_since.setdefault(slot, e["ms"])
                else:
                    down_since.pop(slot, None)
            elif is_f(e) and e["down"] and down_since:
                age = max(e["ms"] - t for t in down_since.values())
                gaps.append(age)
                if age > window:
                    misclassified.append((e["key"], age, "+".join(sorted(down_since))))
        span = b[-1]["ms"] - b[0]["ms"]
        lag = max(e["lag"] for e in b)
        print(f"{label}: {len(f_down)} F-key presses ({', '.join(keys)}) over {span} ms, max delivery lag {lag} ms")
        seen = keys_seen(b)
        verdict = ""
        if expect:
            # what arrived, as key or mods+key, against what the config says this gesture sends
            verdict = "MATCH" if seen == {expect} else "MISMATCH"
            print(f"  expected {expect}, got {', '.join(sorted(seen)) or 'nothing'}: {verdict}")
        shape = "NONE" if not f_down else "ONCE" if len(f_down) == 1 else f"STREAM x{len(f_down)}"
        rows.append((len(f_down), span, ", ".join(sorted(seen)), shape, verdict, label))
        if len(f_down) > 1:
            gaps_between = [f_down[i]["ms"] - f_down[i - 1]["ms"] for i in range(1, len(f_down))]
            print(f"  {len(f_down)} presses from one gesture: gaps between presses {min(gaps_between)}-{max(gaps_between)} ms (a firmware burst if you performed it once)")
        print(f"  modifier pattern: {pattern}  ({len(mod_down)} modifier presses, {len(mod_up)} releases)")
        if gaps:
            print(f"  modifier-to-key gap: max {max(gaps)} ms, mean {sum(gaps) / len(gaps):.1f} ms")
        if misclassified:
            print(f"  WOULD BE TREATED AS USER-HELD ({len(misclassified)}):")
            for key, age, mods in misclassified:
                print(f"    {mods}+{key}: modifier down {age} ms before the key")
        elif pattern == "SPANNING":
            print("  spanning but every key landed inside the window; a longer run would not")
        print()
    summary(rows)


def summary(rows):
    """One line per group: did the gesture send one key or a run?"""
    if not rows:
        return
    kw = max(4, *(len(r[2]) for r in rows))
    print("Summary: presses per gesture (ONCE = one key per gesture, STREAM = a run of keys from one gesture)")
    print(f"  {'presses':>7}  {'span ms':>7}  {'keys':<{kw}}  {'verdict':<16}  gesture")
    for presses, span, keys, shape, verdict, label in rows:
        print(f"  {presses:>7}  {span:>7}  {keys:<{kw}}  {(shape + ' ' + verdict).strip():<16}  {label}")
    once = sum(1 for r in rows if r[3] == "ONCE")
    streams = [r for r in rows if r[3].startswith("STREAM")]
    print(f"  {once} gesture(s) sent one key, {len(streams)} sent a run.")
    if streams:
        print("  Runs: " + "; ".join(f"{r[5]} ({r[0]})" for r in streams))


if __name__ == "__main__":
    args = sys.argv[1:]
    window = WINDOW_MS
    if "--window" in args:
        i = args.index("--window")
        window = int(args[i + 1])
        del args[i : i + 2]
    if not args:
        print(__doc__)
        sys.exit(1)
    analyze(args[0], window)
