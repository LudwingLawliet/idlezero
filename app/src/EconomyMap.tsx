import { memo, useCallback, useEffect, useMemo, useRef } from "react";
import { MapLayout, PX_PER_SECOND, Segment, TC_W, TC_X } from "./layout";
import { Labels } from "./labels";
import { RESOURCES, Resource, Timeline, TreeNode, clock } from "./types";

export type Selection = { kind: "villager"; id: number } | { kind: "building"; placement: number } | null;

const STRIP_W = 64;
const STRIP_GAP = 18;
const TOP = 90; // espacio para cabeceras y la meta
const BOTTOM = 120; // nombres de columna + bloque de inicio
const BADGE = 9; // radio de las insignias de tarea
const CHEVRON_EVERY = 60; // segundos entre flechas de seguimiento
const MIN_K = 0.2;
const MAX_K = 3;

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
  /** Escudo de la civilización (para las bandas de edad). */
  civIcon?: string;
}

type View = { x: number; y: number; k: number };

/** Recurso de un tramo (para el color), o null si construye, espera o no hace nada. */
function segResource(s: Segment, labels: Labels): Resource | null {
  if (s.task.kind === "gather" || s.task.kind === "lure") return labels.poolResource(s.task.pool);
  if (s.task.kind === "build" && s.task.building === "farm") return "food";
  return null;
}

function taskIcon(s: Segment, labels: Labels): string {
  const t = s.task;
  if (t.kind === "gather" || t.kind === "lure") return labels.poolIcon(t.pool);
  if (t.kind === "build" || t.kind === "wait") return labels.icon("building", t.building);
  return "/img/unit/83.png";
}

// ---------- capa estática (memorizada: no se redibuja al mover el cursor ni al hacer zoom) ----------

interface ContentProps {
  layout: MapLayout;
  timeline: Timeline;
  labels: Labels;
  selV: number | null;
  selB: number | null;
  selNode: TreeNode | undefined;
  goalTime: number | null;
  civIcon?: string;
  onSelect: (s: Selection) => void;
  dragMoved: () => boolean;
}

