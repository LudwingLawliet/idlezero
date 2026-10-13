import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Labels } from "./labels";
import { Age, AppInfo, CivView, Cost, Goal, clock } from "./types";
import UnitPicker, { CostLine } from "./UnitPicker";
import CivPicker from "./CivPicker";

const AGES: Age[] = ["Dark", "Feudal", "Castle", "Imperial"];
const SIDES = [0, 1] as const;

interface SideState {
  civ: string;
  age: Age;
  techs: string[];
  squads: { unit: string; count: number }[];
}

interface UnitCount {
  id: number;
  name: string;
  count: number;
  hp: number;
}

interface BattleView {
  winner: "a" | "b" | null;
  seconds: number;
  hp: [number[], number[]];
  sample: number;
  sides: { cost: Cost; destroyed: number; survivors: UnitCount[] }[];
}

interface DuelView {
  goal_time: [number | null, number | null];
  minutes: { minute: number; units: [UnitCount[], UnitCount[]]; winner: "a" | "b" | null; margin: number }[];
}

function stored<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return v ? (JSON.parse(v) as T) : fallback;
  } catch {
    return fallback;
  }
}

function remember(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Sin almacenamiento: solo no se recuerda.
  }
}

const sum = (c: Cost) => c.food + c.wood + c.gold + c.stone;

