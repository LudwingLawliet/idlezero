import { AppInfo, Entity, MapInfo, Task } from "./types";
import { UiText, ui } from "./i18n";

/** Textos e íconos del idioma actual, a partir de AppInfo. */
export interface Labels {
  ui: UiText;
  t: (key: string) => string;
  pool: (id: string) => string;
  poolIcon: (id: string) => string;
  /** Nombre e ícono de una entidad por su clave (nombre corto o id). */
  name: (kind: Kind, key: string) => string;
  icon: (kind: Kind, key: string) => string;
  task: (t: Task) => string;
}

export type Kind = "unit" | "tech" | "building";

const FARM_ID = 50;

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
    poolIcon: (id) => {
      if (id === "farms") return `/img/building/${FARM_ID}.png`;
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
        default:
          return u.idle;
      }
    },
  };
}
