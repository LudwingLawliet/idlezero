import { AppInfo, MapInfo } from "./types";
import { UiText, ui } from "./i18n";

/** Textos e íconos del idioma actual, a partir de AppInfo. */
export interface Labels {
  ui: UiText;
  t: (key: string) => string;
  pool: (id: string) => string;
  poolIcon: (id: string) => string;
  tech: (id: string) => string;
  building: (id: string) => string;
}

export function makeLabels(info: AppInfo, lang: string, map: MapInfo | undefined): Labels {
  const t = (key: string) => info.labels[key] ?? key;
  return {
    ui: ui(lang),
    t,
    pool: (id) => t(`pools.${id}`),
    poolIcon: (id) => {
      if (id === "farms") return "/img/building/farm.png";
      const r = map?.pools.find((p) => p.id === id)?.resource ?? "food";
      return `/img/resource/${r}.png`;
    },
    tech: (id) => info.techs[id] ?? id,
    building: (id) => info.buildings[id] ?? id,
  };
}
