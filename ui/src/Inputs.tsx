import { useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import {
  buildMods,
  eventSortKey,
  fingerOptions,
  fingersLabel,
  FUNCTION_KEYS,
  gestureLabel,
  gesturesFor,
  makeEvent,
  MODULES,
  moduleLabel,
  nayaBehavior,
  parseEvent,
  parseMods,
  takesFingers,
  TRANSPORT_MODS,
  transportLabel,
  type Mods,
  type TransportCode,
} from "./types";
import type { GestureId, ModuleId } from "./events";

export interface Learned {
  key: string;
  mods: Mods;
  seq: number;
}

const MOD_BUTTON: Record<string, string> = { ctrl: "Ctrl", shift: "Shift", alt: "Alt", cmd: "Cmd / Win", fn: "Fn (macOS)" };
/** Naya keycode tokens for the modifiers a module can send. */
const NAYA_MOD: Record<string, string> = { ctrl: "LCTRL", shift: "LSHIFT", alt: "LALT", cmd: "LGUI" };
/** macOS has virtual key codes for F13–F20 only. */
const MAC_OK = new Set(["F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20"]);

/** Naya-style chord token, the vocabulary OpenFlow's encoder speaks. */
function nayaChord(t: TransportCode): string {
  const toks = [...parseMods(t.mods)].filter((m) => NAYA_MOD[m]).map((m) => NAYA_MOD[m]);
  return [...toks, t.key].join(" + ");
}

/** An OpenFlow module-profile action for one transport code. */
function nayaAction(t: TransportCode): { actionType: string; actionCode: string } {
  return parseMods(t.mods).size === 0 ? { actionType: "key", actionCode: t.key } : { actionType: "shortcut_alias", actionCode: nayaChord(t) };
}

function KeySelect({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <select className="mono" value={value} onChange={(e) => onChange(e.target.value)} title="F21–F24 do not exist on macOS">
      {FUNCTION_KEYS.map((k) => (
        <option key={k} value={k}>
          {k}
          {MAC_OK.has(k) ? "" : " (Windows only)"}
        </option>
      ))}
    </select>
  );
}

/** Toggle buttons for the namespace modifiers a module can hold with its key. */
function ModToggles({ value, onChange }: { value: Mods | undefined; onChange: (v: Mods) => void }) {
  const set = parseMods(value);
  return (
    <div className="modtoggles">
      {TRANSPORT_MODS.map((m) => (
        <button
          key={m}
          type="button"
          className={set.has(m) ? "on" : ""}
          onClick={() => {
            const next = new Set(set);
            if (next.has(m)) next.delete(m);
            else next.add(m);
            onChange(buildMods(next));
          }}
        >
          {MOD_BUTTON[m]}
        </button>
      ))}
    </div>
  );
}

function FingerSelect({ gesture, value, onChange }: { gesture: GestureId; value: number | null; onChange: (v: number | null) => void }) {
  if (!takesFingers(gesture)) return <span className="muted">dial</span>;
  const opts = fingerOptions(gesture);
  return (
    <select value={value ?? ""} onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))} title="How many fingers the module was flashed for. Blank = any count (needed for export).">
      <option value="">any fingers</option>
      {opts.map((n) => (
        <option key={n} value={n}>
          {fingersLabel(n)}
        </option>
      ))}
    </select>
  );
}

