import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ActionPicker } from "./ActionPicker";
import { AddApp } from "./AddApp";
import { api } from "./api";
import {
  describeAction,
  eventLabel,
  eventSortKey,
  transportLabel,
  type Accel,
  type Action,
  type CatalogEntry,
  type Config,
  type EngineMsg,
  type Profile,
} from "./types";

type SaveState = { kind: "idle" } | { kind: "dirty" } | { kind: "saving" } | { kind: "saved" } | { kind: "applied" } | { kind: "error"; msg: string };

const ACCELS: Accel[] = ["none", "light", "medium", "aggressive"];
const DEFAULT = -1; // selected index for the default profile

export default function App() {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [path, setPath] = useState("");
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [sel, setSel] = useState<number>(DEFAULT);
  const [save, setSave] = useState<SaveState>({ kind: "idle" });
  const [engine, setEngine] = useState<{ connected: boolean; profile?: string; paused?: boolean; version?: string }>({ connected: false });
  const [last, setLast] = useState<{ event: string; profile: string; action: string; at: number } | null>(null);
  const [picker, setPicker] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const saveTimer = useRef<number | null>(null);

  useEffect(() => {
    api
      .loadConfig()
      .then((l) => {
        setCfg(l.config);
        setPath(l.path);
      })
      .catch((e) => setLoadErr(String(e)));
    api.actionCatalog().then(setCatalog).catch(() => setCatalog([]));
  }, []);

  useEffect(() => {
    let un: (() => void) | undefined;
    api
      .onEngine((m: EngineMsg) => {
        switch (m.type) {
          case "connected":
            setEngine((e) => ({ ...e, connected: true }));
            break;
          case "disconnected":
            setEngine((e) => ({ ...e, connected: false }));
            break;
          case "hello":
            setEngine((e) => ({ ...e, connected: true, version: m.version }));
            break;
          case "status":
            setEngine((e) => ({ ...e, connected: true, profile: m.profile, paused: m.paused }));
            break;
          case "event":
            setLast({ event: m.event!, profile: m.profile!, action: m.action!, at: Date.now() });
            setFlash(m.event!);
            window.setTimeout(() => setFlash(null), 900);
            break;
          case "config_applied":
            setSave((s) => (s.kind === "saved" || s.kind === "saving" ? { kind: "applied" } : s));
            break;
          case "config_rejected":
            setSave({ kind: "error", msg: m.error ?? "engine rejected the configuration" });
            break;
        }
      })
      .then(async (u) => {
        un = u;
        // Catch up on anything emitted before this listener existed.
        try {
          const st = await api.engineState();
          setEngine((e) => ({
            ...e,
            connected: st.connected,
            version: st.hello?.version ?? e.version,
            profile: st.status?.profile ?? e.profile,
            paused: st.status?.paused ?? e.paused,
          }));
        } catch {
          /* engine_state unavailable: stay as-is */
        }
      });
    return () => un?.();
  }, []);

  const profile: Profile | null = useMemo(() => {
    if (!cfg) return null;
    return sel === DEFAULT ? cfg.default_profile : cfg.profiles[sel] ?? null;
  }, [cfg, sel]);

  const events = useMemo(() => (cfg ? Object.keys(cfg.transport).sort((a, b) => eventSortKey(a) - eventSortKey(b)) : []), [cfg]);

  /** Apply an edit to the selected profile and schedule a save. */
  const update = useCallback(
    (fn: (p: Profile) => Profile, target: number = sel) => {
      setCfg((c) => {
        if (!c) return c;
        if (target === DEFAULT) return { ...c, default_profile: fn(c.default_profile) };
        const profiles = c.profiles.slice();
        profiles[target] = fn(profiles[target]);
        return { ...c, profiles };
      });
      setSave({ kind: "dirty" });
    },
    [sel],
  );

  // Autosave 600 ms after the last edit; the engine picks the file up.
  useEffect(() => {
    if (save.kind !== "dirty" || !cfg) return;
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(async () => {
      setSave({ kind: "saving" });
      try {
        await api.saveConfig(cfg);
        setSave({ kind: "saved" });
      } catch (e) {
        setSave({ kind: "error", msg: String(e) });
      }
    }, 600);
  }, [cfg, save.kind]);

  function setBinding(ev: string, action: Action | null) {
    update((p) => {
      const bindings = { ...p.bindings };
      if (action === null) delete bindings[ev];
      else bindings[ev] = { action, accel: bindings[ev]?.accel ?? "none" };
      return { ...p, bindings };
    });
    setPicker(null);
  }

  function setAccel(ev: string, accel: Accel) {
    update((p) => (p.bindings[ev] ? { ...p, bindings: { ...p.bindings, [ev]: { ...p.bindings[ev], accel } } } : p));
  }

  function addProfile(p: Profile) {
    setCfg((c) => (c ? { ...c, profiles: [...c.profiles, p] } : c));
    setSel(cfg ? cfg.profiles.length : 0);
    setSave({ kind: "dirty" });
    setAdding(false);
  }

  function removeProfile(i: number) {
    setCfg((c) => (c ? { ...c, profiles: c.profiles.filter((_, k) => k !== i) } : c));
    setSel(DEFAULT);
    setSave({ kind: "dirty" });
  }

  function editMatch(field: "windows_exe" | "macos_bundle" | "window_title", values: string[]) {
    update((p) => ({ ...p, match: { ...p.match, [field]: values } }));
  }

  if (loadErr) {
    return (
      <div style={{ padding: 30 }}>
        <h2>Could not load the configuration</h2>
        <pre className="error">{loadErr}</pre>
        <p className="muted">Fix the file and reopen this window.</p>
        <button onClick={() => api.openConfigFolder()}>Open configuration folder</button>
      </div>
    );
  }
  if (!cfg || !profile) return <div style={{ padding: 30 }} className="muted">Loading…</div>;

  const isDefault = sel === DEFAULT;
  const liveName = engine.profile;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <span className={"dot " + (engine.connected ? "on" : "")} title={engine.connected ? "Engine connected" : "Engine not running"} />
          Naya Companion
        </div>
        <div className="profiles">
          <div className={"profile-item " + (isDefault ? "active " : "") + (liveName === cfg.default_profile.name ? "live" : "")} onClick={() => setSel(DEFAULT)}>
            <span className="name">{cfg.default_profile.name}</span>
            <span className="badge">fallback</span>
          </div>
          {cfg.profiles.map((p, i) => (
            <div key={i} className={"profile-item " + (sel === i ? "active " : "") + (liveName === p.name ? "live" : "")} onClick={() => setSel(i)}>
              <span className="name">{p.name}</span>
              {p.match.window_title.length > 0 && <span className="badge">site</span>}
            </div>
          ))}
          <div className="profile-item" onClick={() => setAdding(true)}>
            <span className="name muted">＋ Add application</span>
          </div>
        </div>
        <div className="sidebar-foot">
          <div>
            Engine: {engine.connected ? `running${engine.version ? " v" + engine.version : ""}${engine.paused ? " (paused)" : ""}` : "not connected"}
          </div>
          <div>Active: {engine.profile ?? "—"}</div>
          <button className="ghost" style={{ justifySelf: "start", padding: "2px 6px" }} onClick={() => api.openConfigFolder()} title={path}>
            Open config folder
          </button>
        </div>
      </aside>

      <main className="main">
        <div className="topbar">
          {isDefault ? (
            <h1>{profile.name}</h1>
          ) : (
            <input className="title" value={profile.name} onChange={(e) => update((p) => ({ ...p, name: e.target.value }))} />
          )}
          <span className={"save-state " + (save.kind === "applied" ? "applied" : save.kind === "error" ? "error" : "")}>
            {save.kind === "idle" && "Changes save automatically"}
            {save.kind === "dirty" && "Editing…"}
            {save.kind === "saving" && "Saving…"}
            {save.kind === "saved" && (engine.connected ? "Saved · waiting for engine…" : "Saved")}
            {save.kind === "applied" && "Saved · applied by engine ✓"}
            {save.kind === "error" && `Error: ${save.msg}`}
          </span>
          {!isDefault && (
            <button onClick={() => removeProfile(sel)} title="Remove this profile">
              Remove
            </button>
          )}
        </div>

        <div className="content">
          <div className={"detect " + (last ? "" : "idle")}>
            {last ? (
              <>
                <span className="muted">Detected</span>
                <span className="big">
                  {eventLabel(last.event).join(" / ")}
                </span>
                <span className="muted">
                  in {last.profile} → {last.action}
                </span>
                <span style={{ flex: 1 }} />
                <button onClick={() => setPicker(last.event)}>Change for {profile.name}</button>
              </>
            ) : (
              <>
                <span>Turn the dial or swipe the Tune to detect an input.</span>
                {!engine.connected && <span>· Engine not running: start naya-companion to see live events.</span>}
              </>
            )}
          </div>

          {!isDefault && (
            <div className="card">
              <h2>Matches</h2>
              <div className="body" style={{ display: "grid", gap: 10 }}>
                <ChipEditor label="Windows executable" placeholder="app.exe" values={profile.match.windows_exe} onChange={(v) => editMatch("windows_exe", v)} mono />
                <ChipEditor label="Window title contains" placeholder="YouTube" values={profile.match.window_title} onChange={(v) => editMatch("window_title", v)} />
                <ChipEditor label="macOS bundle id" placeholder="com.example.App" values={profile.match.macos_bundle} onChange={(v) => editMatch("macos_bundle", v)} mono />
              </div>
            </div>
          )}

          <div className="card">
            <h2>
              Mappings
              <span className="spacer" />
              <span className="muted" style={{ textTransform: "none", letterSpacing: 0 }}>
                {isDefault ? "Used whenever no app profile overrides an event" : "Unset rows use the Default profile"}
              </span>
            </h2>
            <table className="map">
              <thead>
                <tr>
                  <th style={{ width: "34%" }}>Input</th>
                  <th>Action</th>
                  <th style={{ width: 130 }}>Acceleration</th>
                  <th style={{ width: 110 }}></th>
                </tr>
              </thead>
              <tbody>
                {events.map((ev) => {
                  const [mod, gesture] = eventLabel(ev);
                  const b = profile.bindings[ev];
                  const inherited = !b && !isDefault ? cfg.default_profile.bindings[ev] : undefined;
                  const repeatable = b && (b.action.type === "keys" || b.action.type === "media" || b.action.type === "scroll");
                  return (
                    <tr key={ev} className={flash === ev ? "flash" : ""}>
                      <td className="ev">
                        <div>{gesture}</div>
                        <div className="module">
                          {mod} · <span className="mono">{transportLabel(cfg.transport[ev])}</span>
                        </div>
                      </td>
                      <td className="action" onClick={() => setPicker(ev)}>
                        {b ? describeAction(b.action) : inherited ? <span className="inherit">Default: {describeAction(inherited.action)}</span> : <span className="inherit">Not bound</span>}
                      </td>
                      <td>
                        {b ? (
                          <select value={b.accel ?? "none"} disabled={!repeatable} onChange={(e) => setAccel(ev, e.target.value as Accel)} title={repeatable ? "" : "Only key, media and scroll actions repeat"}>
                            {ACCELS.map((a) => (
                              <option key={a} value={a}>
                                {a}
                              </option>
                            ))}
                          </select>
                        ) : (
                          <span className="muted">—</span>
                        )}
                      </td>
                      <td className="actions">
                        <button className="ghost" onClick={() => setPicker(ev)}>
                          Change
                        </button>
                        {b && (
                          <button className="ghost" title={isDefault ? "Unbind" : "Use Default"} onClick={() => setBinding(ev, null)}>
                            ✕
                          </button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="muted" style={{ fontSize: 12 }}>
            Config file: <span className="mono">{path}</span>. Transport keys (which F-key each gesture sends) are edited there for now.
          </div>
        </div>
      </main>

      {picker && <ActionPicker event={picker} current={profile.bindings[picker]?.action} catalog={catalog} onPick={(a) => setBinding(picker, a)} onClose={() => setPicker(null)} />}
      {adding && <AddApp onAdd={addProfile} onClose={() => setAdding(false)} />}
    </div>
  );
}

function ChipEditor(props: { label: string; placeholder: string; values: string[]; onChange: (v: string[]) => void; mono?: boolean }) {
  const [draft, setDraft] = useState("");
  function commit() {
    const v = draft.trim();
    if (v && !props.values.includes(v)) props.onChange([...props.values, v]);
    setDraft("");
  }
  return (
    <div className="field">
      <label>{props.label}</label>
      <div className="chips">
        {props.values.map((v) => (
          <span key={v} className={"chip " + (props.mono ? "mono" : "")}>
            {v}
            <button title="Remove" onClick={() => props.onChange(props.values.filter((x) => x !== v))}>
              ✕
            </button>
          </span>
        ))}
        <span className="chip-add">
          <input
            className={props.mono ? "mono" : ""}
            placeholder={props.placeholder}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && commit()}
            onBlur={commit}
          />
        </span>
      </div>
    </div>
  );
}
