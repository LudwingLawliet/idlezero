import { useState } from "react";
import UnitPicker, { PickResult } from "./UnitPicker";
import { Labels, Kind } from "./labels";
import { Age, AppInfo, CivView, Entity, Goal } from "./types";

type AddKind = "age" | "villagers" | Kind;
const AGES: Age[] = ["Feudal", "Castle", "Imperial"];

export const emptyGoal = (): Goal => ({ id: "custom", age: null, villagers: null, units: [], techs: [], buildings: [], allow_tc_idle: false });

/** Objetivo como chips combinables: edad, aldeanos, unidades, tecnologías y edificios. */
export default function GoalBuilder({
  goal,
  onChange,
  info,
  labels,
  civ,
  lang,
}: {
  goal: Goal;
  onChange: (g: Goal) => void;
  info: AppInfo;
  labels: Labels;
  civ?: CivView;
  lang: string;
}) {
  const L = labels.ui;
  const [adding, setAdding] = useState<AddKind | null>(null);
  const [qty, setQty] = useState(10);

  const set = (patch: Partial<Goal>) => onChange({ ...goal, id: "custom", ...patch });
  const close = () => setAdding(null);
  const confirm = (age?: Age) => {
    if (adding === "age" && age) set({ age });
    if (adding === "villagers") set({ villagers: Math.max(1, qty) });
    close();
  };
  // Lo elegido en la carta, más las mejoras marcadas como tecnologías.
  const picked = (r: PickResult) => {
    const techs = [...goal.techs.filter((t) => !r.techs.includes(t) && t !== r.key), ...r.techs];
    if (adding === "unit") set({ units: [...goal.units.filter((u) => u.unit !== r.key), { unit: r.key, count: r.count }], techs });
    if (adding === "tech") set({ techs: [...techs, r.key] });
    if (adding === "building") set({ buildings: [...goal.buildings.filter((b) => b.building !== r.key), { building: r.key, count: r.count }] });
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

  const preset = info.presets.find((p) => p.id === goal.id);
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
        {[...new Set(info.presets.map((p) => p.category))].map((cat) => (
          <optgroup key={cat} label={info.presets.find((p) => p.category === cat)!.category_name}>
            {info.presets
              .filter((p) => p.category === cat)
              .map((p) => (
                <option key={p.id} value={p.id} title={p.description}>
                  {p.name}
                </option>
              ))}
          </optgroup>
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
        <span className="add-buttons">
          {(
            [
              ["unit", L.addUnit, "/img/unit/74.png"],
              ["building", L.addBuilding, "/img/building/82.png"],
              ["tech", L.addTech, "/img/tech/22.png"],
              ["age", L.addAge, "/img/age/Castle.png"],
              ["villagers", L.addVillagers, "/img/unit/83.png"],
            ] as [AddKind, string, string][]
          ).map(([k, text, icon]) => (
            <button key={k} className={`add-btn${adding === k ? " active" : ""}`} onClick={() => setAdding(adding === k ? null : k)}>
              <img src={icon} alt="" className="icon tiny" />+ {text}
            </button>
          ))}
        </span>
      </div>
      {preset && (
        <p className="preset-desc" title={L.strategyNote}>
          <strong>{preset.name}:</strong> {preset.description}
        </p>
      )}
      <label className="check">
        <input type="checkbox" checked={goal.allow_tc_idle} onChange={(e) => set({ allow_tc_idle: e.target.checked })} />
        {L.allowIdle}
      </label>

      {(adding === "age" || adding === "villagers") && (
        <div className="popover">
          {adding === "age" &&
            AGES.map((a) => (
              <button key={a} className="pick" onClick={() => confirm(a)}>
                <img src={`/img/age/${a}.png`} alt="" className="icon small" /> {labels.t(`ages.${a}`)}
              </button>
            ))}
          {adding === "villagers" && (
            <label className="qty">
              {L.quantity}
              <input autoFocus type="number" min={1} max={200} value={qty} onChange={(e) => setQty(Number(e.target.value))} />
            </label>
          )}
          <div className="popover-actions">
            <button onClick={close}>{L.cancel}</button>
            {adding === "villagers" && (
              <button className="primary" onClick={() => confirm()}>
                {L.ok}
              </button>
            )}
          </div>
        </div>
      )}
      {(adding === "unit" || adding === "tech" || adding === "building") && (
        <UnitPicker
          kind={adding}
          list={adding === "unit" ? info.catalog.units : adding === "tech" ? info.catalog.techs : info.catalog.buildings}
          civ={civ}
          lang={lang}
          labels={labels}
          onAdd={picked}
          onClose={close}
        />
      )}
    </div>
  );
}
