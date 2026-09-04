import { useEffect, useMemo, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import {
  eventSortKey,
  FUNCTION_KEYS,
  gestureLabel,
  gesturesFor,
  MODS,
  MODULES,
  moduleLabel,
  transportLabel,
  type Mods,
  type TransportCode,
} from "./types";

export interface Learned {
  key: string;
  mods: Mods;
  seq: number;
}

const MOD_LABEL: Record<Mods, string> = {
  none: "no modifier",
  shift: "Shift +",
  ctrl: "Ctrl +",
  alt: "Alt +",
  ctrl_shift: "Ctrl + Shift +",
};

/** Naya-style chord token, the vocabulary OpenFlow's encoder speaks. */
function nayaChord(t: TransportCode): string {
  const m: Record<Mods, string[]> = { none: [], shift: ["LSHIFT"], ctrl: ["LCTRL"], alt: ["LALT"], ctrl_shift: ["LCTRL", "LSHIFT"] };
  return [...m[t.mods ?? "none"], t.key].join(" + ");
}

function hidUsage(key: string): string {
  const n = Number(key.slice(1)); // F13 -> 0x68
  return "0x" + (0x68 + (n - 13)).toString(16);
}

function KeySelect({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <select className="mono" value={value} onChange={(e) => onChange(e.target.value)}>
      {FUNCTION_KEYS.map((k) => (
        <option key={k} value={k}>
          {k}
        </option>
      ))}
    </select>
  );
}

function ModSelect({ value, onChange }: { value: Mods; onChange: (v: Mods) => void }) {
  return (
    <select value={value} onChange={(e) => onChange(e.target.value as Mods)}>
      {MODS.map((m) => (
        <option key={m} value={m}>
          {MOD_LABEL[m]}
        </option>
      ))}
    </select>
  );
}

export function Inputs(props: {
  transport: Record<string, TransportCode>;
  onChange: (t: Record<string, TransportCode>) => void;
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
  const [newModule, setNewModule] = useState<string>("TUNE");
  const [newGesture, setNewGesture] = useState<string>("TAP");
  const [newKey, setNewKey] = useState<string>("F13");
  const [newMods, setNewMods] = useState<Mods>("none");
  const [msg, setMsg] = useState<string | null>(null);

  const used = useMemo(() => {
    const m = new Map<string, string>();
    for (const ev of events) m.set(codeKey(props.transport[ev]), ev);
    return m;
  }, [events, props.transport]);

  // A learned key arrives: fill the row that was armed when Learn was clicked.
  useEffect(() => {
    const target = armedRef.current;
    if (!props.learned || !target) return;
    const code: TransportCode = { key: props.learned.key, mods: props.learned.mods };
    if (target === "new") {
      setNewKey(code.key);
      setNewMods(code.mods ?? "none");
    } else {
      setCode(target, code);
    }
    setArmed(null);
    setMsg(`Learned ${transportLabel(code)}${target === "new" ? " — press Add to keep it" : ""}`);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.learned?.seq]);

  function codeKey(c: TransportCode) {
    return `${c.mods ?? "none"}+${c.key}`;
  }

  function setCode(ev: string, code: TransportCode) {
    const owner = used.get(codeKey(code));
    if (owner && owner !== ev) {
      setMsg(`${transportLabel(code)} is already used by ${moduleLabel(owner.split("_")[0] === "TUNE" ? "TUNE" : owner.startsWith("LEFT") ? "LEFT_TOUCH" : "RIGHT_TOUCH")} / ${gestureLabel(owner.replace(/^(TUNE|LEFT_TOUCH|RIGHT_TOUCH)_/, ""))}.`);
      return;
    }
    setMsg(null);
    props.onChange({ ...props.transport, [ev]: code });
  }

  function remove(ev: string) {
    const t = { ...props.transport };
    delete t[ev];
    props.onChange(t);
  }

  function add() {
    const ev = `${newModule}_${newGesture}`;
    if (props.transport[ev]) {
      setMsg(`${moduleLabel(newModule)} / ${gestureLabel(newGesture)} already has a key. Edit it in the list.`);
      return;
    }
    const code: TransportCode = { key: newKey, mods: newMods };
    if (used.has(codeKey(code))) {
      setMsg(`${transportLabel(code)} is already used. Pick another key or modifier.`);
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
      setMsg(`Learn needs the engine running: ${e}`);
    }
  }

  async function exportJson() {
    const path = await save({ defaultPath: "create-companion-inputs.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    const modules = MODULES.map((m) => ({
      module: m,
      gestures: events
        .filter((ev) => ev.startsWith(m + "_"))
        .map((ev) => {
          const t = props.transport[ev];
          const mods: Record<Mods, string[]> = { none: [], shift: ["LSHIFT"], ctrl: ["LCTRL"], alt: ["LALT"], ctrl_shift: ["LCTRL", "LSHIFT"] };
          return {
            event: ev,
            gesture: ev.slice(m.length + 1),
            key: t.key,
            hid_usage: hidUsage(t.key),
            modifiers: mods[t.mods ?? "none"],
            chord: nayaChord(t),
          };
        }),
    })).filter((m) => m.gestures.length > 0);
    const doc = {
      format: "create-companion-inputs",
      version: 1,
      exported: new Date().toISOString(),
      note: "Flash each gesture on the module so it sends `chord`. Produced by Create Companion; importable into OpenFlow module profiles.",
      modules,
    };
    try {
      await api.writeTextFile(path, JSON.stringify(doc, null, 2) + "\n");
      setMsg(`Exported ${path}`);
    } catch (e) {
      setMsg(`Export failed: ${e}`);
    }
  }

  return (
    <>
      <div className="detect idle">
        <span>
          Each gesture is flashed on the module (with OpenFlow) to send one key. List the same keys here; only these are intercepted, every other key passes
          through. Use a modifier to keep two modules apart, e.g. Tune on plain keys, Left Touch on Shift.
        </span>
      </div>

      {msg && <div className={"detect " + (msg.startsWith("Learned") || msg.startsWith("Exported") ? "" : "idle")}><span className={msg.startsWith("Learned") || msg.startsWith("Exported") ? "big" : "error"}>{msg}</span></div>}
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
          <button className="ghost" onClick={exportJson} title="Save a JSON file with every gesture and the chord to flash on the module">
            Export for OpenFlow…
          </button>
        </h2>
        <div className="tablewrap">
        <table className="map">
          <thead>
            <tr>
              <th style={{ width: "30%" }}>Gesture</th>
              <th>Sends</th>
              <th style={{ width: 150 }}></th>
            </tr>
          </thead>
          <tbody>
            {events.map((ev) => {
              const t = props.transport[ev];
              const m = ev.startsWith("TUNE_") ? "TUNE" : ev.startsWith("LEFT_TOUCH_") ? "LEFT_TOUCH" : "RIGHT_TOUCH";
              const g = ev.slice(m.length + 1);
              return (
                <tr key={ev} className={armed === ev ? "flash" : ""}>
                  <td className="ev">
                    <div>{gestureLabel(g)}</div>
                    <div className="module">{moduleLabel(m)}</div>
                  </td>
                  <td>
                    <div className="inline" style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <ModSelect value={t.mods ?? "none"} onChange={(mods) => setCode(ev, { ...t, mods })} />
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
                      setNewModule(e.target.value);
                      if (!gesturesFor(e.target.value).includes(newGesture)) setNewGesture("TAP");
                    }}
                  >
                    {MODULES.map((m) => (
                      <option key={m} value={m}>
                        {moduleLabel(m)}
                      </option>
                    ))}
                  </select>
                  <select value={newGesture} onChange={(e) => setNewGesture(e.target.value)}>
                    {gesturesFor(newModule).map((g) => (
                      <option key={g} value={g} disabled={!!props.transport[`${newModule}_${g}`]}>
                        {gestureLabel(g)}
                      </option>
                    ))}
                  </select>
                </div>
              </td>
              <td>
                <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                  <ModSelect value={newMods} onChange={setNewMods} />
                  <KeySelect value={newKey} onChange={setNewKey} />
                  <kbd>{transportLabel({ key: newKey, mods: newMods })}</kbd>
                </div>
              </td>
              <td className="actions">
                <button className={"ghost " + (armed === "new" ? "primary" : "")} disabled={!props.engineConnected && armed !== "new"} onClick={() => toggleLearn("new")}>
                  {armed === "new" ? "Waiting…" : "Learn"}
                </button>
                <button className="primary" onClick={add}>
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
