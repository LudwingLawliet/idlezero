import { useEffect, useMemo, useRef, useState } from "react";
import { MapLayout, PX_PER_SECOND, TC_W, TC_X } from "./layout";
import { Labels } from "./labels";
import { RESOURCES, Timeline, TreeNode, clock } from "./types";

export type Selection = { kind: "villager"; id: number } | { kind: "building"; placement: number } | null;

const STRIP_W = 64;
const STRIP_GAP = 18;
const TOP = 90; // espacio para cabeceras y la meta
const BOTTOM = 110; // bloque de inicio

interface Props {
  layout: MapLayout;
  timeline: Timeline;
  tree: TreeNode[];
  labels: Labels;
  selected: Selection;
  onSelect: (s: Selection) => void;
  cursor: number | null;
  onCursor: (t: number | null) => void;
  goalTime: number | null;
}

export default function EconomyMap({ layout, timeline, tree, labels, selected, onSelect, cursor, onCursor, goalTime }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ x: 0, y: 0, k: 1 });
  const drag = useRef<{ x: number; y: number; vx: number; vy: number; moved: boolean } | null>(null);
  const L = labels.ui;
  const { y } = layout;

  const stripsX = layout.right + 10;
  const contentRight = stripsX + RESOURCES.length * (STRIP_W + STRIP_GAP);
  const width = contentRight - layout.left;
  const height = TOP + layout.height + BOTTOM;

  // Vista inicial: todo el ancho y el inicio (abajo) a la vista.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const k = Math.min(1.1, Math.max(0.35, el.clientWidth / width));
    setView({ k, x: -layout.left * k + Math.max(0, (el.clientWidth - width * k) / 2), y: el.clientHeight - height * k });
  }, [layout, width, height]);

  const toContent = (cx: number, cy: number) => {
    const r = box.current!.getBoundingClientRect();
    return { x: (cx - r.left - view.x) / view.k, y: (cy - r.top - view.y) / view.k - TOP };
  };

  const zoomAt = (mx: number, my: number, f: number) =>
    setView((v) => {
      const k = Math.min(3, Math.max(0.2, v.k * f));
      return { k, x: mx - ((mx - v.x) * k) / v.k, y: my - ((my - v.y) * k) / v.k };
    });

  // Rueda: subir/bajar en el tiempo; Mayús+rueda: izquierda/derecha; Ctrl+rueda: zoom.
  const onWheel = (e: React.WheelEvent) => {
    const r = box.current!.getBoundingClientRect();
    if (e.ctrlKey || e.metaKey) {
      zoomAt(e.clientX - r.left, e.clientY - r.top, Math.exp(-e.deltaY * 0.0015));
    } else if (e.shiftKey) {
      setView((v) => ({ ...v, x: v.x - (e.deltaY || e.deltaX) }));
    } else {
      setView((v) => ({ ...v, x: v.x - e.deltaX, y: v.y - e.deltaY }));
    }
  };

  // Todos los aldeanos nacen en el carril de los CU (la etiqueta dice de cuál).
  const bornX = (_tc: number | null) => TC_X + TC_W / 2 - 10;
  const villagerPath = useMemo(() => {
    const byV = new Map<number, string>();
    for (const b of layout.births) {
      const segs = layout.segments.filter((s) => s.villager === b.villager);
      let d = `M ${bornX(b.tc) + 10} ${y(b.time)}`;
      for (const s of segs) d += ` L ${s.x} ${y(s.start)} L ${s.x} ${y(s.end)}`;
      byV.set(b.villager, d);
    }
    return byV;
  }, [layout, y]);

  const selV = selected?.kind === "villager" ? selected.id : null;
  const selB = selected?.kind === "building" ? selected.placement : null;
  const selNode = selV !== null ? tree.find((n) => n.villager === selV) : undefined;
  const maxRes = RESOURCES.map((r) => Math.max(1, ...timeline.series[r]));
  const minutes = Array.from({ length: Math.floor(layout.end / 60) + 1 }, (_, i) => i * 60);

  return (
    <div
      ref={box}
      className="emap"
      onWheel={onWheel}
      onMouseDown={(e) => (drag.current = { x: e.clientX, y: e.clientY, vx: view.x, vy: view.y, moved: false })}
      onMouseMove={(e) => {
        const d = drag.current;
        if (d) {
          if (Math.abs(e.clientX - d.x) + Math.abs(e.clientY - d.y) > 3) d.moved = true;
          setView((v) => ({ ...v, x: d.vx + e.clientX - d.x, y: d.vy + e.clientY - d.y }));
          return;
        }
        const p = toContent(e.clientX, e.clientY);
        const t = layout.end - p.y / PX_PER_SECOND;
        onCursor(t >= 0 && t <= layout.end ? Math.round(t) : null);
      }}
      onMouseUp={() => setTimeout(() => (drag.current = null))}
      onMouseLeave={() => {
        drag.current = null;
      }}
    >
      <div className="zoom">
        <button onClick={() => zoomAt(box.current!.clientWidth / 2, box.current!.clientHeight / 2, 1.25)}>+</button>
        <button onClick={() => zoomAt(box.current!.clientWidth / 2, box.current!.clientHeight / 2, 0.8)}>−</button>
      </div>
      <svg width="100%" height="100%">
        <g transform={`translate(${view.x} ${view.y}) scale(${view.k})`}>
          <g transform={`translate(0 ${TOP})`}>
            {/* Rejilla de minutos */}
            {minutes.map((t) => (
              <g key={t} className="grid">
                <line x1={layout.left} x2={contentRight} y1={y(t)} y2={y(t)} />
                <text x={layout.left + 4} y={y(t) - 4}>
                  {clock(t)}
                </text>
              </g>
            ))}

            {/* Edades: banda entre el clic y la llegada */}
            {layout.ages.map((a) => (
              <g key={a.age} className="age">
                {a.click !== undefined && (
                  <rect x={layout.left} width={contentRight - layout.left} y={y(a.reached)} height={y(a.click) - y(a.reached)} />
                )}
                <line x1={layout.left} x2={contentRight} y1={y(a.reached)} y2={y(a.reached)} />
                <image href={`/img/age/${a.age}.png`} x={layout.left + 50} y={y(a.reached) - 26} width={22} height={22} />
                <text x={layout.left + 78} y={y(a.reached) - 9} className="age-label">
                  {labels.t(`ages.${a.age}`)} · {L.reached} {clock(a.reached)}
                  {a.click !== undefined ? ` · ${L.click} ${clock(a.click)}` : ""}
                </text>
              </g>
            ))}

            {/* Cabeceras de zona (arriba) */}
            <g className="heads">
              <text x={TC_X - 60} y={-56} textAnchor="end" className="zone">
                ← {L.build}
              </text>
              <text x={TC_X + TC_W / 2} y={-56} textAnchor="middle" className="zone">
                {layout.tcCount > 1 ? `${L.tc} ×${layout.tcCount}` : L.tc}
              </text>
              <text x={TC_X + TC_W + 46} y={-56} className="zone">
                {L.gather} →
              </text>
              {layout.columns.map((c) => (
                <g key={c.pool}>
                  <image href={labels.poolIcon(c.pool)} x={c.x} y={-44} width={18} height={18} />
                  <text x={c.x + 22} y={-30} className="col-label">
                    {labels.pool(c.pool)}
                  </text>
                  <rect x={c.x - 2} y={0} width={c.width} height={layout.height} className="col" />
                </g>
              ))}
              {RESOURCES.map((r, i) => (
                <g key={r}>
                  <image href={`/img/resource/${r}.png`} x={stripsX + i * (STRIP_W + STRIP_GAP)} y={-44} width={18} height={18} />
                  <text x={stripsX + i * (STRIP_W + STRIP_GAP) + 22} y={-30} className="col-label">
                    {labels.t(`resources.${r}`)}
                  </text>
                </g>
              ))}
            </g>

            {/* Centros urbanos */}
            <rect x={TC_X} y={0} width={TC_W} height={layout.height} rx={6} className="tc" />

            {/* Edificios */}
            {layout.buildings.map((b) => {
              const top = y(Math.max(b.end, b.start + 40 / PX_PER_SECOND));
              const h = y(b.start) - top;
              return (
                <g
                  key={b.placement}
                  className={`bld${selB === b.placement ? " selected" : ""}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    if (!drag.current?.moved) onSelect({ kind: "building", placement: b.placement });
                  }}
                >
                  <rect x={b.x} y={top} width={128} height={h} rx={6} />
                  <image href={labels.icon("building", b.building)} x={b.x + 5} y={top + 5} width={22} height={22} />
                  <text x={b.x + 31} y={top + 15} className="bname">
                    {labels.name("building", b.building)}
                  </text>
                  <text x={b.x + 31} y={top + 28} className="bmeta">
                    {clock(b.start)} · {b.end - b.start} {L.seconds} · {b.builders.length}👷
                  </text>
                  <line x1={b.x + 128} x2={TC_X - 4} y1={y(b.start)} y2={y(b.start)} className="blink" />
                </g>
              );
            })}

            {/* Ramas de los aldeanos */}
            {layout.births.map((b) => {
              const d = villagerPath.get(b.villager) ?? "";
              const on = selV === b.villager;
              return (
                <g
                  key={b.villager}
                  className={`branch${on ? " selected" : ""}${selV !== null && !on ? " dim" : ""}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    if (!drag.current?.moved) onSelect({ kind: "villager", id: b.villager });
                  }}
                >
                  <path d={d} className="hit" />
                  <path d={d} className="line" />
                  <circle cx={bornX(b.tc) + 10} cy={y(b.time)} r={on ? 5 : 3.5} className="birth" />
                  <text x={TC_X - 4} y={y(b.time) + 4} textAnchor="end" className="vlabel">
                    a{b.villager}
                    {b.tc !== null && b.tc > 0 ? ` ·CU${b.tc + 1}` : ""}
                  </text>
                </g>
              );
            })}

            {/* Ramas alternativas del aldeano seleccionado */}
            {selNode &&
              selNode.alternatives.slice(0, 5).map((a, i) => {
                const x0 = bornX(null) + 10;
                const y0 = y(selNode.time);
                const x1 = layout.poolX[a.pool] ?? layout.right;
                const y1 = y(selNode.time) - 18 - i * 16;
                const cls = a.delta === null ? "fail" : a.delta === 0 ? "same" : a.delta > 0 ? "worse" : "better";
                const txt =
                  a.delta === null ? L.impossible : a.delta === 0 ? `= ${L.same}` : `${a.delta > 0 ? "+" : ""}${a.delta} s`;
                return (
                  <g key={i} className={`alt ${cls}`}>
                    <path d={`M ${x0} ${y0} C ${x0 + 40} ${y1}, ${x1 - 40} ${y1}, ${x1} ${y1}`} />
                    <text x={x1 + 6} y={y1 + 4}>
                      {a.build ? `${labels.name("building", a.build)} → ` : ""}
                      {labels.pool(a.pool)} {txt}
                    </text>
                  </g>
                );
              })}

            {/* Reservas: una tira por recurso (x = cantidad, y = tiempo) */}
            {RESOURCES.map((r, i) => {
              const x0 = stripsX + i * (STRIP_W + STRIP_GAP);
              const s = timeline.series[r];
              const step = Math.max(1, Math.floor(s.length / 600));
              let d = "";
              for (let t = 0; t < s.length; t += step) d += `${t === 0 ? "M" : "L"} ${x0 + (s[t] / maxRes[i]) * STRIP_W} ${y(t)} `;
              return (
                <g key={r} className="strip">
                  <line x1={x0} x2={x0} y1={0} y2={layout.height} className="base" />
                  <path d={d} />
                  <text x={x0 + STRIP_W} y={layout.height + 14} textAnchor="end" className="max">
                    {L.max} {Math.round(maxRes[i])}
                  </text>
                </g>
              );
            })}

            {/* Meta (arriba) e inicio (abajo) */}
            <g className="goal-banner">
              <text x={TC_X + TC_W / 2} y={-8} textAnchor="middle">
                {goalTime !== null ? `🏁 ${L.goalReached} · ${clock(goalTime)}` : L.notReached}
              </text>
            </g>
            <g className="start-block">
              <rect x={layout.left + 4} y={layout.height + 24} width={contentRight - layout.left - 8} height={66} rx={8} />
              <text x={layout.left + 18} y={layout.height + 48} className="start-title">
                {L.start} · 0:00 · {labels.t("ages.Dark")}
              </text>
              <text x={layout.left + 18} y={layout.height + 70}>
                {L.startDesc} · {labels.t("resources.food")} {Math.round(timeline.series.food[0])} ·{" "}
                {labels.t("resources.wood")} {Math.round(timeline.series.wood[0])} · {labels.t("resources.gold")}{" "}
                {Math.round(timeline.series.gold[0])} · {labels.t("resources.stone")} {Math.round(timeline.series.stone[0])} ·{" "}
                {L.population} {timeline.series.pop[0]}/{timeline.series.pop_cap[0]}
              </text>
            </g>

            {/* Cursor de tiempo */}
            {cursor !== null && (
              <g className="cursor">
                <line x1={layout.left} x2={contentRight} y1={y(cursor)} y2={y(cursor)} />
                <rect x={layout.left + 2} y={y(cursor) - 20} width={44} height={16} rx={4} />
                <text x={layout.left + 24} y={y(cursor) - 8} textAnchor="middle">
                  {clock(cursor)}
                </text>
                {RESOURCES.map((r, i) => {
                  const v = timeline.series[r][Math.min(cursor, timeline.series[r].length - 1)] ?? 0;
                  const x0 = stripsX + i * (STRIP_W + STRIP_GAP);
                  return (
                    <g key={r}>
                      <circle cx={x0 + (v / maxRes[i]) * STRIP_W} cy={y(cursor)} r={4} />
                      <text x={x0 + STRIP_W} y={y(cursor) - 6} textAnchor="end" className="val">
                        {Math.round(v)}
                      </text>
                    </g>
                  );
                })}
              </g>
            )}
          </g>
        </g>
      </svg>
    </div>
  );
}
