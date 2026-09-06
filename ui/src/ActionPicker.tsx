import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { chordFromEvent } from "./keys";
import { SearchBox } from "./App";
import { OS_NAME, actionFor, describeAction, eventLabel, type Action, type AppAction, type CatalogEntry, type MediaKey, type Os, type ScrollDirection } from "./types";

type Tab = "app" | "search" | "shortcut" | "media" | "scroll" | "launch" | "other";

/** `Shift+Ctrl+t` and `Ctrl+Shift+T` compare equal. */
function canonChord(c: string): string {
  const parts = c.split("+").map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return "";
  const key = parts.pop()!;
  const alias: Record<string, string> = { control: "ctrl", option: "alt", opt: "alt", command: "cmd", win: "cmd", meta: "cmd", super: "cmd", windows: "cmd" };
  const mods = parts.map((m) => m.toLowerCase()).map((m) => alias[m] ?? m).sort();
  const k = key.length === 1 ? key.toUpperCase() : key.toLowerCase();
  return [...mods, k].join("+");
}

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
  current: { action: Action; name?: string } | undefined;
  catalog: CatalogEntry[];
  appName?: string;
  appActions: AppAction[];
  /** The running OS: its chord column is offered first. */
  os: Os;
  /** The running OS's system shortcuts, for naming a recorded chord. */
  systemActions?: AppAction[];
  /** Also offer actions documented only for other platforms (tagged). */
  allPlatforms: boolean;
  onAllPlatforms: (v: boolean) => void;
  /** `null` action = remove this app-specific mapping (inherit from Default). */
  onPick: (a: Action | null, name?: string) => void;
  onClose: () => void;
}) {
  const hasApp = props.appActions.length > 0;
  const [tab, setTab] = useState<Tab>(hasApp ? "app" : "search");
  const [q, setQ] = useState("");
  const [chord, setChord] = useState(props.current?.action.type === "keys" ? props.current.action.chord : "");
  const [chordName, setChordName] = useState(props.current?.action.type === "keys" ? props.current.name ?? "" : "");
  const [chordErr, setChordErr] = useState<string | null>(null);
  const [hold, setHold] = useState(props.current?.action.type === "keys" && !!props.current.action.hold_ms);
  const [holdSec, setHoldSec] = useState(props.current?.action.type === "keys" && props.current.action.hold_ms ? props.current.action.hold_ms / 1000 : 1.5);
  /** A documented action whose chord equals what was typed or recorded. */
  const known = useMemo(() => {
    const want = canonChord(chord);
    if (!want) return null;
    const pools: { name: string; ctx?: string; a: { windows?: Action; mac?: Action; linux?: Action } }[] = [
      ...props.appActions.map((a) => ({ name: a.name, ctx: props.appName, a })),
      ...(props.systemActions ?? []).map((a) => ({ name: a.name, ctx: "System", a })),
      ...props.catalog.map((c) => ({ name: c.name, ctx: c.category, a: c })),
    ];
    for (const p of pools) {
      const act = actionFor(p.a, props.os, false)?.action;
      if (act?.type === "keys" && canonChord(act.chord) === want) return p;
    }
    return null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chord, props.appActions, props.systemActions, props.catalog, props.os]);
  // Fill the name from the catalog unless the user typed their own.
  const [nameTouched, setNameTouched] = useState(!!chordName);
  useEffect(() => {
    if (!nameTouched) setChordName(known?.name ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [known?.name]);
  const [armed, setArmed] = useState(false);
  const [program, setProgram] = useState(props.current?.action.type === "launch" ? props.current.action.program : "");
  const [args, setArgs] = useState(props.current?.action.type === "launch" ? (props.current.action.args ?? []).join(" ") : "");
  const [command, setCommand] = useState(props.current?.action.type === "command" ? props.current.action.command : "");
  const [lines, setLines] = useState(props.current?.action.type === "scroll" ? props.current.action.lines ?? 1 : 1);
  const searchRef = useRef<HTMLInputElement>(null);
  const [mod, gesture] = eventLabel(props.event);

  useEffect(() => {
    searchRef.current?.focus();
  }, [tab]);

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

  const needle = q.trim().toLowerCase();
  const col = `${props.os}:${props.allPlatforms}`;
  /** The chord for this OS (or, with All platforms, another OS's, tagged). */
  const forOs = (a: { windows?: Action; mac?: Action; linux?: Action }) => actionFor(a, props.os, props.allPlatforms);
  const appResults = useMemo(
    () =>
      props.appActions
        .filter((a) => forOs(a))
        .filter((a) => !needle || a.name.toLowerCase().includes(needle) || a.context.toLowerCase().includes(needle) || describeAction(forOs(a)?.action).toLowerCase().includes(needle))
        .slice(0, 300),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [props.appActions, needle, col],
  );

  const categories = useMemo(() => Array.from(new Set(props.catalog.map((c) => c.category))).sort(), [props.catalog]);
  const [cat, setCat] = useState<string>("All");
  const results = useMemo(
    () =>
      props.catalog
        .filter((c) => forOs(c))
        .filter((c) => cat === "All" || c.category === cat)
        .filter((c) => !needle || c.name.toLowerCase().includes(needle) || c.id.includes(needle) || describeAction(forOs(c)?.action).toLowerCase().includes(needle))
        .slice(0, 200),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [props.catalog, needle, cat, col],
  );

  async function pickChord() {
    try {
      const normalized = await api.validateChord(chord);
      const action: Action = hold ? { type: "keys", chord: normalized, hold_ms: Math.round(Math.max(0.2, Math.min(30, holdSec)) * 1000) } : { type: "keys", chord: normalized };
      props.onPick(action, chordName.trim() || undefined);
    } catch (e) {
      setChordErr(String(e));
    }
  }

  const tabs: [Tab, string][] = [
    ...(hasApp ? ([["app", `${props.appName ?? "App"} actions`]] as [Tab, string][]) : []),
    ["search", "All actions"],
    ["shortcut", "Keyboard shortcut"],
    ["media", "Media"],
    ["scroll", "Mouse / scroll"],
    ["launch", "Launch / script"],
    ["other", "Other"],
  ];

  return (
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="modal" role="dialog">
        <header>
          <h3>
            {mod} / {gesture}
            {props.current && (
              <span className="muted">
                {" "}
                · now: {props.current.name ?? describeAction(props.current.action)}
              </span>
            )}
          </h3>
          <div className="seg compact" title="Show only shortcuts that work on this computer, or every platform's">
            <button className={props.allPlatforms ? "" : "on"} onClick={() => props.onAllPlatforms(false)}>
              This computer
            </button>
            <button className={props.allPlatforms ? "on" : ""} onClick={() => props.onAllPlatforms(true)}>
              All platforms
            </button>
          </div>
          <button className="ghost" onClick={props.onClose}>✕</button>
        </header>
        <div className="mbody">
          <div className="tabs">
            {tabs.map(([t, label]) => (
              <button key={t} className={tab === t ? "on" : ""} onClick={() => setTab(t)}>
                {label}
              </button>
            ))}
          </div>

          {tab === "app" && (
            <>
              <div className="field">
                <SearchBox ref={searchRef} placeholder={`Search ${props.appName ?? "app"} shortcuts…`} value={q} onChange={setQ} />
              </div>
              <div className="list">
                {appResults.map((a) => (
                  <div key={a.id} className="row" onClick={() => props.onPick(forOs(a)!.action, a.name)}>
                    <div>
                      {a.name}
                      {a.context && <div className="sub">{a.context}</div>}
                    </div>
                    {forOs(a)?.os && <span className="ostag">{OS_NAME[forOs(a)!.os!]}</span>}
                    <kbd>{describeAction(forOs(a)?.action)}</kbd>
                  </div>
                ))}
                {appResults.length === 0 && <div className="row muted">No matches here. Try All actions or record a shortcut.</div>}
              </div>
              <div className="muted">
                {props.appActions.length} shortcuts for {props.appName}. Showing the first {Math.min(300, appResults.length)}.
              </div>
            </>
          )}

          {tab === "search" && (
            <>
              <div className="field">
                <SearchBox ref={searchRef} placeholder="Search actions, e.g. tab, zoom, undo…" value={q} onChange={setQ} />
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
                  <div key={c.id} className="row" onClick={() => props.onPick(forOs(c)!.action, c.name)}>
                    <div>
                      {c.name}
                      <div className="sub">{c.category}</div>
                    </div>
                    {forOs(c)?.os && <span className="ostag">{OS_NAME[forOs(c)!.os!]}</span>}
                    <kbd>{describeAction(forOs(c)?.action)}</kbd>
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
              <div className="grid-2">
                <div className="field">
                  <label>Keys</label>
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
                </div>
                <div className="field">
                  <label>What it does (optional)</label>
                  <input
                    placeholder="e.g. Toggle sidebar"
                    value={chordName}
                    onChange={(e) => {
                      setChordName(e.target.value);
                      setNameTouched(e.target.value.length > 0);
                    }}
                    onKeyDown={(e) => e.key === "Enter" && chord && pickChord()}
                  />
                </div>
              </div>
              {known && (
                <div className="muted">
                  Matches <b>{known.name}</b>
                  {known.ctx ? ` (${known.ctx})` : ""}.
                </div>
              )}
              {chordErr && <div className="error">{chordErr}</div>}
              <div className="field">
                <label className="inline" style={{ gap: 8, alignItems: "center" }}>
                  <input type="checkbox" checked={hold} onChange={(e) => setHold(e.target.checked)} />
                  Keep the modifiers held afterwards for
                  <input type="number" min={0.2} max={30} step={0.1} value={holdSec} disabled={!hold} onChange={(e) => setHoldSec(Number(e.target.value) || 1.5)} style={{ width: 70 }} />
                  seconds
                </label>
                <div className="muted">Saved with the shortcut; the engine does not act on it yet.</div>
              </div>
              <div className="field">
                <div className="inline">
                  <button className="primary" disabled={!chord} onClick={pickChord}>
                    Use this shortcut
                  </button>
                  <span className="muted">Modifiers: Ctrl, Shift, Alt, Win. Keys: letters, digits, F1–F24, Tab, Enter, Esc, Space, arrows, Home/End, PageUp/PageDown, punctuation.</span>
                </div>
              </div>
            </>
          )}

          {tab === "media" && (
            <div className="list">
              {MEDIA.map((m) => (
                <div key={m.key} className="row" onClick={() => props.onPick({ type: "media", key: m.key }, m.name)}>
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
                  <div key={d} className="row" onClick={() => props.onPick({ type: "scroll", direction: d, lines }, `Scroll ${d}`)}>
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
                  <button className="primary" disabled={!program} onClick={() => props.onPick({ type: "launch", program, args: args.trim() ? args.trim().split(/\s+/) : [] }, `Launch ${program.split(/[\\/]/).pop()}`)}>
                    Use
                  </button>
                </div>
              </div>
              <div className="field">
                <label>Run a shell command (cmd /C)</label>
                <div className="inline">
                  <input placeholder='powershell -Command "..."' value={command} onChange={(e) => setCommand(e.target.value)} style={{ flex: 1 }} />
                  <button className="primary" disabled={!command} onClick={() => props.onPick({ type: "command", command }, "Run command")}>
                    Use
                  </button>
                </div>
              </div>
            </>
          )}

          {tab === "other" && (
            <div className="list">
              <div className="row" onClick={() => props.onPick({ type: "noop" }, "Do nothing")}>
                <div>
                  Do nothing
                  <div className="sub">Swallow the event in this app without any action.</div>
                </div>
              </div>
              <div className="row" onClick={() => props.onPick(null)}>
                <div>
                  Use the System profile's action
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
