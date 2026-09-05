"""Read a keymon.ps1 capture and report how modules send their modifier namespaces.

Usage:
    powershell -File tools/keymon.ps1 -Seconds 40 -OnlyFKeys | Tee-Object -FilePath burst.log
    python tools/keymon_analyze.py burst.log [--window 100]

For every burst (a run of F13-F24 events with gaps under 300 ms) it prints:
  - how many F-key presses it holds and which keys
  - the modifier pattern: PER-EVENT (a modifier press and release around each
    F-key: safe), SPANNING (one modifier press covering several F-keys: the
    timing rule misclassifies repeats after the window), or NONE
  - the largest gap between a modifier press and the F-key that followed it
  - every F-key press whose modifier had already been down longer than the
    window when it arrived, i.e. what the engine would treat as user-held
"""
import re
import sys
from datetime import datetime

WINDOW_MS = 100
BURST_GAP_MS = 300
MODS = {"LShift", "RShift", "Shift", "LCtrl", "RCtrl", "Ctrl", "LAlt", "RAlt", "Alt", "LWin", "RWin"}
LINE = re.compile(r"^(\d\d:\d\d:\d\d\.\d{3})\s+(?:hw\+\s*(-?\d+)ms\s+)?(DOWN|UP)\s+(\S+)\s+vk=0x([0-9A-F]+)")


def parse(path):
    events = []
    for raw in open(path, encoding="utf-8", errors="replace"):
        m = LINE.match(raw.strip("\ufeff \r\n"))
        if not m:
            continue
        t = datetime.strptime(m.group(1), "%H:%M:%S.%f")
        ms = t.hour * 3600000 + t.minute * 60000 + t.second * 1000 + t.microsecond // 1000
        events.append({"ms": ms, "lag": int(m.group(2) or 0), "down": m.group(3) == "DOWN", "key": m.group(4), "vk": int(m.group(5), 16)})
    return events


def is_f(e):
    return 0x7C <= e["vk"] <= 0x87


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
    print(f"{len(events)} events, window {window} ms\n")
    for n, b in enumerate(bursts(events), 1):
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
        print(f"burst {n}: {len(f_down)} F-key presses ({', '.join(keys)}) over {span} ms, max delivery lag {lag} ms")
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
