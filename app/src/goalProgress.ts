import { Labels, Kind } from "./labels";
import { AppInfo, Age, Entity, Goal, Resource, Timeline } from "./types";

/** Una unidad, edificio o tecnología concreta del objetivo (la 1.ª, la 2.ª…). */
export interface GoalItem {
  n: number;
  /** Segundo en que hubo recursos y se pagó. */
  paid: number | null;
  /** Segundo en que se terminó (entrenada, construida, investigada). */
  done: number | null;
  /** Reservas justo antes de pagar. */
  before: Record<Resource, number> | null;
  cost: Record<Resource, number> | null;
  /** Para filas de tecnologías: cuál. */
  name?: string;
  icon?: string;
}

export interface GoalRow {
  kind: Kind | "age";
  key: string;
  name: string;
  icon: string;
  count: number;
  items: GoalItem[];
}

const RES: Resource[] = ["food", "wood", "gold", "stone"];

/** Cuándo se pudo pagar y cuándo se terminó cada cosa del objetivo, a partir de los eventos. */
export function goalProgress(goal: Goal, timeline: Timeline, info: AppInfo, labels: Labels): GoalRow[] {
  const list = (k: Kind): Entity[] => (k === "unit" ? info.catalog.units : k === "tech" ? info.catalog.techs : info.catalog.buildings);
  const idOf = (k: Kind, key: string) => list(k).find((e) => e.key === key || String(e.id) === key)?.id;
  const same = (k: Kind, a: string, b: string) => a === b || (idOf(k, a) !== undefined && idOf(k, a) === idOf(k, b));

  const pays = (k: Kind, key: string) =>
    timeline.events.filter(({ event: e }) => e.type === "spent" && same(k, e.what, key)) as {
      time: number;
      event: { type: "spent"; what: string; cost: Record<Resource, number>; stock: number[] };
    }[];
  const item = (n: number, pay: ReturnType<typeof pays>[number] | undefined, done: number | null): GoalItem => ({
    n,
    paid: pay?.time ?? null,
    done,
    cost: pay?.event.cost ?? null,
    before: pay ? (Object.fromEntries(RES.map((r, i) => [r, Math.round(pay.event.stock[i] + (pay.event.cost[r] ?? 0))])) as Record<Resource, number>) : null,
  });

  const rows: GoalRow[] = [];
  for (const b of goal.buildings) {
    const p = pays("building", b.building);
    const fin = timeline.events.filter(({ event: e }) => e.type === "build_finished" && same("building", e.building, b.building)).map((e) => e.time);
    rows.push({
      kind: "building",
      key: b.building,
      name: labels.name("building", b.building),
      icon: labels.icon("building", b.building),
      count: b.count,
      items: Array.from({ length: b.count }, (_, i) => item(i + 1, p[i], fin[i] ?? null)),
    });
  }
  for (const u of goal.units) {
    const p = pays("unit", u.unit);
    const fin = new Map<number, number>();
    for (const { time, event: e } of timeline.events)
      if (e.type === "unit_trained" && same("unit", e.unit, u.unit) && !fin.has(e.count)) fin.set(e.count, time);
    rows.push({
      kind: "unit",
      key: u.unit,
      name: labels.name("unit", u.unit),
      icon: labels.icon("unit", u.unit),
      count: u.count,
      items: Array.from({ length: u.count }, (_, i) => item(i + 1, p[i], fin.get(i + 1) ?? null)),
    });
  }
  if (goal.techs.length) {
    rows.push({
      kind: "tech",
      key: "techs",
      name: labels.ui.addTech,
      icon: labels.icon("tech", goal.techs[0]),
      count: goal.techs.length,
      items: goal.techs.map((t, i) => {
        const fin = timeline.events.find(({ event: e }) => e.type === "research_finished" && same("tech", e.tech, t));
        return { ...item(i + 1, pays("tech", t)[0], fin?.time ?? null), name: labels.name("tech", t), icon: labels.icon("tech", t) };
      }),
    });
  }
  if (goal.age) {
    const s = timeline.summary;
    const ages: Age[] = ["Feudal", "Castle", "Imperial"];
    const upTo = ages.slice(0, ages.indexOf(goal.age) + 1);
    rows.unshift({
      kind: "age",
      key: "age",
      name: labels.t(`ages.${goal.age}`),
      icon: `/img/age/${goal.age}.png`,
      count: upTo.length,
      items: upTo.map((a, i) => ({
        n: i + 1,
        paid: s.age_clicks[a] ?? null,
        done: s.age_reached[a] ?? null,
        before: null,
        cost: null,
        name: labels.t(`ages.${a}`),
        icon: `/img/age/${a}.png`,
      })),
    });
  }
  return rows;
}

/** Primer pago de algo del objetivo (sin contar las edades intermedias). */
export function goalStart(rows: GoalRow[]): number | null {
  const t = rows.filter((r) => r.kind !== "age").flatMap((r) => r.items.map((i) => i.paid).filter((x): x is number => x !== null));
  return t.length ? Math.min(...t) : null;
}
