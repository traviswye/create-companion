import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { chordFromEvent } from "./keys";
import { describeAction, eventLabel, type Action, type CatalogEntry, type MediaKey, type ScrollDirection } from "./types";

type Tab = "search" | "shortcut" | "media" | "scroll" | "launch" | "other";

const MEDIA: { key: MediaKey; name: string }[] = [
  { key: "volume_up", name: "Volume up" },
  { key: "volume_down", name: "Volume down" },
  { key: "mute", name: "Mute" },
  { key: "play_pause", name: "Play / pause" },
  { key: "next_track", name: "Next track" },
  { key: "previous_track", name: "Previous track" },
];
const SCROLL: ScrollDirection[] = ["up", "down", "left", "right"];

export function ActionPicker(props: {
  event: string;
  current: Action | undefined;
  catalog: CatalogEntry[];
  onPick: (a: Action | null) => void; // null = unbind (inherit from Default)
  onClose: () => void;
}) {
  const [tab, setTab] = useState<Tab>("search");
  const [q, setQ] = useState("");
  const [chord, setChord] = useState(props.current?.type === "keys" ? props.current.chord : "");
  const [chordErr, setChordErr] = useState<string | null>(null);
  const [armed, setArmed] = useState(false);
  const [program, setProgram] = useState(props.current?.type === "launch" ? props.current.program : "");
  const [args, setArgs] = useState(props.current?.type === "launch" ? (props.current.args ?? []).join(" ") : "");
  const [command, setCommand] = useState(props.current?.type === "command" ? props.current.command : "");
  const [lines, setLines] = useState(props.current?.type === "scroll" ? props.current.lines ?? 1 : 1);
  const searchRef = useRef<HTMLInputElement>(null);
  const [mod, gesture] = eventLabel(props.event);

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  // Shortcut recorder: capture the next real key combination.
  useEffect(() => {
    if (!armed) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const c = chordFromEvent(e);
      if (c) {
        setChord(c);
        setChordErr(null);
        setArmed(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [armed]);

  const categories = useMemo(() => Array.from(new Set(props.catalog.map((c) => c.category))).sort(), [props.catalog]);
  const [cat, setCat] = useState<string>("All");
  const results = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return props.catalog
      .filter((c) => c.windows)
      .filter((c) => cat === "All" || c.category === cat)
      .filter((c) => !needle || c.name.toLowerCase().includes(needle) || c.id.includes(needle) || describeAction(c.windows).toLowerCase().includes(needle))
      .slice(0, 200);
  }, [props.catalog, q, cat]);

  async function pickChord() {
    try {
      const normalized = await api.validateChord(chord);
      props.onPick({ type: "keys", chord: normalized });
    } catch (e) {
      setChordErr(String(e));
    }
  }

  return (
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="modal" role="dialog">
        <header>
          <h3>
            {mod} / {gesture}
            {props.current && <span className="muted"> · now: {describeAction(props.current)}</span>}
          </h3>
          <button className="ghost" onClick={props.onClose}>✕</button>
        </header>
        <div className="mbody">
          <div className="tabs">
            {(
              [
                ["search", "Search actions"],
                ["shortcut", "Keyboard shortcut"],
                ["media", "Media"],
                ["scroll", "Mouse / scroll"],
                ["launch", "Launch / script"],
                ["other", "Other"],
              ] as [Tab, string][]
            ).map(([t, label]) => (
              <button key={t} className={tab === t ? "on" : ""} onClick={() => setTab(t)}>
                {label}
              </button>
            ))}
          </div>

          {tab === "search" && (
            <>
              <div className="field">
                <input ref={searchRef} placeholder="Search actions, e.g. tab, zoom, undo…" value={q} onChange={(e) => setQ(e.target.value)} />
              </div>
              <div className="tabs">
                {["All", ...categories].map((c) => (
                  <button key={c} className={cat === c ? "on" : ""} onClick={() => setCat(c)}>
                    {c}
                  </button>
                ))}
              </div>
              <div className="list">
                {results.map((c) => (
                  <div key={c.id} className="row" onClick={() => props.onPick(c.windows!)}>
                    <div>
                      {c.name}
                      <div className="sub">{c.category}</div>
                    </div>
                    <kbd>{describeAction(c.windows)}</kbd>
                  </div>
                ))}
                {results.length === 0 && <div className="row muted">No matches. Try the Keyboard shortcut tab.</div>}
              </div>
            </>
          )}

          {tab === "shortcut" && (
            <>
              <div className={"recorder " + (armed ? "armed" : "")}>
                <button className={armed ? "primary" : ""} onClick={() => setArmed((a) => !a)}>
                  {armed ? "Press keys…" : "Record"}
                </button>
                <span className="muted">{armed ? "Press the shortcut now (Esc records Esc)." : "Click Record, then press the shortcut."}</span>
              </div>
              <div className="field">
                <label>Or type it</label>
                <div className="inline">
                  <input
                    className="mono"
                    placeholder="Ctrl+Shift+T"
                    value={chord}
                    onChange={(e) => {
                      setChord(e.target.value);
                      setChordErr(null);
                    }}
                    onKeyDown={(e) => e.key === "Enter" && chord && pickChord()}
                  />
                  <button className="primary" disabled={!chord} onClick={pickChord}>
                    Use
                  </button>
                </div>
                {chordErr && <div className="error">{chordErr}</div>}
                <div className="muted">Modifiers: Ctrl, Shift, Alt, Win. Keys: letters, digits, F1–F24, Tab, Enter, Esc, Space, arrows, Home/End, PageUp/PageDown, punctuation.</div>
              </div>
            </>
          )}

          {tab === "media" && (
            <div className="list">
              {MEDIA.map((m) => (
                <div key={m.key} className="row" onClick={() => props.onPick({ type: "media", key: m.key })}>
                  <div>{m.name}</div>
                </div>
              ))}
            </div>
          )}

          {tab === "scroll" && (
            <>
              <div className="field">
                <label>Lines per detent (multiplied by acceleration)</label>
                <input type="number" min={1} max={20} value={lines} onChange={(e) => setLines(Math.max(1, Number(e.target.value) || 1))} style={{ width: 90 }} />
              </div>
              <div className="list">
                {SCROLL.map((d) => (
                  <div key={d} className="row" onClick={() => props.onPick({ type: "scroll", direction: d, lines })}>
                    <div>Scroll {d}</div>
                  </div>
                ))}
              </div>
            </>
          )}

          {tab === "launch" && (
            <>
              <div className="field">
                <label>Launch a program</label>
                <div className="inline">
                  <input placeholder="C:\\Path\\to\\app.exe or notepad" value={program} onChange={(e) => setProgram(e.target.value)} style={{ flex: 1 }} />
                  <input placeholder="arguments" value={args} onChange={(e) => setArgs(e.target.value)} style={{ width: 180 }} />
                  <button className="primary" disabled={!program} onClick={() => props.onPick({ type: "launch", program, args: args.trim() ? args.trim().split(/\s+/) : [] })}>
                    Use
                  </button>
                </div>
              </div>
              <div className="field">
                <label>Run a shell command (cmd /C)</label>
                <div className="inline">
                  <input placeholder='powershell -Command "..."' value={command} onChange={(e) => setCommand(e.target.value)} style={{ flex: 1 }} />
                  <button className="primary" disabled={!command} onClick={() => props.onPick({ type: "command", command })}>
                    Use
                  </button>
                </div>
              </div>
            </>
          )}

          {tab === "other" && (
            <div className="list">
              <div className="row" onClick={() => props.onPick({ type: "noop" })}>
                <div>
                  Do nothing
                  <div className="sub">Swallow the event in this app without any action.</div>
                </div>
              </div>
              <div className="row" onClick={() => props.onPick(null)}>
                <div>
                  Use the Default profile's action
                  <div className="sub">Remove this app-specific mapping.</div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