const Content = memo(function Content({ layout, timeline, labels, selV, selB, selNode, goalTime, civIcon, onSelect, dragMoved }: ContentProps) {
  const L = labels.ui;
  const { y } = layout;
  const stripsX = layout.right + 10;
  const contentRight = stripsX + RESOURCES.length * (STRIP_W + STRIP_GAP);
  const maxRes = RESOURCES.map((r) => Math.max(1, ...timeline.series[r]));
  const minutes = Array.from({ length: Math.floor(layout.end / 60) + 1 }, (_, i) => i * 60);
  const bornX = TC_X + TC_W / 2;

  // Ramas: por aldeano, tramos verticales coloreados por recurso y conectores con flecha.
  const branches = useMemo(() => {
    const placed: [number, number][] = [];
    const free = (x: number, yy: number) => {
      if (placed.some(([px, py]) => Math.abs(px - x) < BADGE * 2 && Math.abs(py - yy) < BADGE * 2)) return false;
      placed.push([x, yy]);
      return true;
    };
    return layout.births.map((b) => {
      const segs = layout.segments.filter((s) => s.villager === b.villager);
      let px = bornX;
      let py = y(b.time);
      const parts: { d: string; cls: string; arrow: boolean; title: string }[] = [];
      const chevrons: { x: number; y: number; cls: string }[] = [];
      const badges: { x: number; y: number; icon: string; cls: string }[] = [];
      segs.forEach((s, i) => {
        const res = segResource(s, labels);
        const cls = res ?? "build";
        const y0 = y(s.start);
        const y1 = y(s.end);
        // Conector: del punto anterior a la columna de la nueva tarea.
        if (Math.abs(px - s.x) > 0.5 || Math.abs(py - y0) > 0.5) {
          parts.push({ d: `M ${px} ${py} L ${s.x} ${y0}`, cls: "link", arrow: true, title: "" });
        }
        const title = `${L.villager} ${b.villager} · ${clock(s.start)}–${clock(s.end)} · ${labels.task(s.task)}`;
        parts.push({ d: `M ${s.x} ${y0} L ${s.x} ${y1}`, cls, arrow: false, title });
        // Flechas de seguimiento a lo largo del tramo (el tiempo sube).
        for (let t = s.start + CHEVRON_EVERY; t < s.end - 10; t += CHEVRON_EVERY) chevrons.push({ x: s.x, y: y(t), cls });
        // Insignia con el ícono de la tarea al empezar el tramo (si no tapa otra).
        const prev = segs[i - 1];
        const changed = !prev || taskIcon(prev, labels) !== taskIcon(s, labels);
        if (changed && s.end - s.start >= 8 && free(s.x, y0 - BADGE - 2)) {
          badges.push({ x: s.x, y: y0 - BADGE - 2, icon: taskIcon(s, labels), cls });
        }
        px = s.x;
        py = y1;
      });
      return { b, parts, chevrons, badges };
    });
  }, [layout, labels, y, bornX, L.villager]);

  return (
    <g transform={`translate(0 ${TOP})`}>
      <defs>
        {(["food", "wood", "gold", "stone", "build", "link", "sel"] as const).map((c) => (
          <marker key={c} id={`arr-${c}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M 0 0 L 10 5 L 0 10 z" className={`arrowhead ${c}`} />
          </marker>
        ))}
      </defs>

      {/* Rejilla de minutos */}
      {minutes.map((t) => (
        <g key={t} className="grid">
          <line x1={layout.left} x2={contentRight} y1={y(t)} y2={y(t)} />
          <text x={layout.left + 4} y={y(t) - 4}>
            {clock(t)}
          </text>
        </g>
      ))}

      {/* Columnas de recolección (fondo) */}
      {layout.columns.map((c) => (
        <rect key={c.pool} x={c.x - 4} y={0} width={c.width + 4} height={layout.height} rx={6} className={`col ${labels.poolResource(c.pool)}`} />
      ))}

      {/* Edades: banda entre el clic y la llegada */}
      {layout.ages.map((a) => (
        <g key={a.age} className="age">
          {a.click !== undefined && <rect x={layout.left} width={contentRight - layout.left} y={y(a.reached)} height={y(a.click) - y(a.reached)} />}
          <line x1={layout.left} x2={contentRight} y1={y(a.reached)} y2={y(a.reached)} />
          {civIcon && <image href={civIcon} x={layout.left + 46} y={y(a.reached) - 28} width={24} height={24} />}
          <image href={`/img/age/${a.age}.png`} x={layout.left + (civIcon ? 74 : 50)} y={y(a.reached) - 28} width={24} height={24} />
          <text x={layout.left + (civIcon ? 104 : 80)} y={y(a.reached) - 10} className="age-label">
            {labels.t(`ages.${a.age}`)} · {L.reached} {clock(a.reached)}
            {a.click !== undefined ? ` · ${L.click} ${clock(a.click)}` : ""}
          </text>
        </g>
      ))}

      {/* Cabeceras (arriba y abajo) */}
      {[-56, layout.height + 16].map((hy, i) => (
        <g key={i} className="heads">
          <text x={TC_X - 60} y={hy} textAnchor="end" className="zone">
            ← {L.build}
          </text>
          <text x={TC_X + TC_W / 2} y={hy} textAnchor="middle" className="zone">
            {layout.tcCount > 1 ? `${L.tc} ×${layout.tcCount}` : L.tc}
          </text>
          {i === 0 && (
            <text x={TC_X + TC_W + 46} y={hy} className="zone">
              {L.gather} →
            </text>
          )}
          {layout.columns.map((c) => (
            <g key={c.pool}>
              <image href={labels.poolIcon(c.pool)} x={c.x} y={i === 0 ? -46 : layout.height + 2} width={20} height={20} />
              <text x={c.x + 24} y={i === 0 ? -31 : layout.height + 16} className="col-label">
                {labels.pool(c.pool)}
              </text>
            </g>
          ))}
        </g>
      ))}
      {RESOURCES.map((r, i) => (
        <g key={r} className="heads">
          <image href={`/img/resource/${r}.png`} x={stripsX + i * (STRIP_W + STRIP_GAP)} y={-46} width={20} height={20} />
          <text x={stripsX + i * (STRIP_W + STRIP_GAP) + 24} y={-31} className="col-label">
            {labels.t(`resources.${r}`)}
          </text>
        </g>
      ))}

      {/* Carril del CU */}
      <rect x={TC_X} y={0} width={TC_W} height={layout.height} rx={8} className="tc" />

      {/* Edificios: caja con ícono, hora, duración y constructores; flecha hacia el CU */}
      {layout.buildings.map((b) => {
        const top = y(Math.max(b.end, b.start + 40 / PX_PER_SECOND));
        const h = y(b.start) - top;
        return (
          <g
            key={b.placement}
            className={`bld${selB === b.placement ? " selected" : ""}`}
            onClick={(e) => {
              e.stopPropagation();
              if (!dragMoved()) onSelect({ kind: "building", placement: b.placement });
            }}
          >
            <title>
              {labels.name("building", b.building)} · {clock(b.start)}–{clock(b.end)} · {b.builders.map((v) => `a${v}`).join(", ")}
            </title>
            <rect x={b.x} y={top} width={128} height={h} rx={8} />
            <image href={labels.icon("building", b.building)} x={b.x + 6} y={top + 6} width={24} height={24} />
            <text x={b.x + 35} y={top + 16} className="bname">
              {labels.name("building", b.building)}
            </text>
            <text x={b.x + 35} y={top + 30} className="bmeta">
              {clock(b.start)} · {b.end - b.start} s · {b.builders.length}
              {" 👷"}
            </text>
          </g>
        );
      })}

      {/* Ramas de los aldeanos */}
      {branches.map(({ b, parts, chevrons, badges }) => {
        const on = selV === b.villager;
        return (
          <g
            key={b.villager}
            className={`branch${on ? " selected" : ""}${selV !== null && !on ? " dim" : ""}`}
            onClick={(e) => {
              e.stopPropagation();
              if (!dragMoved()) onSelect({ kind: "villager", id: b.villager });
            }}
          >
            {parts.map((p, i) => (
              <g key={i}>
                <path d={p.d} className="hit" />
                <path d={p.d} className={`seg ${p.cls}`} markerEnd={p.arrow ? `url(#arr-${on ? "sel" : "link"})` : undefined}>
                  {p.title && <title>{p.title}</title>}
                </path>
              </g>
            ))}
            {chevrons.map((c, i) => (
              <path key={i} d={`M ${c.x - 4} ${c.y + 3} L ${c.x} ${c.y - 2} L ${c.x + 4} ${c.y + 3}`} className={`chev ${c.cls}`} />
            ))}
            {badges.map((g, i) => (
              <g key={i} className={`badge ${g.cls}`}>
                <circle cx={g.x} cy={g.y} r={BADGE} />
                <image href={g.icon} x={g.x - 7} y={g.y - 7} width={14} height={14} />
              </g>
            ))}
            {/* Nacimiento: ícono de aldeano sobre el CU */}
            <g className="birth">
              <circle cx={bornX} cy={y(b.time)} r={on ? 10 : 8} />
              <image href="/img/unit/83.png" x={bornX - 6} y={y(b.time) - 6} width={12} height={12} />
            </g>
            <text x={TC_X - 6} y={y(b.time) + 4} textAnchor="end" className="vlabel">
              a{b.villager}
              {b.tc !== null && b.tc > 0 ? ` · ${L.tc}${b.tc + 1}` : ""}
            </text>
          </g>
        );
      })}

      {/* Ramas alternativas del aldeano seleccionado */}
      {selNode &&
        selNode.alternatives.slice(0, 5).map((a, i) => {
          const x0 = bornX;
          const y0 = y(selNode.time);
          const x1 = layout.poolX[a.pool] ?? layout.right;
          const y1 = y(selNode.time) - 22 - i * 18;
          const cls = a.delta === null ? "fail" : a.delta === 0 ? "same" : a.delta > 0 ? "worse" : "better";
          const txt =
            (a.delta === null ? L.impossible : a.delta === 0 ? `= ${L.same}` : `${a.delta > 0 ? "+" : ""}${a.delta} s`) +
            (a.tc_idle > 0 ? ` · ${L.tcStopped} ${a.tc_idle} s` : "");
          return (
            <g key={i} className={`alt ${cls}`}>
              <path d={`M ${x0} ${y0} C ${x0 + 40} ${y1}, ${x1 - 40} ${y1}, ${x1} ${y1}`} markerEnd={`url(#arr-link)`} />
              <image href={labels.poolIcon(a.pool)} x={x1 + 4} y={y1 - 8} width={16} height={16} />
              <text x={x1 + 24} y={y1 + 4}>
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
        const step = Math.max(1, Math.floor(s.length / 400));
        let d = "";
        for (let t = 0; t < s.length; t += step) d += `${t === 0 ? "M" : "L"} ${(x0 + (s[t] / maxRes[i]) * STRIP_W).toFixed(1)} ${y(t).toFixed(1)} `;
        return (
          <g key={r} className={`strip ${r}`}>
            <line x1={x0} x2={x0} y1={0} y2={layout.height} className="base" />
            <path d={d} />
            <text x={x0 + STRIP_W} y={layout.height + 15} textAnchor="end" className="max">
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
        <rect x={layout.left + 4} y={layout.height + 30} width={contentRight - layout.left - 8} height={66} rx={8} />
        <image href="/img/unit/83.png" x={layout.left + 16} y={layout.height + 44} width={28} height={28} />
        <text x={layout.left + 54} y={layout.height + 56} className="start-title">
          {L.start} · 0:00 · {labels.t("ages.Dark")}
        </text>
        <text x={layout.left + 54} y={layout.height + 78}>
          {L.startDesc} · {labels.t("resources.food")} {Math.round(timeline.series.food[0])} · {labels.t("resources.wood")}{" "}
          {Math.round(timeline.series.wood[0])} · {labels.t("resources.gold")} {Math.round(timeline.series.gold[0])} ·{" "}
          {labels.t("resources.stone")} {Math.round(timeline.series.stone[0])} · {L.population} {timeline.series.pop[0]}/
          {timeline.series.pop_cap[0]}
        </text>
      </g>
    </g>
  );
});

// ---------- capa del cursor (cambia a menudo, es barata) ----------

const Cursor = memo(function Cursor({ t, layout, timeline }: { t: number; layout: MapLayout; timeline: Timeline }) {
  const { y } = layout;
  const stripsX = layout.right + 10;
  const contentRight = stripsX + RESOURCES.length * (STRIP_W + STRIP_GAP);
  return (
    <g transform={`translate(0 ${TOP})`} className="cursor">
      <line x1={layout.left} x2={contentRight} y1={y(t)} y2={y(t)} />
      <rect x={layout.left + 2} y={y(t) - 20} width={44} height={16} rx={4} />
      <text x={layout.left + 24} y={y(t) - 8} textAnchor="middle">
        {clock(t)}
      </text>
      {RESOURCES.map((r, i) => {
        const s = timeline.series[r];
        const v = s[Math.min(t, s.length - 1)] ?? 0;
        const max = Math.max(1, ...s);
        const x0 = stripsX + i * (STRIP_W + STRIP_GAP);
        return (
          <g key={r} className={r}>
            <circle cx={x0 + (v / max) * STRIP_W} cy={y(t)} r={4} />
            <text x={x0 + STRIP_W} y={y(t) - 6} textAnchor="end" className="val">
              {Math.round(v)}
            </text>
          </g>
        );
      })}
    </g>
  );
});

// ---------- contenedor: zoom, paneo y teclado ----------

export default function EconomyMap({ layout, timeline, tree, labels, selected, onSelect, cursor, onCursor, goalTime, civIcon }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const layer = useRef<SVGGElement>(null);
  const view = useRef<View>({ x: 0, y: 0, k: 1 });
  const frame = useRef(0);
  const drag = useRef<{ x: number; y: number; vx: number; vy: number; moved: boolean } | null>(null);
  const lastCursor = useRef<number | null>(null);
  const L = labels.ui;

  const stripsX = layout.right + 10;
  const contentRight = stripsX + RESOURCES.length * (STRIP_W + STRIP_GAP);
  const width = contentRight - layout.left;
  const height = TOP + layout.height + BOTTOM;

  // El zoom y el paneo solo cambian la transformación (sin redibujar React).
  const apply = useCallback(() => {
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      const v = view.current;
      layer.current?.setAttribute("transform", `translate(${v.x} ${v.y}) scale(${v.k})`);
    });
  }, []);
  const setView = useCallback(
    (f: (v: View) => View) => {
      view.current = f(view.current);
      apply();
    },
    [apply],
  );
  const zoomAt = useCallback(
    (mx: number, my: number, f: number) =>
      setView((v) => {
        const k = Math.min(MAX_K, Math.max(MIN_K, v.k * f));
        return { k, x: mx - ((mx - v.x) * k) / v.k, y: my - ((my - v.y) * k) / v.k };
      }),
    [setView],
  );
  const home = useCallback(() => {
    const el = box.current;
    if (!el) return;
    const k = Math.min(1.1, Math.max(0.8, el.clientWidth / width));
    setView(() => ({ k, x: -layout.left * k + Math.max(0, (el.clientWidth - width * k) / 2), y: el.clientHeight - height * k }));
  }, [layout, width, height, setView]);
  const toEnd = useCallback(() => setView((v) => ({ ...v, y: 20 })), [setView]);

  // Vista inicial: el inicio (abajo) a la vista.
  useEffect(home, [home]);

  // Rueda: zoom alrededor del cursor; Mayús+rueda: subir/bajar en el tiempo.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const r = el.getBoundingClientRect();
      if (e.shiftKey) setView((v) => ({ ...v, y: v.y - (e.deltaY || e.deltaX) }));
      else zoomAt(e.clientX - r.left, e.clientY - r.top, Math.exp(-e.deltaY * 0.0015));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [setView, zoomAt]);

  // Teclado del mapa: +/- zoom, flechas desplazar, Inicio/Fin, Esc.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA" || e.ctrlKey || e.metaKey || e.altKey) return;
      const el = box.current;
      if (!el) return;
      const cx = el.clientWidth / 2;
      const cy = el.clientHeight / 2;
      const step = e.shiftKey ? 240 : 80;
      const actions: Record<string, () => void> = {
        "+": () => zoomAt(cx, cy, 1.25),
        "=": () => zoomAt(cx, cy, 1.25),
        "-": () => zoomAt(cx, cy, 0.8),
        ArrowUp: () => setView((v) => ({ ...v, y: v.y + step })),
        ArrowDown: () => setView((v) => ({ ...v, y: v.y - step })),
        ArrowLeft: () => setView((v) => ({ ...v, x: v.x + step })),
        ArrowRight: () => setView((v) => ({ ...v, x: v.x - step })),
        Home: home,
        End: toEnd,
        Escape: () => onSelect(null),
      };
      const act = actions[e.key];
      if (act) {
        e.preventDefault();
        act();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [zoomAt, setView, home, toEnd, onSelect]);

  const dragMoved = useCallback(() => !!drag.current?.moved, []);
  const selV = selected?.kind === "villager" ? selected.id : null;
  const selB = selected?.kind === "building" ? selected.placement : null;
  const selNode = useMemo(() => (selV !== null ? tree.find((n) => n.villager === selV) : undefined), [selV, tree]);

  // Cursor: solo cuando cambia el segundo, como mucho una vez por fotograma.
  const cursorFrame = useRef(0);
  const moveCursor = (clientY: number) => {
    cancelAnimationFrame(cursorFrame.current);
    cursorFrame.current = requestAnimationFrame(() => {
      const r = box.current!.getBoundingClientRect();
      const v = view.current;
      const cyContent = (clientY - r.top - v.y) / v.k - TOP;
      const t = Math.round(layout.end - cyContent / PX_PER_SECOND);
      const next = t >= 0 && t <= layout.end ? t : null;
      if (next !== lastCursor.current) {
        lastCursor.current = next;
        onCursor(next);
      }
    });
  };

  return (
    <div
      ref={box}
      className="emap"
      onMouseDown={(e) => (drag.current = { x: e.clientX, y: e.clientY, vx: view.current.x, vy: view.current.y, moved: false })}
      onMouseMove={(e) => {
        const d = drag.current;
        if (d) {
          if (Math.abs(e.clientX - d.x) + Math.abs(e.clientY - d.y) > 3) d.moved = true;
          setView((v) => ({ ...v, x: d.vx + e.clientX - d.x, y: d.vy + e.clientY - d.y }));
          return;
        }
        moveCursor(e.clientY);
      }}
      onMouseUp={() => setTimeout(() => (drag.current = null))}
      onMouseLeave={() => (drag.current = null)}
      onDoubleClick={(e) => {
        const r = box.current!.getBoundingClientRect();
        zoomAt(e.clientX - r.left, e.clientY - r.top, 1.6);
      }}
    >
      <div className="zoom">
        <button title="+" onClick={() => zoomAt(box.current!.clientWidth / 2, box.current!.clientHeight / 2, 1.25)}>
          +
        </button>
        <button title="−" onClick={() => zoomAt(box.current!.clientWidth / 2, box.current!.clientHeight / 2, 0.8)}>
          −
        </button>
        <button title={L.home} onClick={home}>
          ⌂
        </button>
      </div>
      <svg width="100%" height="100%">
        <g ref={layer}>
          <Content
            layout={layout}
            timeline={timeline}
            labels={labels}
            selV={selV}
            selB={selB}
            selNode={selNode}
            goalTime={goalTime}
            civIcon={civIcon}
            onSelect={onSelect}
            dragMoved={dragMoved}
          />
          {cursor !== null && <Cursor t={cursor} layout={layout} timeline={timeline} />}
        </g>
      </svg>
    </div>
  );
}

/** Leyenda del mapa (va en la barra de pestañas, fuera del lienzo). */
export function MapLegend({ labels }: { labels: Labels }) {
  const L = labels.ui;
  return (
    <div className="map-legend">
      {RESOURCES.map((r) => (
        <span key={r}>
          <i className={`swatch ${r}`} />
          <img src={`/img/resource/${r}.png`} alt="" className="icon small" />
          {labels.t(`resources.${r}`)}
        </span>
      ))}
      <span>
        <i className="swatch build" />
        {L.build}
      </span>
      <span className="muted">↑ {L.timeUp}</span>
      </div>
  );
}
