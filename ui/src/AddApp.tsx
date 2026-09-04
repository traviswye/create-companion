import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import type { Profile, WindowInfo } from "./types";

export function AddApp(props: { onAdd: (p: Profile) => void; onClose: () => void }) {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [name, setName] = useState("");
  const [exe, setExe] = useState("");
  const [title, setTitle] = useState("");
  const [q, setQ] = useState("");

  useEffect(() => {
    api.runningWindows().then(setWindows).catch(() => setWindows([]));
  }, []);

  const grouped = useMemo(() => {
    const m = new Map<string, string[]>();
    for (const w of windows) {
      const k = w.exe.toLowerCase();
      if (!m.has(k)) m.set(k, []);
      const arr = m.get(k)!;
      if (arr.length < 3) arr.push(w.title);
    }
    const needle = q.toLowerCase();
    return Array.from(m.entries())
      .map(([k, titles]) => ({ exe: windows.find((w) => w.exe.toLowerCase() === k)!.exe, titles }))
      .filter((g) => !needle || g.exe.toLowerCase().includes(needle) || g.titles.some((t) => t.toLowerCase().includes(needle)));
  }, [windows, q]);

  function choose(exeName: string) {
    setExe(exeName);
    if (!name) setName(exeName.replace(/\.exe$/i, ""));
  }

  async function browse() {
    const f = await open({ multiple: false, filters: [{ name: "Programs", extensions: ["exe"] }] });
    if (typeof f === "string") {
      const base = f.split(/[\\/]/).pop() ?? f;
      choose(base);
    }
  }

  function add() {
    props.onAdd({
      name: name.trim() || exe.replace(/\.exe$/i, ""),
      match: {
        windows_exe: exe.trim() ? [exe.trim()] : [],
        macos_bundle: [],
        window_title: title.trim() ? [title.trim()] : [],
      },
      bindings: {},
    });
  }

  const valid = exe.trim().length > 0 || title.trim().length > 0;

  return (
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="modal" role="dialog">
        <header>
          <h3>Add application</h3>
          <button className="ghost" onClick={props.onClose}>✕</button>
        </header>
        <div className="mbody">
          <div className="field">
            <label>Pick a running application</label>
            <input placeholder="Filter…" value={q} onChange={(e) => setQ(e.target.value)} />
          </div>
          <div className="list">
            {grouped.map((g) => (
              <div key={g.exe} className={"row " + (g.exe === exe ? "on" : "")} onClick={() => choose(g.exe)}>
                <div>
                  <span className="mono">{g.exe}</span>
                  <div className="sub">{g.titles.join(" · ")}</div>
                </div>
              </div>
            ))}
            {grouped.length === 0 && <div className="row muted">No visible windows found.</div>}
          </div>
          <div className="grid-2">
            <div className="field">
              <label>Executable name</label>
              <div className="inline">
                <input className="mono" placeholder="app.exe" value={exe} onChange={(e) => setExe(e.target.value)} style={{ flex: 1 }} />
                <button onClick={browse}>Browse…</button>
              </div>
            </div>
            <div className="field">
              <label>Profile name</label>
              <input placeholder="Shown in the list" value={name} onChange={(e) => setName(e.target.value)} />
            </div>
          </div>
          <div className="field">
            <label>Only when the window title contains (optional, for websites)</label>
            <input placeholder="e.g. YouTube" value={title} onChange={(e) => setTitle(e.target.value)} />
            <div className="muted">Leave the executable empty and set a title to match any app whose title contains the text.</div>
          </div>
        </div>
        <footer>
          <button onClick={props.onClose}>Cancel</button>
          <button className="primary" disabled={!valid} onClick={add}>
            Add
          </button>
        </footer>
      </div>
    </div>
  );
}
