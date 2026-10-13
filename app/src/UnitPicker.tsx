import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Labels, Kind } from "./labels";
import { Age, Cost, CivView, Entity, RESOURCES, UnitCard } from "./types";

const AGE_ORDER: Age[] = ["Dark", "Feudal", "Castle", "Imperial"];
const CATEGORIES = ["infantry", "archers", "cav_archers", "cavalry", "siege", "monks", "ships", "other"];
// Ícono representativo de cada categoría (ids de aoe2techtree).
const CATEGORY_ICON: Record<string, number> = {
  infantry: 74,
  archers: 4,
  cav_archers: 39,
  cavalry: 38,
  siege: 280,
  monks: 125,
  ships: 539,
  other: 83,
};

type UpgradeMode = "none" | "rec" | "all";

export interface PickResult {
  key: string;
  count: number;
  /** Tecnologías extra (mejoras marcadas en la carta). */
  techs: string[];
}

export function CostLine({ cost }: { cost: Cost }) {
  return (
    <span className="cost-line">
      {RESOURCES.filter((r) => cost[r] > 0).map((r) => (
        <span key={r}>
          <img src={`/img/resource/${r}.png`} alt={r} className="icon tiny" />
          {cost[r]}
        </span>
      ))}
    </span>
  );
}

/** Ventana grande para elegir unidad, edificio o tecnología, con su carta de presentación. */
export default function UnitPicker({
  kind,
  list,
  civ,
  lang,
  labels,
  onAdd,
  onClose,
}: {
  kind: Kind;
  list: Entity[];
  civ?: CivView;
  lang: string;
  labels: Labels;
  onAdd: (r: PickResult) => void;
  onClose: () => void;
}) {
  const L = labels.ui;
  const T = L as unknown as Record<string, string>;
  const [cat, setCat] = useState<string>(kind === "unit" ? "infantry" : "all");
  const [query, setQuery] = useState("");
  const [pick, setPick] = useState<Entity | null>(null);
  const [qty, setQty] = useState(kind === "building" ? 1 : 10);
  const [card, setCard] = useState<UnitCard | null>(null);
  const [mode, setMode] = useState<UpgradeMode>("rec");
  const [checked, setChecked] = useState<Set<number>>(new Set());
  const [details, setDetails] = useState(false);

  const allowed = civ ? (kind === "unit" ? civ.units : kind === "tech" ? civ.techs : civ.buildings) : null;
  const available = useMemo(
    () => list.filter((e) => !allowed || allowed.includes(e.id) || (kind === "tech" && e.name.includes("("))),
    [list, allowed, kind],
  );
  const counts = useMemo(() => {
    const m: Record<string, number> = {};
    for (const e of available) m[e.category ?? "other"] = (m[e.category ?? "other"] ?? 0) + 1;
    return m;
  }, [available]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return available
      .filter((e) => (q ? e.name.toLowerCase().includes(q) : kind !== "unit" || cat === "all" || (e.category ?? "other") === cat))
      .sort(
        (a, b) =>
          AGE_ORDER.indexOf(a.age) - AGE_ORDER.indexOf(b.age) || (a.line ?? a.id) - (b.line ?? b.id) || a.id - b.id,
      );
  }, [available, query, cat, kind]);

  // Carta de la unidad: estadísticas, counters y mejoras.
  useEffect(() => {
    setCard(null);
    if (kind !== "unit" || !pick) return;
    invoke<UnitCard>("unit_card", { id: pick.id, lang, civ: civ?.id ?? null }).then((c) => {
      setCard(c);
      setChecked(new Set(c.upgrades.filter((u) => u.recommended).map((u) => u.id)));
      setMode("rec");
    });
  }, [pick, kind, lang, civ]);

  const chooseMode = (m: UpgradeMode) => {
    setMode(m);
    if (!card) return;
    setChecked(new Set(m === "none" ? [] : card.upgrades.filter((u) => m === "all" || u.recommended).map((u) => u.id)));
  };
  const toggle = (id: number) => {
    const s = new Set(checked);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    setChecked(s);
  };

  const add = () => {
    if (!pick) return;
    const techs = card ? card.upgrades.filter((u) => checked.has(u.id)).map((u) => u.key) : [];
    onAdd({ key: pick.key, count: Math.max(1, qty), techs });
  };

  const title = kind === "unit" ? L.pickUnitTitle : kind === "building" ? L.pickBuildingTitle : L.pickTechTitle;
  const timeLabel = kind === "unit" ? L.trainTime : kind === "building" ? L.buildTime : L.researchTime;
  const fmt = (n: number) => (Number.isInteger(n) ? String(n) : n.toFixed(2).replace(/0+$/, ""));

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal unit-picker" onClick={(e) => e.stopPropagation()} role="dialog" aria-label={title}>
        <div className="civ-head">
          <h2>{title}</h2>
          <input autoFocus placeholder={L.search} value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
        {kind === "unit" && (
          <div className="cat-tabs" role="tablist">
            {CATEGORIES.filter((c) => counts[c]).map((c) => (
              <button key={c} className={cat === c && !query ? "active" : ""} onClick={() => (setCat(c), setQuery(""))}>
                <img src={`/img/unit/${CATEGORY_ICON[c]}.png`} alt="" />
                <span>{T[`cat_${c}`]}</span>
                <small className="muted">{counts[c]}</small>
              </button>
            ))}
            <button className={cat === "all" && !query ? "active" : ""} onClick={() => (setCat("all"), setQuery(""))}>
              <span className="cat-all">∗</span>
              <span>{L.catAll}</span>
            </button>
          </div>
        )}
        <div className="picker-body">
          <div className="picker-grid">
            {AGE_ORDER.map((age) => {
              const items = shown.filter((e) => e.age === age);
              if (!items.length) return null;
              return (
                <section key={age}>
                  <h4>
                    <img src={`/img/age/${age}.png`} alt="" className="icon tiny" /> {labels.t(`ages.${age}`)}
                  </h4>
                  <div className="picker-cells">
                    {items.map((e) => (
                      <button
                        key={e.key}
                        className={`picker-cell${pick?.key === e.key ? " active" : ""}`}
                        onClick={() => setPick(e)}
                        onDoubleClick={() => (setPick(e), kind !== "unit" && onAdd({ key: e.key, count: Math.max(1, qty), techs: [] }))}
                        title={e.name}
                      >
                        <img src={`/img/${kind}/${e.id}.png`} alt="" />
                        <span>{e.name}</span>
                      </button>
                    ))}
                  </div>
                </section>
              );
            })}
          </div>
          <div className="picker-card">
            {!pick ? (
              <p className="muted">{L.pickHint}</p>
            ) : (
              <>
                <div className="card-title">
                  <img src={`/img/${kind}/${pick.id}.png`} alt="" />
                  <div>
                    <h3>{pick.name}</h3>
                    <span className="muted">
                      <img src={`/img/age/${pick.age}.png`} alt="" className="icon tiny" /> {labels.t(`ages.${pick.age}`)}
                      {card?.produced_at && ` · ${L.producedAt}: ${card.produced_at}`}
                    </span>
                  </div>
                </div>
                <div className="card-stats">
                  <CostLine cost={pick.cost} />
                  <span>
                    ⏱ {timeLabel}: {pick.time} s
                  </span>
                  {pick.hp > 0 && (
                    <span>
                      ❤ {L.hp}: {pick.hp}
                    </span>
                  )}
                </div>
                {card && (
                  <>
                    <div className="counters">
                      <div>
                        <h5 className="good">{L.strongVs}</h5>
                        {card.strong_vs.length ? (
                          <ul>
                            {card.strong_vs.map((c) => (
                              <li key={c.class}>
                                {c.class} <strong>+{c.amount}</strong>
                              </li>
                            ))}
                          </ul>
                        ) : (
                          <p className="muted tiny">{L.noCounters}</p>
                        )}
                      </div>
                      <div>
                        <h5 className="bad">{L.weakVs}</h5>
                        <div className="ref-icons">
                          {card.weak_vs.map((u) => (
                            <span key={u.id} title={`${u.name} (+${u.bonus})`}>
                              <img src={`/img/unit/${u.id}.png`} alt={u.name} />
                            </span>
                          ))}
                        </div>
                      </div>
                    </div>
                    {card.line.length > 1 && (
                      <div className="unit-line">
                        <h5>{L.unitLine}</h5>
                        <div className="ref-icons">
                          {card.line.map((u, i) => (
                            <span key={u.id} className={u.id === card.id ? "current" : ""} title={u.name}>
                              {i > 0 && <i>→</i>}
                              <img src={`/img/unit/${u.id}.png`} alt={u.name} />
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                    {card.upgrades.length > 0 && (
                      <div className="upgrades">
                        <div className="upgrades-head">
                          <h5>{L.upgrades}</h5>
                          <div className="seg">
                            {(["none", "rec", "all"] as UpgradeMode[]).map((m) => (
                              <button key={m} className={mode === m ? "active" : ""} onClick={() => chooseMode(m)}>
                                {m === "none" ? L.upgradesNone : m === "rec" ? L.upgradesRec : L.upgradesAll}
                              </button>
                            ))}
                          </div>
                        </div>
                        <div className="upgrade-list">
                          {card.upgrades.map((u) => (
                            <label key={u.id} className={checked.has(u.id) ? "on" : ""} title={u.name}>
                              <input type="checkbox" checked={checked.has(u.id)} onChange={() => toggle(u.id)} />
                              <img src={`/img/tech/${u.id}.png`} alt="" />
                              <span>
                                {u.name}
                                {u.recommended && <b className="rec">★</b>}
                                <small>
                                  <CostLine cost={u.cost} />
                                </small>
                              </span>
                            </label>
                          ))}
                        </div>
                        <p className="muted tiny">{L.upgradesNote}</p>
                      </div>
                    )}
                    <details open={details} onToggle={(e) => setDetails((e.target as HTMLDetailsElement).open)}>
                      <summary>{L.moreDetails}</summary>
                      <table className="stat-table">
                        <tbody>
                          <tr>
                            <td>{L.attack}</td>
                            <td>{card.attack}</td>
                          </tr>
                          <tr>
                            <td>{L.armor}</td>
                            <td>
                              {card.melee_armor} / {card.pierce_armor}
                            </td>
                          </tr>
                          <tr>
                            <td>{L.range}</td>
                            <td>{fmt(card.range)}</td>
                          </tr>
                          <tr>
                            <td>{L.reload}</td>
                            <td>{fmt(card.reload)} s</td>
                          </tr>
                          <tr>
                            <td>{L.speed}</td>
                            <td>{fmt(card.speed)}</td>
                          </tr>
                        </tbody>
                      </table>
                    </details>
                  </>
                )}
              </>
            )}
          </div>
        </div>
        <div className="modal-actions">
          {kind !== "tech" && (
            <label className="qty">
              {L.quantity}
              <input type="number" min={1} max={200} value={qty} onChange={(e) => setQty(Number(e.target.value))} />
            </label>
          )}
          <button onClick={onClose}>{L.cancel}</button>
          <button className="primary" disabled={!pick} onClick={add}>
            {L.addToGoal}
          </button>
        </div>
      </div>
    </div>
  );
}
