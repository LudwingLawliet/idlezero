import { AppInfo, Entity, MapInfo, Task } from "./types";
import { UiText, ui } from "./i18n";

/** Textos e íconos del idioma actual, a partir de AppInfo. */
export interface Labels {
  ui: UiText;
  t: (key: string) => string;
  pool: (id: string) => string;
  poolIcon: (id: string) => string;
  /** Recurso que da un pool (food, wood, gold, stone). */
  poolResource: (id: string) => "food" | "wood" | "gold" | "stone";
  /** Nombre e ícono de una entidad por su clave (nombre corto o id). */
  name: (kind: Kind, key: string) => string;
  icon: (kind: Kind, key: string) => string;
  task: (t: Task) => string;
}

export type Kind = "unit" | "tech" | "building";

const FARM_ID = 50;
// Íconos de comida dibujados para IdleZero (public/img/food): aoe2techtree no los trae.
const FOOD_ICONS = ["sheep", "boar", "berries", "deer", "fish"];

/** Ícono propio de un pool de comida, por su id (p. ej. "boar", "shore_fish"). */
export function foodIcon(pool: string): string | null {
  const k = FOOD_ICONS.find((f) => pool === f || pool.includes(f));
  return k ? `/img/food/${k}.svg` : null;
}

export function makeLabels(info: AppInfo, lang: string, map: MapInfo | undefined): Labels {
  const t = (key: string) => info.labels[key] ?? key;
  const list = (k: Kind): Entity[] =>
    k === "unit" ? info.catalog.units : k === "tech" ? info.catalog.techs : info.catalog.buildings;
  const find = (k: Kind, key: string) => list(k).find((e) => e.key === key || String(e.id) === key);
  const u = ui(lang);
  const name = (k: Kind, key: string) => find(k, key)?.name ?? key;
  const pool = (id: string) => t(`pools.${id}`);
  return {
    ui: u,
    t,
    pool,
    poolResource: (id) => (id === "farms" ? "food" : map?.pools.find((p) => p.id === id)?.resource ?? "food"),
    poolIcon: (id) => {
      if (id === "farms") return `/img/building/${FARM_ID}.png`;
      const own = foodIcon(id);
      if (own) return own;
      const r = map?.pools.find((p) => p.id === id)?.resource ?? "food";
      return `/img/resource/${r}.png`;
    },
    name,
    icon: (k, key) => {
      const e = find(k, key);
      return e ? `/img/${k}/${e.id}.png` : "/img/resource/food.png";
    },
    task: (task) => {
      switch (task.kind) {
        case "gather":
          return pool(task.pool);
        case "lure":
          return `${u.lure} ${pool(task.pool)}`;
        case "build":
          return `${u.build}: ${name("building", task.building)}`;
        case "wait":
          return `${u.waiting} ${name("building", task.building)}`;
        default: {
          const why = (u as unknown as Record<string, string>)[`idle_${task.reason}`];
          return why ? `${u.idle}: ${why}` : u.idle;
        }
      }
    },
  };
}
