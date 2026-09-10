"""Read an input_capture.ps1 log and report what the PC received per gesture.

Usage:
    python tools/input_analyze.py tools/plans/stock-touch.log

Events are grouped by their "### gesture:" / "### stream" header; a header-less
log is grouped into bursts (gaps over 300 ms). For each group it prints:
  - the key chords (Ctrl+Tab, Win+Tab, ...) and how many times each was pressed
  - button clicks by button
  - wheel and horizontal-wheel events, notches (delta / 120) and net delta
  - cursor moves: count, net dx/dy, path length, and the report rate
  - the span from first to last event and the largest gap inside the group
and ends with a summary table, one row per gesture.
"""
import re
import sys
from datetime import datetime

BURST_GAP_MS = 300
SECTION = re.compile(r"^###\s+(gesture:\s+(.*?)|stream.*?)(?:\s+expect=(\S+))?\s*(\[.*\])?\s*$")
LINE = re.compile(r"^(\d\d:\d\d:\d\d\.\d{3})\s+\+\s*(\d+)ms\s+(KEY|MOUSE)\s+(.*?)\s*$")
KEY = re.compile(r"^(DOWN|UP)\s+(\S+)\s+vk=0x([0-9A-F]+)\s+sc=0x[0-9A-F]+(?:\s+chord=(\S+))?")
MOVE = re.compile(r"^MOVE\s+x=(-?\d+),y=(-?\d+)\s+dx=(-?\d+),dy=(-?\d+)")
WHEEL = re.compile(r"^(WHEEL|HWHEEL)\s+delta=([+-]?\d+)")
BUTTON = re.compile(r"^(DOWN|UP)\s+(\S+)")


def parse(path):
    events, section = [], None
    for raw in open(path, encoding="utf-8", errors="replace"):
        line = raw.strip("\ufeff \r\n")
        sm = SECTION.match(line)
        if sm:
            label = sm.group(2) if sm.group(2) is not None else sm.group(1)
            if sm.group(4):
                label += " " + sm.group(4)
            section = (label, sm.group(3))
            events.append({"marker": True, "section": section})
            continue
        m = LINE.match(line)
        if not m:
            continue
        t = datetime.strptime(m.group(1), "%H:%M:%S.%f")
        ms = t.hour * 3600000 + t.minute * 60000 + t.second * 1000 + t.microsecond // 1000
        e = {"ms": ms, "rel": int(m.group(2)), "kind": m.group(3), "rest": m.group(4), "section": section}
        rest = m.group(4)
        if e["kind"] == "KEY":
            k = KEY.match(rest)
            if k:
                e.update(down=k.group(1) == "DOWN", key=k.group(2), chord=k.group(4))
        else:
            mv, wh, bt = MOVE.match(rest), WHEEL.match(rest), BUTTON.match(rest)
            if mv:
                e.update(sub="move", dx=int(mv.group(3)), dy=int(mv.group(4)))
            elif wh:
                e.update(sub=wh.group(1).lower(), delta=int(wh.group(2)))
            elif bt:
                e.update(sub="button", down=bt.group(1) == "DOWN", button=bt.group(2))
        events.append(e)
    return events


def groups(events):
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
    out, cur, last = [], [], None
    for e in events:
        if last is not None and e["ms"] - last > BURST_GAP_MS and cur:
            out.append(cur)
            cur = []
        cur.append(e)
        last = e["ms"]
    if cur:
        out.append(cur)
    return [(f"burst {i}", None, b) for i, b in enumerate(out, 1)]


def describe(b):
    """(one-line summary, observed tokens) for a group."""
    chords, clicks = {}, {}
    wheel = {"wheel": [0, 0], "hwheel": [0, 0]}
    moves, dx, dy, path = 0, 0, 0, 0.0
    for e in b:
        if e["kind"] == "KEY":
            if e.get("down") and e.get("chord"):
                chords[e["chord"]] = chords.get(e["chord"], 0) + 1
        elif e.get("sub") == "move":
            moves += 1
            dx += e["dx"]
            dy += e["dy"]
            path += (e["dx"] ** 2 + e["dy"] ** 2) ** 0.5
        elif e.get("sub") in wheel:
            wheel[e["sub"]][0] += 1
            wheel[e["sub"]][1] += e["delta"]
        elif e.get("sub") == "button" and e.get("down"):
            clicks[e["button"].lower()] = clicks.get(e["button"].lower(), 0) + 1
    parts, tokens = [], []
    if chords:
        tokens += list(chords)
        parts.append("keys: " + ", ".join(f"{k} x{n}" for k, n in sorted(chords.items())))
    if clicks:
        tokens += [f"click:{k}" for k in clicks]
        parts.append("clicks: " + ", ".join(f"{k} x{n}" for k, n in sorted(clicks.items())))
    for name in ("wheel", "hwheel"):
        n, d = wheel[name]
        if n:
            tokens.append(name)
            parts.append(f"{name}: {n} event(s), {d / 120:g} notch(es) (delta {d:+d})")
    if moves:
        tokens.append("move")
        gaps = [b[i]["ms"] - b[i - 1]["ms"] for i in range(1, len(b)) if b[i].get("sub") == "move" and b[i - 1].get("sub") == "move"]
        rate = f", ~{1000 / (sum(gaps) / len(gaps)):.0f} reports/s" if gaps and sum(gaps) else ""
        parts.append(f"moves: {moves} (dx {dx:+d}, dy {dy:+d}, path {path:.0f} px{rate})")
    return (" | ".join(parts) if parts else "nothing"), tokens


def analyze(path):
    events = parse(path)
    if not events:
        print("no input_capture lines found in", path)
        return
    print(f"{len([e for e in events if not e.get('marker')])} events\n")
    rows = []
    for label, expect, b in groups(events):
        if not b:
            print(f"{label}: no events captured")
            v = ""
            if expect:
                v = "MATCH" if expect.strip().lower() == "none" else f"MISMATCH (expected {expect})"
                print(f"  {v}")
            print()
            rows.append((0, 0, "nothing", v, label))
            continue
        text, tokens = describe(b)
        span = b[-1]["ms"] - b[0]["ms"]
        gaps = [b[i]["ms"] - b[i - 1]["ms"] for i in range(1, len(b))]
        print(f"{label}: {len(b)} event(s) over {span} ms" + (f", largest gap {max(gaps)} ms" if gaps else ""))
        print(f"  {text}")
        verdict = ""
        if expect:
            exp = [t.strip().lower() for t in expect.split(",") if t.strip()]
            obs = [t.lower() for t in tokens]
            if "none" in exp:
                verdict = "MATCH" if not obs else "MISMATCH (expected nothing)"
            else:
                missing = [t for t in exp if t not in obs]
                extra = [t for t in obs if t not in exp]
                verdict = "MATCH" if not missing else "MISMATCH (missing " + ", ".join(missing) + ")"
                if extra:
                    verdict += " +extra " + ", ".join(extra)
            print(f"  stock map says {expect}: {verdict}")
        print()
        rows.append((len(b), span, text, verdict, label))
    print("Summary: what the PC received per gesture")
    print(f"  {'events':>6}  {'span ms':>7}  {'verdict':<28}  gesture  |  observed")
    for n, span, text, verdict, label in rows:
        print(f"  {n:>6}  {span:>7}  {verdict:<28}  {label}  |  {text}")
    bad = [r for r in rows if r[3].startswith("MISMATCH")]
    if bad:
        print(f"  {len(bad)} gesture(s) did not match the stock map: " + "; ".join(r[4] for r in bad))


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    analyze(sys.argv[1])
