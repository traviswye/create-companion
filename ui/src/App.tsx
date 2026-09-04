import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ActionPicker } from "./ActionPicker";
import { AddApp } from "./AddApp";
import { Inputs, type Learned } from "./Inputs";
import { api } from "./api";
import {
  describeAction,
  eventLabel,
  eventSortKey,
  transportLabel,
  type Accel,
  type Action,
  type AppEntry,
  type Binding,
  type CatalogEntry,
  type Config,
  type EngineMsg,
  type Profile,
} from "./types";

type SaveState = { kind: "idle" } | { kind: "dirty" } | { kind: "saving" } | { kind: "saved" } | { kind: "applied" } | { kind: "error"; msg: string };
type NavTab = "active" | "available";

const ACCELS: Accel[] = ["none", "light", "medium", "aggressive"];
const DEFAULT = -1; // selected index for the default profile
const INPUTS = -2; // the Inputs (transport) editor

function isEnabled(p: Profile) {
  return p.enabled !== false;
}

/** Which action column applies on this OS. */
export function osColumn(os: "windows" | "macos" | "linux"): "windows" | "mac" | "linux" {
  return os === "macos" ? "mac" : os === "linux" ? "linux" : "windows";
}

/** Can this catalog entry run on the given OS? */
function entryOnOs(a: AppEntry, os: "windows" | "macos" | "linux"): boolean {
  if (a.kind === "system") return a.os?.includes(os) ?? false;
  if (a.kind === "site") return true;
  if (os === "macos") return a.match.macos_bundle.length > 0;
  return a.match.windows_exe.length > 0;
}

function currentOs(): "windows" | "macos" | "linux" {
  const p = navigator.platform.toLowerCase();
  if (p.startsWith("win")) return "windows";
  if (p.startsWith("mac")) return "macos";
  return "linux";
}

function matchText(p: { name: string; match: Profile["match"] }) {
  return [p.name, ...p.match.windows_exe, ...p.match.window_title, ...p.match.macos_bundle].join(" ").toLowerCase();
}

/**
 * Link a profile to its catalog entry: by name first, then by a shared
 * executable or bundle id with the same kind of title rule (so "Photoshop"
 * still finds "Adobe Photoshop", and Browser does not swallow YouTube).
 */
function catalogFor(apps: AppEntry[], p: Profile): AppEntry | undefined {
  const byName = apps.find((a) => a.name.toLowerCase() === p.name.toLowerCase());
  if (byName) return byName;
  const exes = new Set(p.match.windows_exe.map((e) => e.toLowerCase()));
  const bundles = new Set(p.match.macos_bundle);
  const hasTitle = p.match.window_title.length > 0;
  return apps.find(
    (a) =>
      a.match.window_title.length > 0 === hasTitle &&
      (a.match.windows_exe.some((e) => exes.has(e.toLowerCase())) || a.match.macos_bundle.some((b) => bundles.has(b))),
  );
}

