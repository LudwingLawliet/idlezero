import { useMemo, useState } from "react";
import { Labels, Kind } from "./labels";
import { Age, AppInfo, CivView, Entity, Goal } from "./types";

type AddKind = "age" | "villagers" | Kind;
const AGES: Age[] = ["Feudal", "Castle", "Imperial"];

export const emptyGoal = (): Goal => ({ id: "custom", age: null, villagers: null, units: [], techs: [], buildings: [], allow_tc_idle: false });

/** Objetivo como chips combinables: edad, aldeanos, unidades, tecnologías y edificios. */
export default function GoalBuilder({ goal, onChange, info, labels, civ }: { goal: Goal; onChange: (g: Goal) => void; info: AppInfo; labels: Labels; civ?: CivView }) {
  const L = labels.ui;
  const [adding, setAdding] = useState<AddKind | null>(null);
  const [query, setQuery] = useState("");
  const [pick, setPick] = useState<Entity | null>(null);
  const [qty, setQty] = useState(10);

  const options = useMemo(() => {
    if (adding !== "unit" && adding !== "tech" && adding !== "building") return [];
    const list = adding === "unit" ? info.catalog.units : adding === "tech" ? info.catalog.techs : info.catalog.buildings;
    // Solo lo que tiene el árbol de la civilización elegida.
    const allowed = civ ? (adding === "unit" ? civ.units : adding === "tech" ? civ.techs : civ.buildings) : null;
    const q = query.trim().toLowerCase();
    return list
      .filter((e) => !allowed || allowed.includes(e.id) || (adding === "tech" && e.name.includes("(")))
      .filter((e) => !q || e.name.toLowerCase().includes(q))
      .slice(0, 40);
  }, [adding, query, info, civ]);

  const set = (patch: Partial<Goal>) => onChange({ ...goal, id: "custom", ...patch });
  const close = () => {
    setAdding(null);
    setQuery("");
    setPick(null);
  };
  const confirm = (age?: Age) => {
    if (adding === "age" && age) set({ age });
    if (adding === "villagers") set({ villagers: Math.max(1, qty) });
    if (pick && adding === "unit") set({ units: [...goal.units.filter((u) => u.unit !== pick.key), { unit: pick.key, count: Math.max(1, qty) }] });
    if (pick && adding === "tech") set({ techs: [...goal.techs.filter((t) => t !== pick.key), pick.key] });
    if (pick && adding === "building")
      set({ buildings: [...goal.buildings.filter((b) => b.building !== pick.key), { building: pick.key, count: Math.max(1, qty) }] });
    close();
  };

  const chips: { key: string; icon: string; text: string; remove: () => void; missing?: boolean }[] = [];
  const idOf = (list: Entity[], key: string) => list.find((e) => e.key === key || String(e.id) === key)?.id;
  if (goal.age) chips.push({ key: "age", icon: `/img/age/${goal.age}.png`, text: labels.t(`ages.${goal.age}`), remove: () => set({ age: null }) });
  if (goal.villagers)
    chips.push({ key: "vil", icon: "/img/unit/83.png", text: `${goal.villagers} ${L.villagers.toLowerCase()}`, remove: () => set({ villagers: null }) });
  for (const u of goal.units)
    chips.push({
      key: `u${u.unit}`,
      icon: labels.icon("unit", u.unit),
      text: `${u.count} × ${labels.name("unit", u.unit)}`,
      remove: () => set({ units: goal.units.filter((x) => x !== u) }),
      missing: !!civ && !civ.units.includes(idOf(info.catalog.units, u.unit) ?? -1),
    });
  for (const t of goal.techs)
    chips.push({ key: `t${t}`, icon: labels.icon("tech", t), text: labels.name("tech", t), remove: () => set({ techs: goal.techs.filter((x) => x !== t) }) });
  for (const b of goal.buildings)
    chips.push({
      key: `b${b.building}`,
      icon: labels.icon("building", b.building),
      text: `${b.count} × ${labels.name("building", b.building)}`,
      remove: () => set({ buildings: goal.buildings.filter((x) => x !== b) }),
    });

  const needsQty = adding === "villagers" || adding === "unit" || adding === "building";
  return (
    <div className="goal-builder">
      <select
        value=""
        onChange={(e) => {
          const p = info.presets.find((x) => x.id === e.target.value);
          if (p) onChange({ ...emptyGoal(), ...p.goal });
        }}
      >
        <option value="">{L.presets}</option>
        {info.presets.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      <div className="chips">
        {chips.map((c) => (
          <span className={`chip${c.missing ? " missing" : ""}`} key={c.key} title={c.missing ? `${c.text} ${L.notInCiv}` : undefined}>
            <img src={c.icon} alt="" className="icon small" />
            {c.text}
            <button onClick={c.remove} aria-label="×">
              ×
            </button>
          </span>
        ))}
        <span className="add">
          <select value={adding ?? ""} onChange={(e) => setAdding((e.target.value || null) as AddKind | null)}>
            <option value="">{L.add}</option>
            <option value="age">{L.addAge}</option>
            <option value="villagers">{L.addVillagers}</option>
            <option value="unit">{L.addUnit}</option>
            <option value="tech">{L.addTech}</option>
            <option value="building">{L.addBuilding}</option>
          </select>
        </span>
      </div>
      <label className="check">
        <input type="checkbox" checked={goal.allow_tc_idle} onChange={(e) => set({ allow_tc_idle: e.target.checked })} />
        {L.allowIdle}
      </label>

      {adding && (
        <div className="popover">
          {adding === "age" &&
            AGES.map((a) => (
              <button key={a} className="pick" onClick={() => confirm(a)}>
                <img src={`/img/age/${a}.png`} alt="" className="icon small" /> {labels.t(`ages.${a}`)}
              </button>
            ))}
          {(adding === "unit" || adding === "tech" || adding === "building") && (
            <>
              <input autoFocus placeholder={L.search} value={query} onChange={(e) => setQuery(e.target.value)} />
              <div className="pick-list">
                {options.map((e) => (
                  <button key={e.key} className={`pick${pick?.key === e.key ? " active" : ""}`} onClick={() => setPick(e)}>
                    <img src={`/img/${adding}/${e.id}.png`} alt="" className="icon small" />
                    {e.name}
                    <span className="muted tiny"> · {labels.t(`ages.${e.age}`)}</span>
                  </button>
                ))}
              </div>
            </>
          )}
          {needsQty && (
            <label className="qty">
              {L.quantity}
              <input type="number" min={1} max={200} value={qty} onChange={(e) => setQty(Number(e.target.value))} />
            </label>
          )}
          <div className="popover-actions">
            <button onClick={close}>{L.cancel}</button>
            {adding !== "age" && (
              <button className="primary" disabled={adding !== "villagers" && !pick} onClick={() => confirm()}>
                {L.ok}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