export function Inputs(props: {
  transport: Record<string, TransportCode>;
  onChange: (t: Record<string, TransportCode>) => void;
  onRename: (from: string, to: string) => void;
  engineConnected: boolean;
  learned: Learned | null;
}) {
  const events = useMemo(() => Object.keys(props.transport).sort((a, b) => eventSortKey(a) - eventSortKey(b)), [props.transport]);
  const [armed, setArmedState] = useState<string | null>(null); // event being learned, or "new"
  const armedRef = useRef<string | null>(null);
  const setArmed = (v: string | null) => {
    armedRef.current = v;
    setArmedState(v);
  };
  const [newModule, setNewModule] = useState<ModuleId>("TUNE");
  const [newGesture, setNewGesture] = useState<GestureId>("TAP");
  const [newFingers, setNewFingers] = useState<number | null>(1);
  const [newKey, setNewKey] = useState<string>("F13");
  const [newMods, setNewMods] = useState<Mods>("none");
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const codeKey = (c: TransportCode) => `${buildMods(parseMods(c.mods))}+${c.key}`;
  const used = useMemo(() => {
    const m = new Map<string, string>();
    for (const ev of events) m.set(codeKey(props.transport[ev]), ev);
    return m;
  }, [events, props.transport]);

  const describe = (ev: string) => {
    const p = parseEvent(ev);
    return p ? `${moduleLabel(p.module)} / ${gestureLabel(p.gesture)}${p.fingers ? ` (${fingersLabel(p.fingers)})` : ""}` : ev;
  };

  // A learned key arrives: fill the row that was armed when Learn was clicked.
  useEffect(() => {
    const target = armedRef.current;
    if (!props.learned || !target) return;
    const code: TransportCode = { key: props.learned.key, mods: props.learned.mods };
    if (target === "new") {
      setNewKey(code.key);
      setNewMods(code.mods ?? "none");
      setMsg({ text: `Learned ${transportLabel(code)} — press Add to keep it`, ok: true });
    } else {
      setCode(target, code);
      setMsg({ text: `Learned ${transportLabel(code)} for ${describe(target)}`, ok: true });
    }
    setArmed(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.learned?.seq]);

  function setCode(ev: string, code: TransportCode) {
    const owner = used.get(codeKey(code));
    if (owner && owner !== ev) {
      setMsg({ text: `${transportLabel(code)} is already used by ${describe(owner)}.`, ok: false });
      return;
    }
    setMsg(null);
    props.onChange({ ...props.transport, [ev]: code });
  }

  function setFingers(ev: string, fingers: number | null) {
    const p = parseEvent(ev);
    if (!p) return;
    const to = makeEvent(p.module, p.gesture, fingers);
    if (to !== ev && props.transport[to]) {
      setMsg({ text: `${describe(to)} already exists.`, ok: false });
      return;
    }
    setMsg(null);
    props.onRename(ev, to);
  }

  function remove(ev: string) {
    const t = { ...props.transport };
    delete t[ev];
    props.onChange(t);
  }

  function add() {
    const ev = makeEvent(newModule, newGesture, newFingers);
    if (props.transport[ev]) {
      setMsg({ text: `${describe(ev)} already has a key. Edit it in the list.`, ok: false });
      return;
    }
    const code: TransportCode = { key: newKey, mods: newMods };
    if (used.has(codeKey(code))) {
      setMsg({ text: `${transportLabel(code)} is already used by ${describe(used.get(codeKey(code))!)}.`, ok: false });
      return;
    }
    setMsg(null);
    props.onChange({ ...props.transport, [ev]: code });
  }

  async function toggleLearn(target: string) {
    if (armed === target) {
      setArmed(null);
      await api.engineSend({ cmd: "learn", on: false }).catch(() => {});
      return;
    }
    try {
      await api.engineSend({ cmd: "learn", on: true });
      setArmed(target);
      setMsg(null);
    } catch (e) {
      setMsg({ text: `Learn needs the engine running: ${e}`, ok: false });
    }
  }

  /**
   * One OpenFlow module profile per module (`openflow.module-profile` v1):
   * discrete gestures become `key` / `shortcut_alias` actions; the dial and
   * scroll axes become a `value` pair with `split` halves.
   */
  async function exportProfiles() {
    const dir = await open({ directory: true, multiple: false, title: "Folder for the module profile files" });
    if (typeof dir !== "string") return;
    const written: string[] = [];
    const skipped: string[] = [];
    for (const module of MODULES) {
      const evs = events.filter((ev) => parseEvent(ev)?.module === module);
      if (evs.length === 0) continue;
      const bindings: Record<string, { actionType: string; actionCode: string; split?: Record<string, { actionType: string; actionCode: string }> }> = {};
      for (const ev of evs) {
        const nb = nayaBehavior(ev);
        if (!nb) {
          skipped.push(`${describe(ev)}: set a finger count first`);
          continue;
        }
        if (parseMods(props.transport[ev].mods).has("fn")) skipped.push(`${describe(ev)}: Fn has no keycode in the module profile format; exported without it`);
        const action = nayaAction(props.transport[ev]);
        if (nb.half) {
          const pair = (bindings[nb.behavior] ??= { actionType: "value", actionCode: "", split: {} });
          pair.split![nb.half] = action;
          const minus = pair.split!["-"]?.actionCode ?? "";
          const plus = pair.split!["+"]?.actionCode ?? "";
          pair.actionCode = `${minus} - ${plus}`;
        } else {
          bindings[nb.behavior] = action;
        }
      }
      for (const [b, v] of Object.entries(bindings)) {
        if (v.split && (!v.split["-"] || !v.split["+"])) skipped.push(`${b}: only one direction is bound (exported as is)`);
      }
      if (Object.keys(bindings).length === 0) continue;
      const doc = {
        format: "openflow.module-profile",
        version: 1,
        moduleType: module === "TUNE" ? "TUNE" : "TOUCH",
        name: `Create Companion ${moduleLabel(module)}`,
        bindings,
      };
      const file = `${dir.replace(/[\\/]+$/, "")}\\create-companion-${module.toLowerCase().replace("_", "-")}.json`;
      try {
        await api.writeTextFile(file, JSON.stringify(doc, null, 2) + "\n");
        written.push(file.split(/[\\/]/).pop()!);
      } catch (e) {
        skipped.push(`${moduleLabel(module)}: ${e}`);
      }
    }
    setMsg({
      text: `${written.length ? `Exported ${written.join(", ")} to ${dir}` : "Nothing exported"}${skipped.length ? ` · ${skipped.join("; ")}` : ""}`,
      ok: written.length > 0,
    });
  }

  return (
    <>
      <div className="detect idle">
        <span>
          Each gesture is flashed on the module (with OpenFlow) to send one key. List the same keys here; only these are intercepted, every other key passes
          through. Hold modifiers on the module to keep two modules apart, e.g. Tune on plain keys, Left Touch on Shift, Right Touch on Cmd. On macOS use
          F13–F20 only (F21–F24 do not exist there). Set the finger count to export a module profile.
        </span>
      </div>

      {msg && (
        <div className={"detect " + (msg.ok ? "" : "idle")}>
          <span className={msg.ok ? "big" : "error"}>{msg.text}</span>
        </div>
      )}
      {armed && (
        <div className="detect">
          <span className="big">Listening…</span>
          <span className="muted">Perform the gesture on the module now. Any F13–F24 key it sends will be captured (and passed through if it is not yet an input).</span>
        </div>
      )}

      <div className="card">
        <h2>
          Inputs
          <span className="spacer" />
          <button className="ghost" onClick={exportProfiles} title="Write one openflow.module-profile JSON per module, ready for OpenFlow's Import">
            Export for OpenFlow…
          </button>
        </h2>
        <div className="tablewrap">
          <table className="map">
            <thead>
              <tr>
                <th style={{ width: "22%" }}>Gesture</th>
                <th style={{ width: 120 }}>Fingers</th>
                <th>Sends</th>
                <th style={{ width: 150 }}></th>
              </tr>
            </thead>
            <tbody>
              {events.map((ev) => {
                const t = props.transport[ev];
                const p = parseEvent(ev);
                if (!p) return null;
                return (
                  <tr key={ev} className={armed === ev ? "flash" : ""}>
                    <td className="ev">
                      <div>{gestureLabel(p.gesture)}</div>
                      <div className="module">{moduleLabel(p.module)}</div>
                    </td>
                    <td>
                      <FingerSelect gesture={p.gesture} value={p.fingers} onChange={(n) => setFingers(ev, n)} />
                    </td>
                    <td>
                      <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                        <ModToggles value={t.mods} onChange={(mods) => setCode(ev, { ...t, mods })} />
                        <KeySelect value={t.key} onChange={(key) => setCode(ev, { ...t, key })} />
                        <kbd>{transportLabel(t)}</kbd>
                      </div>
                    </td>
                    <td className="actions">
                      <button className={"ghost " + (armed === ev ? "primary" : "")} disabled={!props.engineConnected && armed !== ev} onClick={() => toggleLearn(ev)} title="Press the gesture on the module; the key it sends fills this row">
                        {armed === ev ? "Waiting…" : "Learn"}
                      </button>
                      <button className="ghost" title="Remove this input" onClick={() => remove(ev)}>
                        ✕
                      </button>
                    </td>
                  </tr>
                );
              })}
              <tr>
                <td className="ev">
                  <div style={{ display: "flex", gap: 6 }}>
                    <select
                      value={newModule}
                      onChange={(e) => {
                        const m = e.target.value as ModuleId;
                        setNewModule(m);
                        if (!gesturesFor(m).includes(newGesture)) setNewGesture("TAP");
                      }}
                    >
                      {MODULES.map((m) => (
                        <option key={m} value={m}>
                          {moduleLabel(m)}
                        </option>
                      ))}
                    </select>
                    <select
                      value={newGesture}
                      onChange={(e) => {
                        const g = e.target.value as GestureId;
                        setNewGesture(g);
                        const opts = fingerOptions(g);
                        setNewFingers(opts.length === 0 ? null : newFingers && opts.includes(newFingers) ? newFingers : opts[0]);
                      }}
                    >
                      {gesturesFor(newModule).map((g) => (
                        <option key={g} value={g}>
                          {gestureLabel(g)}
                        </option>
                      ))}
                    </select>
                  </div>
                </td>
                <td>
                  <FingerSelect gesture={newGesture} value={newFingers} onChange={setNewFingers} />
                </td>
                <td>
                  <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                    <ModToggles value={newMods} onChange={setNewMods} />
                    <KeySelect value={newKey} onChange={setNewKey} />
                    <kbd>{transportLabel({ key: newKey, mods: newMods })}</kbd>
                  </div>
                </td>
                <td className="actions">
                  <button className={"ghost " + (armed === "new" ? "primary" : "")} disabled={!props.engineConnected && armed !== "new"} onClick={() => toggleLearn("new")}>
                    {armed === "new" ? "Waiting…" : "Learn"}
                  </button>
                  <button className="primary" onClick={add} disabled={!!props.transport[makeEvent(newModule, newGesture, newFingers)]} title={props.transport[makeEvent(newModule, newGesture, newFingers)] ? "This gesture already has a key" : ""}>
                    Add
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}