export default function App() {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [path, setPath] = useState("");
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [sel, setSel] = useState<number>(DEFAULT);
  const [navTab, setNavTab] = useState<NavTab>("active");
  const [navQ, setNavQ] = useState("");
  const [save, setSave] = useState<SaveState>({ kind: "idle" });
  const [engine, setEngine] = useState<{ connected: boolean; profile?: string; paused?: boolean; version?: string }>({ connected: false });
  const [last, setLast] = useState<{ event: string; profile: string; action: string; at: number } | null>(null);
  const [picker, setPicker] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [learned, setLearned] = useState<Learned | null>(null);
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
    api.appCatalog().then(setApps).catch(() => setApps([]));
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
          case "learned":
            setLearned((l) => ({ key: m.key!, mods: m.mods ?? "none", seq: (l?.seq ?? 0) + 1 }));
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
    if (!cfg || sel === INPUTS) return null;
    return sel === DEFAULT ? cfg.default_profile : cfg.profiles[sel] ?? null;
  }, [cfg, sel]);

  function setTransport(transport: Config["transport"]) {
    setCfg((c) => (c ? { ...c, transport } : c));
    setSave({ kind: "dirty" });
  }

  /** Rename an event everywhere: transport row and every profile's binding. */
  function renameEvent(from: string, to: string) {
    if (from === to) return;
    setCfg((c) => {
      if (!c) return c;
      const ren = (b: Record<string, Binding>) => {
        if (!(from in b)) return b;
        const { [from]: moved, ...rest } = b;
        return { ...rest, [to]: moved };
      };
      const transport = { ...c.transport };
      if (from in transport) {
        transport[to] = transport[from];
        delete transport[from];
      }
      return {
        ...c,
        transport,
        default_profile: { ...c.default_profile, bindings: ren(c.default_profile.bindings) },
        profiles: c.profiles.map((p) => ({ ...p, bindings: ren(p.bindings) })),
      };
    });
    setSave({ kind: "dirty" });
  }

  const events = useMemo(() => (cfg ? Object.keys(cfg.transport).sort((a, b) => eventSortKey(a) - eventSortKey(b)) : []), [cfg]);

  /**
   * The catalog entry that corresponds to the selected profile: the app's own
   * entry, or for the Default profile the running OS's system shortcuts.
   */
  const appEntry = useMemo(() => {
    if (!profile) return undefined;
    if (sel === DEFAULT) return apps.find((a) => a.kind === "system" && a.os?.includes(currentOs()));
    return catalogFor(apps, profile);
  }, [apps, profile, sel]);

  // ---- nav lists ---------------------------------------------------------
  const nav = useMemo(() => {
    if (!cfg) return { active: [] as number[], disabled: [] as number[], available: [] as AppEntry[] };
    const q = navQ.trim().toLowerCase();
    const hit = (t: string) => !q || t.includes(q);
    const active: number[] = [];
    const disabled: number[] = [];
    cfg.profiles.forEach((p, i) => {
      if (!hit(matchText(p))) return;
      (isEnabled(p) ? active : disabled).push(i);
    });
    const known = new Set(cfg.profiles.map((p) => catalogFor(apps, p)?.id).filter(Boolean));
    // System entries are not profiles; they feed the Default profile's picker.
    const os = currentOs();
    const available = apps.filter((a) => a.kind !== "system" && entryOnOs(a, os) && !known.has(a.id)).filter((a) => hit(matchText(a)));
    return { active, disabled, available };
  }, [cfg, apps, navQ]);

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
        // An edit made while this save was in flight has already set "dirty"
        // again; leave that alone so it gets its own save.
        setSave((s) => (s.kind === "saving" ? { kind: "saved" } : s));
      } catch (e) {
        setSave({ kind: "error", msg: String(e) });
      }
    }, 600);
  }, [cfg, save.kind]);

  function setBinding(ev: string, action: Action | null, name?: string) {
    update((p) => {
      const bindings = { ...p.bindings };
      if (action === null) delete bindings[ev];
      else {
        const b: Binding = { action, accel: bindings[ev]?.accel ?? "none" };
        if (name) b.name = name;
        bindings[ev] = b;
      }
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
    setNavTab("active");
    setSave({ kind: "dirty" });
    setAdding(false);
  }

  /** Star an app from the catalog: create its profile with the bundled defaults. */
  function activateApp(a: AppEntry) {
    addProfile({ name: a.name, enabled: true, match: { ...a.match }, bindings: { ...a.defaults } });
  }

  function setEnabled(i: number, enabled: boolean) {
    update((p) => ({ ...p, enabled }), i);
    if (enabled) setSel(i);
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
  if (!cfg || (sel !== INPUTS && !profile)) return <div style={{ padding: 30 }} className="muted">Loading…</div>;

  const showInputs = sel === INPUTS;
  const prof = (profile ?? cfg.default_profile) as Profile;
  const isDefault = sel === DEFAULT;
  const liveName = engine.profile;
  const selectedDisabled = !isDefault && !showInputs && !isEnabled(prof);

  const ProfileRow = ({ i }: { i: number }) => {
    const p = cfg.profiles[i];
    const on = isEnabled(p);
    return (
      <div className={"profile-item " + (sel === i ? "active " : "") + (liveName === p.name ? "live " : "") + (on ? "" : "dim")} onClick={() => setSel(i)}>
        <button
          className={"star " + (on ? "on" : "")}
          title={on ? "Disable this profile (keeps its mappings)" : "Enable this profile"}
          onClick={(e) => {
            e.stopPropagation();
            setEnabled(i, !on);
          }}
        >
          {on ? "★" : "☆"}
        </button>
        <span className="name">{p.name}</span>
        {p.match.window_title.length > 0 && <span className="badge">site</span>}
        {!on && Object.keys(p.bindings).length > 0 && <span className="badge">customized</span>}
      </div>
    );
  };

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <span className={"dot " + (engine.connected ? "on" : "")} title={engine.connected ? "Engine connected" : "Engine not running"} />
          Create Companion
        </div>
        <div className="nav-tools">
          <input placeholder="Search apps and sites…" value={navQ} onChange={(e) => setNavQ(e.target.value)} />
          <div className="seg">
            <button className={navTab === "active" ? "on" : ""} onClick={() => setNavTab("active")}>
              Active
            </button>
            <button className={navTab === "available" ? "on" : ""} onClick={() => setNavTab("available")}>
              Available{nav.available.length + nav.disabled.length > 0 ? ` (${nav.available.length + nav.disabled.length})` : ""}
            </button>
          </div>
        </div>
        <div className="profiles">
          {navTab === "active" && (
            <>
              <div className={"profile-item " + (isDefault ? "active " : "") + (liveName === cfg.default_profile.name ? "live" : "")} onClick={() => setSel(DEFAULT)}>
                <span className="star on" title="Always active" style={{ cursor: "default" }}>
                  ★
                </span>
                <span className="name">{cfg.default_profile.name}</span>
                <span className="badge">fallback</span>
              </div>
              {nav.active.map((i) => (
                <ProfileRow key={i} i={i} />
              ))}
              {nav.active.length === 0 && navQ && <div className="nav-empty">No active profile matches "{navQ}".</div>}
            </>
          )}
          {navTab === "available" && (
            <>
              {nav.disabled.map((i) => (
                <ProfileRow key={i} i={i} />
              ))}
              {nav.available.map((a) => (
                <div key={a.id} className="profile-item dim" onClick={() => activateApp(a)} title="Click the star to activate with its default mappings">
                  <button
                    className="star"
                    onClick={(e) => {
                      e.stopPropagation();
                      activateApp(a);
                    }}
                  >
                    ☆
                  </button>
                  <span className="name">{a.name}</span>
                  <span className="badge">{a.kind === "site" ? "site" : `${a.actions.length} shortcuts`}</span>
                </div>
              ))}
              {nav.available.length + nav.disabled.length === 0 && <div className="nav-empty">{navQ ? `Nothing matches "${navQ}".` : "Everything in the catalog is active."}</div>}
            </>
          )}
        </div>
        <div className="nav-add">
          <button onClick={() => setAdding(true)}>＋ Add application</button>
          <button className={showInputs ? "on" : ""} onClick={() => setSel(INPUTS)} title="Which key each module gesture sends">
            Inputs
          </button>
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
        {showInputs ? (
          <>
            <div className="topbar">
              <h1>Inputs</h1>
              <span className={"save-state " + (save.kind === "applied" ? "applied" : save.kind === "error" ? "error" : "")}>
                {save.kind === "idle" && "Changes save automatically"}
                {save.kind === "dirty" && "Editing…"}
                {save.kind === "saving" && "Saving…"}
                {save.kind === "saved" && (engine.connected ? "Saved · waiting for engine…" : "Saved")}
                {save.kind === "applied" && "Saved · applied by engine ✓"}
                {save.kind === "error" && `Error: ${save.msg}`}
              </span>
            </div>
            <div className="content">
              <Inputs transport={cfg.transport} onChange={setTransport} onRename={renameEvent} engineConnected={engine.connected} learned={learned} />
            </div>
          </>
        ) : (
          <>
        <div className="topbar">
          {isDefault ? (
            <h1>{prof.name}</h1>
          ) : (
            <input className="title" value={prof.name} onChange={(e) => update((p) => ({ ...p, name: e.target.value }))} />
          )}
          {selectedDisabled && (
            <button className="primary" onClick={() => setEnabled(sel, true)}>
              ★ Enable
            </button>
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
            <button onClick={() => removeProfile(sel)} title="Delete this profile and its mappings">
              Remove
            </button>
          )}
        </div>

        <div className="content">
          <div className={"detect " + (last ? "" : "idle")}>
            {last ? (
              <>
                <span className="muted">Detected</span>
                <span className="big">{eventLabel(last.event).join(" / ")}</span>
                <span className="muted">
                  in {last.profile} → {last.action}
                </span>
                <span style={{ flex: 1 }} />
                <button onClick={() => setPicker(last.event)}>Change for {prof.name}</button>
              </>
            ) : (
              <>
                <span>Turn the dial or swipe the Tune to detect an input.</span>
                {!engine.connected && <span>· Engine not running: start naya-companion to see live events.</span>}
              </>
            )}
          </div>

          {selectedDisabled && (
            <div className="detect idle">
              This profile is disabled: its mappings are kept but never used. Star it to enable.
            </div>
          )}

          {!isDefault && (
            <div className="card">
              <h2>Matches</h2>
              <div className="body" style={{ display: "grid", gap: 10 }}>
                <ChipEditor label="Windows executable" placeholder="app.exe" values={prof.match.windows_exe} onChange={(v) => editMatch("windows_exe", v)} mono />
                <ChipEditor label="Window title contains" placeholder="YouTube" values={prof.match.window_title} onChange={(v) => editMatch("window_title", v)} />
                <ChipEditor label="macOS bundle id" placeholder="com.example.App" values={prof.match.macos_bundle} onChange={(v) => editMatch("macos_bundle", v)} mono />
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
            <div className="tablewrap">
            <table className="map">
              <thead>
                <tr>
                  <th style={{ width: "20%" }}>Input</th>
                  <th>Action</th>
                  <th style={{ width: 130 }}>Keys</th>
                  <th style={{ width: 110 }}>Acceleration</th>
                  <th style={{ width: 90 }}></th>
                </tr>
              </thead>
              <tbody>
                {events.map((ev) => {
                  const [mod, gesture] = eventLabel(ev);
                  const b = prof.bindings[ev];
                  const inherited = !b && !isDefault ? cfg.default_profile.bindings[ev] : undefined;
                  const shown = b ?? inherited;
                  const repeatable = b && (b.action.type === "keys" || b.action.type === "media" || b.action.type === "scroll");
                  const actionName = shown ? shown.name ?? (shown.action.type === "keys" ? "Custom shortcut" : describeAction(shown.action)) : "Not bound";
                  return (
                    <tr key={ev} className={flash === ev ? "flash" : ""}>
                      <td className="ev">
                        <div>{gesture}</div>
                        <div className="module">
                          {mod} · <span className="mono">{transportLabel(cfg.transport[ev])}</span>
                        </div>
                      </td>
                      <td className="action" onClick={() => setPicker(ev)}>
                        {b ? (
                          <span className={shown?.name ? "" : "name-muted"}>{actionName}</span>
                        ) : (
                          <span className="inherit">{inherited ? `Default: ${actionName}` : "Not bound"}</span>
                        )}
                      </td>
                      <td className="keys">
                        {!shown ? (
                          <span className="muted">—</span>
                        ) : shown.action.type === "keys" ? (
                          <kbd>{shown.action.chord}</kbd>
                        ) : shown.action.type === "sequence" ? (
                          <kbd>{shown.action.chords.join(" , ")}</kbd>
                        ) : (
                          <span className="muted">{shown.action.type === "media" ? "media key" : shown.action.type === "scroll" ? `wheel ${shown.action.direction}` : shown.action.type}</span>
                        )}
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
          </div>

          <div className="muted" style={{ fontSize: 12 }}>
            Config file: <span className="mono">{path}</span>. Which key each gesture sends is set under <b>Inputs</b>.
          </div>
        </div>
          </>
        )}
      </main>

      {picker && (
        <ActionPicker
          event={picker}
          current={prof.bindings[picker]}
          catalog={catalog}
          appName={appEntry?.name}
          appActions={appEntry?.actions ?? []}
          os={currentOs()}
          onPick={(a, name) => setBinding(picker, a, name)}
          onClose={() => setPicker(null)}
        />
      )}
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
