// Convierte una línea de tiempo en el "mapa de economía": tiempo de abajo
// (0:00) hacia arriba, CU en el centro, construcciones a la izquierda y
// recolección a la derecha, una rama por aldeano.

import { Age, MapInfo, Task, Timeline } from "./types";

export const PX_PER_SECOND = 1.6;
const SLOT = 8; // separación entre ramas dentro de una columna
const COL_GAP = 52;
const BUILD_W = 128; // ancho de un edificio en la zona izquierda
const BUILD_GAP = 10;
export const TC_X = 0;
export const TC_W = 34;

export interface Segment {
  villager: number;
  start: number;
  end: number;
  task: Task;
  /// Posición horizontal de la rama en ese tramo.
  x: number;
}

export interface BuildingBox {
  placement: number;
  building: string;
  builders: number[];
  start: number;
  end: number;
  lane: number;
  x: number;
}

export interface Column {
  pool: string;
  x: number;
  width: number;
}

export interface Birth {
  villager: number;
  time: number;
  tc: number | null;
}

export interface AgeMark {
  age: Age;
  click?: number;
  reached: number;
}

export interface MapLayout {
  end: number;
  height: number;
  y: (t: number) => number;
  columns: Column[];
  buildings: BuildingBox[];
  segments: Segment[];
  births: Birth[];
  ages: AgeMark[];
  tcCount: number;
  left: number;
  right: number;
  /// Columna (x) de cada pool, para ramas alternativas.
  poolX: Record<string, number>;
}

/** Asigna carriles a intervalos sin que se pisen (coloreo de intervalos). */
function lanes<T extends { start: number; end: number }>(items: T[]): number[] {
  const ends: number[] = [];
  return items.map((it) => {
    let lane = ends.findIndex((e) => e <= it.start);
    if (lane < 0) {
      lane = ends.length;
      ends.push(0);
    }
    ends[lane] = it.end + 1;
    return lane;
  });
}

const gatherPool = (t: Task): string | null => {
  if (t.kind === "gather" || t.kind === "lure") return t.pool;
  if (t.kind === "build" && t.building === "farm") return "farms";
  return null;
};

export function layoutMap(tl: Timeline, map: MapInfo | undefined): MapLayout {
  const end = Math.max(tl.summary.end_time, 1);
  const y = (t: number) => (end - t) * PX_PER_SECOND;

  // Tramos de cada aldeano a partir de los cambios de tarea.
  const raw = new Map<number, { start: number; task: Task }[]>();
  for (const { time, event } of tl.events) {
    if (event.type !== "task_changed") continue;
    const list = raw.get(event.villager) ?? [];
    list.push({ start: time, task: event.task });
    raw.set(event.villager, list);
  }
  type Raw = { villager: number; start: number; end: number; task: Task };
  const segs: Raw[] = [];
  for (const [villager, list] of raw) {
    list.forEach((s, i) => segs.push({ villager, start: s.start, end: list[i + 1]?.start ?? end, task: s.task }));
  }

  // Columnas de recolección: en el orden del mapa, solo las que se usan.
  const used = new Set(segs.map((s) => gatherPool(s.task)).filter((p): p is string => !!p));
  const order = (map?.pools.map((p) => p.id) ?? []).filter((p) => used.has(p));
  for (const p of used) if (!order.includes(p)) order.push(p);

  // Ramas dentro de cada columna: carril por aldeano y tramo continuo.
  const columns: Column[] = [];
  const slotOf = new Map<string, number>(); // `${villager}:${segIndex}` -> slot
  let x = TC_X + TC_W + 46;
  const poolX: Record<string, number> = {};
  for (const pool of order) {
    const inPool = segs
      .map((s, i) => ({ ...s, i }))
      .filter((s) => gatherPool(s.task) === pool)
      .sort((a, b) => a.start - b.start || a.villager - b.villager);
    const ls = lanes(inPool);
    inPool.forEach((s, k) => slotOf.set(`${s.i}`, ls[k]));
    const width = Math.max(1, ...ls.map((l) => l + 1)) * SLOT + 12;
    columns.push({ pool, x, width });
    poolX[pool] = x + 6;
    x += width + COL_GAP;
  }
  const right = x;

  // Edificios (sin granjas) a la izquierda, en carriles por tiempo.
  const builds = new Map<number, BuildingBox>();
  for (const { time, event } of tl.events) {
    if (event.type === "build_started" && event.building !== "farm") {
      builds.set(event.placement, {
        placement: event.placement,
        building: event.building,
        builders: event.builders,
        start: time,
        end: time,
        lane: 0,
        x: 0,
      });
    }
  }
  for (const s of segs) {
    if (s.task.kind === "build" && s.task.building !== "farm") {
      const b = builds.get(s.task.placement);
      if (b) {
        b.end = Math.max(b.end, s.end);
        if (!b.builders.includes(s.villager)) b.builders.push(s.villager);
      }
    }
  }
  const boxes = [...builds.values()].sort((a, b) => a.start - b.start || a.placement - b.placement);
  // Altura mínima para que quepa el texto (en segundos de mapa).
  const minSpan = 40 / PX_PER_SECOND;
  const bl = lanes(boxes.map((b) => ({ start: b.start, end: Math.max(b.end, b.start + minSpan) })));
  boxes.forEach((b, i) => {
    b.lane = bl[i];
    b.x = TC_X - 46 - (b.lane + 1) * (BUILD_W + BUILD_GAP);
  });
  const left = Math.min(TC_X - 80, ...boxes.map((b) => b.x)) - 70;

  const segments: Segment[] = segs.map((s, i) => {
    const pool = gatherPool(s.task);
    let sx: number;
    if (pool !== null) {
      const col = columns.find((c) => c.pool === pool)!;
      sx = col.x + 6 + (slotOf.get(`${i}`) ?? 0) * SLOT;
    } else if (s.task.kind === "build") {
      const b = builds.get(s.task.placement);
      sx = b ? b.x + BUILD_W - 10 - (b.builders.indexOf(s.villager) % 6) * 6 : TC_X - 30;
    } else {
      sx = TC_X - 22; // esperando recursos o sin tarea: junto al CU
    }
    return { ...s, x: sx };
  });

  const births: Birth[] = tl.villagers.map((v) => ({ villager: v.id, time: v.born, tc: v.born_tc }));
  const ages: AgeMark[] = (["Feudal", "Castle", "Imperial"] as Age[])
    .filter((a) => tl.summary.age_reached[a] !== undefined)
    .map((a) => ({ age: a, click: tl.summary.age_clicks[a], reached: tl.summary.age_reached[a]! }));
  const tcCount = Math.max(1, ...tl.series.tcs);

  return { end, height: y(0), y, columns, buildings: boxes, segments, births, ages, tcCount, left, right, poolX };
}

/** Qué hace cada aldeano en el segundo `t`. */
export function tasksAt(layout: MapLayout, t: number): Map<number, Segment> {
  const out = new Map<number, Segment>();
  for (const s of layout.segments) if (s.start <= t && t < s.end) out.set(s.villager, s);
  return out;
}