/** Gráfica de la vida de cada ejército en el tiempo. */
function HpChart({ hp, sample, labels }: { hp: [number[], number[]]; sample: number; labels: Labels }) {
  const W = 640;
  const H = 180;
  const P = { l: 44, r: 12, t: 10, b: 24 };
  const n = Math.max(hp[0].length, hp[1].length, 2);
  const max = Math.max(1, ...hp[0], ...hp[1]);
  const x = (i: number) => P.l + (i / (n - 1)) * (W - P.l - P.r);
  const y = (v: number) => P.t + (1 - Math.max(0, v) / max) * (H - P.t - P.b);
  const path = (s: number[]) => s.map((v, i) => `${i ? "L" : "M"} ${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
  const total = (n - 1) * sample;
  const step = total > 120 ? 30 : total > 40 ? 10 : 5;
  const ticks: number[] = [];
  for (let t = 0; t <= total; t += step) ticks.push(t);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="chart hp-chart" role="img" aria-label={labels.ui.hpOverTime}>
      {ticks.map((t) => (
        <g key={t}>
          <line x1={x(t / sample)} x2={x(t / sample)} y1={P.t} y2={H - P.b} className="grid" />
          <text x={x(t / sample)} y={H - 6} textAnchor="middle" className="axis">
            {t} s
          </text>
        </g>
      ))}
      {[0, max / 2, max].map((v) => (
        <text key={v} x={P.l - 6} y={y(v) + 4} textAnchor="end" className="axis">
          {Math.round(v)}
        </text>
      ))}
      {SIDES.map((s) => (
        <path key={s} d={path(hp[s])} className={`hp-line side-${s}`} />
      ))}
    </svg>
  );
}

/** Ventaja minuto a minuto del duelo de estrategias. */
function DuelChart({ duel: full, labels }: { duel: DuelView; labels: Labels }) {
  const L = labels.ui;
  const W = 640;
  const H = 230;
  const P = { l: 36, r: 10, t: 26, b: 44 };
  // Desde un minuto antes de que alguien tenga ejército.
  const first = Math.max(0, full.minutes.findIndex((m) => m.units[0].length + m.units[1].length > 0) - 1);
  const duel = { ...full, minutes: full.minutes.slice(first) };
  const m0 = duel.minutes[0]?.minute ?? 1;
  const n = duel.minutes.length;
  const bw = (W - P.l - P.r) / Math.max(n, 1);
  const mid = (H - P.b + P.t) / 2;
  const half = (H - P.b - P.t) / 2;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="chart duel-chart" role="img" aria-label={L.duelTitle}>
      <line x1={P.l} x2={W - P.r} y1={mid} y2={mid} className="grid strong" />
      <text x={P.l - 6} y={P.t + 10} textAnchor="end" className="axis side-0-text">
        A
      </text>
      <text x={P.l - 6} y={H - P.b - 2} textAnchor="end" className="axis side-1-text">
        B
      </text>
      {duel.minutes.map((m, i) => {
        const h = Math.max(m.winner ? 3 : 0, m.margin * half);
        const xx = P.l + i * bw + 2;
        const title = `${m.minute} ${L.minute} · ${
          m.winner === "a" ? `${L.sideA} ${Math.round(m.margin * 100)} %` : m.winner === "b" ? `${L.sideB} ${Math.round(m.margin * 100)} %` : m.units[0].length + m.units[1].length ? L.draw : L.duelNobody
        }\nA: ${m.units[0].map((u) => `${u.count} ${u.name}`).join(", ") || "—"}\nB: ${m.units[1].map((u) => `${u.count} ${u.name}`).join(", ") || "—"}`;
        return (
          <g key={m.minute}>
            <title>{title}</title>
            <rect x={xx} y={P.t} width={bw - 4} height={H - P.b - P.t} className="duel-hit" />
            {m.winner === "a" && <rect x={xx} y={mid - h} width={bw - 4} height={h} rx={2} className="side-0-fill" />}
            {m.winner === "b" && <rect x={xx} y={mid} width={bw - 4} height={h} rx={2} className="side-1-fill" />}
            {(i % Math.ceil(n / 16) === 0 || i === n - 1) && (
              <text x={xx + (bw - 4) / 2} y={H - P.b + 14} textAnchor="middle" className="axis">
                {m.minute}
              </text>
            )}
          </g>
        );
      })}
      {SIDES.map((s) => {
        const t = duel.goal_time[s];
        if (t === null) return null;
        const xx = P.l + (t / 60 - m0 + 0.5) * bw;
        if (xx < P.l) return null;
        return (
          <g key={s} className={`duel-goal side-${s}-text`}>
            <line x1={xx} x2={xx} y1={P.t - 4} y2={H - P.b + 4} />
            <text x={xx} y={s === 0 ? P.t - 8 : H - 6} textAnchor={xx > W - 160 ? "end" : "middle"}>
              {s === 0 ? "A" : "B"} · {L.duelGoal} {clock(t)}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

export default function Battle({ info, labels, civs, lang, map }: { info: AppInfo; labels: Labels; civs: CivView[]; lang: string; map: string }) {
  const L = labels.ui;
  const keyOf = (id: number) => info.catalog.units.find((u) => u.id === id)?.key ?? String(id);
  const [sides, setSides] = useState<[SideState, SideState]>(() =>
    stored("battle", [
      { civ: "", age: "Castle", techs: [], squads: [{ unit: keyOf(38), count: 10 }] },
      { civ: "", age: "Castle", techs: [], squads: [{ unit: keyOf(4), count: 10 }] },
    ] as [SideState, SideState]),
  );
  const [picking, setPicking] = useState<number | null>(null);
  const [civFor, setCivFor] = useState<number | null>(null);
  const [result, setResult] = useState<BattleView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const unitPresets = useMemo(() => info.presets.filter((p) => p.goal.units.length > 0), [info]);
  const [duelPlans, setDuelPlans] = useState<[string, string]>(() =>
    stored("duelPlans", [unitPresets.find((p) => p.id === "knights_10")?.id ?? unitPresets[0]?.id ?? "", unitPresets.find((p) => p.id === "archers_feudal")?.id ?? unitPresets[1]?.id ?? ""]),
  );
  const [duel, setDuel] = useState<DuelView | null>(null);
  const [duelBusy, setDuelBusy] = useState(false);

  useEffect(() => remember("battle", sides), [sides]);
  useEffect(() => remember("duelPlans", duelPlans), [duelPlans]);

  const update = (i: number, patch: Partial<SideState>) => {
    const next = [...sides] as [SideState, SideState];
    next[i] = { ...next[i], ...patch };
    setSides(next);
    setResult(null);
  };
  const unitName = (key: string) => labels.name("unit", key);

  const run = () => {
    setBusy(true);
    setError(null);
    invoke<BattleView>("battle", { a: { ...sides[0], civ: sides[0].civ || null }, b: { ...sides[1], civ: sides[1].civ || null }, lang })
      .then(setResult, (e) => setError(String(e)))
      .finally(() => setBusy(false));
  };

  const runDuel = () => {
    const goal = (id: string) => info.presets.find((p) => p.id === id)?.goal as Goal;
    setDuelBusy(true);
    setError(null);
    invoke<DuelView>("strategy_duel", {
      map,
      a: { goal: goal(duelPlans[0]), civ: sides[0].civ || null },
      b: { goal: goal(duelPlans[1]), civ: sides[1].civ || null },
      lang,
    })
      .then(setDuel, (e) => setError(String(e) === "cancelled" ? null : String(e)))
      .finally(() => setDuelBusy(false));
  };

  // Resumen del duelo: tramos de minutos en que gana cada bando.
  const duelSummary = useMemo(() => {
    if (!duel) return [];
    const spans: { side: "a" | "b"; from: number; to: number }[] = [];
    for (const m of duel.minutes) {
      if (!m.winner) continue;
      const last = spans[spans.length - 1];
      if (last && last.side === m.winner && last.to === m.minute - 1) last.to = m.minute;
      else spans.push({ side: m.winner, from: m.minute, to: m.minute });
    }
    return spans;
  }, [duel]);

  const civOf = (i: number) => civs.find((c) => c.id === sides[i].civ);

  return (
    <div className="battle">
      <div className="battle-sides">
        {SIDES.map((i) => {
          const s = sides[i];
          const civ = civOf(i);
          return (
            <section key={i} className={`battle-side side-${i}`}>
              <header>
                <span className="side-tag">{i === 0 ? L.sideA : L.sideB}</span>
                <button className="civ-button small" onClick={() => setCivFor(i)}>
                  {civ ? <img src={civ.icon} alt="" /> : <span className="civ-neutral">⚖</span>}
                  <span>{civ?.name ?? L.neutral}</span>
                </button>
                <div className="seg ages" role="group" aria-label={L.age}>
                  {AGES.map((a) => (
                    <button key={a} className={s.age === a ? "active" : ""} onClick={() => update(i, { age: a })} title={labels.t(`ages.${a}`)}>
                      <img src={`/img/age/${a}.png`} alt={labels.t(`ages.${a}`)} />
                    </button>
                  ))}
                </div>
              </header>
              <div className="squads">
                {s.squads.length === 0 && <p className="muted tiny">{L.emptyArmy}</p>}
                {s.squads.map((q, k) => (
                  <div className="squad" key={q.unit}>
                    <img src={labels.icon("unit", q.unit)} alt="" />
                    <span className="squad-name">{unitName(q.unit)}</span>
                    <input
                      type="number"
                      min={1}
                      max={200}
                      value={q.count}
                      aria-label={L.quantity}
                      onChange={(e) => update(i, { squads: s.squads.map((x, j) => (j === k ? { ...x, count: Math.max(1, Math.min(200, Number(e.target.value) || 1)) } : x)) })}
                    />
                    <button className="icon-btn" onClick={() => update(i, { squads: s.squads.filter((_, j) => j !== k) })} aria-label="×">
                      ×
                    </button>
                  </div>
                ))}
                <button className="add-btn" onClick={() => setPicking(i)}>
                  <img src="/img/unit/74.png" alt="" className="icon tiny" />+ {L.addUnit}
                </button>
              </div>
              {s.techs.length > 0 && (
                <div className="tech-chips">
                  <span className="muted tiny">{L.upgradesShort}:</span>
                  {s.techs.map((t) => (
                    <span key={t} className="chip small" title={labels.name("tech", t)}>
                      <img src={labels.icon("tech", t)} alt="" className="icon tiny" />
                      <button onClick={() => update(i, { techs: s.techs.filter((x) => x !== t) })} aria-label="×">
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              )}
            </section>
          );
        })}
        <div className="battle-vs">
          <span>VS</span>
          <button className="primary big" onClick={run} disabled={busy || !sides[0].squads.length || !sides[1].squads.length}>
            {busy ? L.fighting : L.fight}
          </button>
        </div>
      </div>

      {error && <div className="error">{error}</div>}

      {result && (
        <section className="chart-card battle-result">
          <div className={`winner-banner ${result.winner ? `side-${result.winner === "a" ? 0 : 1}` : ""}`}>
            {result.winner ? (
              <>
                <strong>{result.winner === "a" ? L.sideA : L.sideB}</strong> {L.winsIn} {result.seconds.toFixed(1)} s
              </>
            ) : (
              <strong>{L.draw}</strong>
            )}
          </div>
          <div className="result-grid">
            {SIDES.map((i) => {
              const side = result.sides[i];
              return (
                <div key={i} className={`result-side side-${i}`}>
                  <h4>{i === 0 ? L.sideA : L.sideB}</h4>
                  <div className="kv">
                    <span>{L.survivors}</span>
                    <span className="ref-icons">
                      {side.survivors.length === 0 && <span className="muted">{L.none}</span>}
                      {side.survivors.map((u) => (
                        <span key={u.id} title={`${u.name} · ${u.hp} PV`}>
                          <img src={`/img/unit/${u.id}.png`} alt={u.name} />×{u.count}
                        </span>
                      ))}
                    </span>
                  </div>
                  <div className="kv">
                    <span>{L.armyCost}</span>
                    <span>
                      <CostLine cost={side.cost} /> ({sum(side.cost)})
                    </span>
                  </div>
                  <div className="kv">
                    <span>{L.destroyed}</span>
                    <span>{side.destroyed}</span>
                  </div>
                </div>
              );
            })}
          </div>
          <h4>{L.hpOverTime}</h4>
          <div className="legend-row">
            {SIDES.map((i) => (
              <span key={i}>
                <i className={`swatch side-${i}-fill`} /> {i === 0 ? L.sideA : L.sideB}
              </span>
            ))}
          </div>
          <HpChart hp={result.hp} sample={result.sample} labels={labels} />
        </section>
      )}

      <section className="chart-card duel">
        <h3>{L.duelTitle}</h3>
        <p className="muted tiny">{L.duelDesc}</p>
        <div className="duel-controls">
          {SIDES.map((i) => (
            <label key={i} className={`side-${i}-text`}>
              {i === 0 ? L.sideA : L.sideB} · {civOf(i)?.name ?? L.neutral} · {L.duelPlan}
              <select value={duelPlans[i]} onChange={(e) => setDuelPlans((p) => (i === 0 ? [e.target.value, p[1]] : [p[0], e.target.value]))}>
                {unitPresets.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
          ))}
          {duelBusy ? (
            <button className="primary busy-btn" onClick={() => invoke("cancel_optimize")} title={L.cancelCalc}>
              <span className="spinner" aria-hidden="true" />
              {L.duelRunning} · {L.cancel}
            </button>
          ) : (
            <button className="primary" onClick={runDuel}>
              {L.duelRun}
            </button>
          )}
        </div>
        {duel && (
          <>
            <ul className="duel-summary">
              {duelSummary.map((s, k) => (
                <li key={k} className={`side-${s.side === "a" ? 0 : 1}-text`}>
                  <strong>{s.side === "a" ? L.sideA : L.sideB}</strong> {L.duelLead}: {s.from === s.to ? `${L.minute} ${s.from}` : `${L.minute} ${s.from}–${s.to}`}
                </li>
              ))}
            </ul>
            <DuelChart duel={duel} labels={labels} />
          </>
        )}
      </section>

      <p className="muted tiny battle-limits">{L.battleLimits}</p>

      {picking !== null && (
        <UnitPicker
          kind="unit"
          list={info.catalog.units}
          civ={civOf(picking)}
          lang={lang}
          labels={labels}
          onClose={() => setPicking(null)}
          onAdd={(r) => {
            const s = sides[picking];
            update(picking, {
              squads: [...s.squads.filter((q) => q.unit !== r.key), { unit: r.key, count: r.count }],
              techs: [...s.techs.filter((t) => !r.techs.includes(t)), ...r.techs],
            });
            setPicking(null);
          }}
        />
      )}
      {civFor !== null && (
        <CivPicker
          civs={civs}
          current={sides[civFor].civ}
          labels={labels}
          onClose={() => setCivFor(null)}
          onAccept={(id) => {
            update(civFor, { civ: id });
            setCivFor(null);
          }}
        />
      )}
    </div>
  );
}
