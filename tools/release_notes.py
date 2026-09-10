"""Print the CHANGELOG.md section for one version, for use as GitHub Release text.

Usage: python tools/release_notes.py <version> [<owner/repo> <tag>]
The section runs from the "## <version>" heading to the next "## " heading.
Exits 1 if there is no such section.
"""
import sys
from pathlib import Path

version = sys.argv[1]
repo = sys.argv[2] if len(sys.argv) > 2 else None
tag = sys.argv[3] if len(sys.argv) > 3 else None

lines = Path(__file__).resolve().parent.parent.joinpath("CHANGELOG.md").read_text(encoding="utf-8").splitlines()
out = []
on = False
for line in lines:
    if line.startswith("## "):
        if on:
            break
        on = line.startswith("## " + version)
        continue
    if on:
        out.append(line)
body = "\n".join(out).strip("\n")
if not body:
    print(f"CHANGELOG.md has no section for {version}", file=sys.stderr)
    sys.exit(1)
# Release notes describe what a user of the installed app sees. Developer tooling under
# tools/ (capture scripts, plans, generators) is for people working on the app; it is
# documented in the README's repository layout and docs/PLAN.md, never in a release.
tooling = [line for line in body.splitlines() if "tools/" in line or "tools\\" in line]
if tooling:
    print(f"CHANGELOG.md section {version} mentions developer tooling; keep tools/ out of release notes:", file=sys.stderr)
    for line in tooling:
        print("  " + line.strip(), file=sys.stderr)
    sys.exit(1)
if repo and tag:
    body += f"\n\n---\nInstall instructions: https://github.com/{repo}/blob/{tag}/INSTALL.md\n"
# Windows consoles default to a legacy code page; the notes are UTF-8.
sys.stdout.buffer.write((body + "\n").encode("utf-8"))
