import { useMemo, useState } from "react";
import { Labels } from "./labels";
import { GoalItem, GoalRow, goalStart } from "./goalProgress";
import { RESOURCES, clock } from "./types";

const W = 900;
const LABEL = 170; // columna de nombres
const PAD_R = 20;
const ROW = 64;
const AXIS = 56; // eje + dos filas de marcas
const BADGE = 26; // diámetro de la insignia

interface Placed {
  item: GoalItem;
  x: number;
  y: number;
  xEnd: number | null;
}

/**
 * Gráfica del objetivo: una fila por edificio, unidad, tecnologías y edad, con
 * una insignia numerada en el segundo en que hubo recursos para pagar cada
 * una y una barra hasta que se terminó.
 */
export default function GoalTimeline({ rows, end, goalTime, labels }: { rows: GoalRow[]; end: number; goalTime: number | null; labels: Labels }) {
  const L = labels.ui;
  const [sel, setSel] = useState<{ row: number; n: number } | null>(null);
  const start = goalStart(rows);
  // Ventana del objetivo: del primer pago a la meta, con un margen a cada lado.
  const times = rows.flatMap((r) => r.items.flatMap((i) => [i.paid, i.done])).filter((t): t is number => t !== null);
  const last = Math.max(goalTime ?? 0, ...times, 1);
  const first = Math.min(start ?? last, ...rows.filter((r) => r.kind !== "age").flatMap((r) => r.items.map((i) => i.paid ?? last)));
  const pad = Math.max(20, (last - first) * 0.08);
  const t0 = Math.max(0, Math.floor(first - pad));
  const t1 = Math.min(Math.max(end, last), Math.ceil(last + pad));
  const span = Math.max(t1 - t0, 1);
  const x = (t: number) => LABEL + ((t - t0) / span) * (W - LABEL - PAD_R);

  // Varias insignias juntas se reparten en sub-carriles para no taparse.
  const placed = useMemo(
    () =>
      rows.map((r, ri) => {
        const lanes: number[] = [];
        const out: Placed[] = [];
        for (const item of r.items) {
          if (item.paid === null) continue;
          const px = x(item.paid);
          let lane = lanes.findIndex((last) => px - last >= BADGE + 2);
          if (lane < 0) lane = lanes.length < 3 ? lanes.length : lanes.indexOf(Math.min(...lanes));
          lanes[lane] = px;
          const y = AXIS + ri * ROW + 20 + lane * 14;
          out.push({ item, x: px, y, xEnd: item.done !== null ? x(item.done) : null });
        }
        return out;
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [rows, span],
  );

  const H = AXIS + rows.length * ROW + 6;
  const ticks: number[] = [];
  const stepT = [10, 15, 30, 60, 120, 300].find((st) => span / st <= 10) ?? 600;
  for (let t = Math.ceil(t0 / stepT) * stepT; t <= t1; t += stepT) ticks.push(t);
  const chosen = sel ? rows[sel.row]?.items.find((i) => i.n === sel.n) : null;
  const chosenRow = sel ? rows[sel.row] : null;

  return (
    <div className="goal-timeline">
      <svg viewBox={`0 0 ${W} ${H}`} className="chart" role="img" aria-label={L.goalChart}>
        {t0 > 0 && (
          <text x={LABEL - 6} y={14} textAnchor="end" className="axis">
            {L.zoomedTo}
          </text>
        )}
        {ticks.map((t) => (
          <g key={t}>
            <line x1={x(t)} x2={x(t)} y1={AXIS} y2={H} className="grid" />
            <text x={x(t)} y={14} textAnchor="middle" className="axis">
              {clock(t)}
            </text>
          </g>
        ))}
        {rows.map((r, ri) => (
          <g key={r.key}>
            <rect x={0} y={AXIS + ri * ROW} width={W} height={ROW} className={ri % 2 ? "row-alt" : "row"} />
            <image href={r.icon} x={6} y={AXIS + ri * ROW + 10} width={44} height={44} />
            <text x={58} y={AXIS + ri * ROW + 30} className="row-name">
              {r.name.length > 16 ? `${r.name.slice(0, 15)}…` : r.name}
            </text>
            <text x={58} y={AXIS + ri * ROW + 48} className="row-sub">
              {r.kind === "unit" || r.kind === "building" ? `× ${r.count}` : `${r.items.filter((i) => i.done !== null).length}/${r.count}`}
            </text>
          </g>
        ))}
        {start !== null && (
          <g className="mark start">
            <line x1={x(start)} x2={x(start)} y1={AXIS - 32} y2={H} />
            <text x={x(start) + 4} y={AXIS - 22}>
              {L.goalStarts} {clock(start)}
            </text>
          </g>
        )}
        {goalTime !== null && (
          <g className="mark done">
            <line x1={x(goalTime)} x2={x(goalTime)} y1={AXIS - 18} y2={H} />
            <text x={x(goalTime) - 4} y={AXIS - 6} textAnchor="end">
              {L.goalDone} {clock(goalTime)}
            </text>
          </g>
        )}
        {placed.map((ps, ri) =>
          ps.map((p) => {
            const active = sel?.row === ri && sel.n === p.item.n;
            const icon = p.item.icon ?? rows[ri].icon;
            return (
              <g
                key={p.item.n}
                className={`badge${active ? " active" : ""}`}
                onClick={() => setSel(active ? null : { row: ri, n: p.item.n })}
                role="button"
              >
                <title>
                  {`${p.item.name ?? rows[ri].name} #${p.item.n} · ${L.paidAt} ${clock(p.item.paid!)}${p.item.done !== null ? ` · ${L.readyAt} ${clock(p.item.done)}` : ""}`}
                </title>
                {p.xEnd !== null && <line x1={p.x} x2={p.xEnd} y1={p.y} y2={p.y} className="build-bar" />}
                {p.xEnd !== null && <circle cx={p.xEnd} cy={p.y} r={3} className="build-end" />}
                <circle cx={p.x} cy={p.y} r={BADGE / 2} className="badge-bg" />
                <image href={icon} x={p.x - 10} y={p.y - 10} width={20} height={20} />
                <circle cx={p.x + 10} cy={p.y - 10} r={7} className="badge-n" />
                <text x={p.x + 10} y={p.y - 7} textAnchor="middle" className="badge-t">
                  {p.item.n}
                </text>
              </g>
            );
          }),
        )}
      </svg>
      <div className="goal-detail">
        {chosen && chosenRow ? (
          <>
            <img src={chosen.icon ?? chosenRow.icon} alt="" className="icon" />
            <div>
              <strong>
                {chosen.name ?? chosenRow.name} #{chosen.n}
              </strong>
              <div>
                {L.paidAt} <b>{chosen.paid !== null ? clock(chosen.paid) : "—"}</b>
                {chosen.done !== null && (
                  <>
                    {" · "}
                    {L.readyAt} <b>{clock(chosen.done)}</b>
                  </>
                )}
              </div>
              {chosen.before && chosen.cost && (
                <div className="res-row">
                  {L.hadBefore}:
                  {RESOURCES.map((r) => (
                    <span key={r} className={chosen.cost![r] > 0 ? "used" : ""}>
                      <img src={`/img/resource/${r}.png`} alt={r} className="icon tiny" />
                      {chosen.before![r]}
                      {chosen.cost![r] > 0 && <small> −{chosen.cost![r]}</small>}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </>
        ) : (
          <span className="muted">{L.goalChartHint}</span>
        )}
      </div>
    </div>
  );
}
