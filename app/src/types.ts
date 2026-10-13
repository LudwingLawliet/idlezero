// Tipos de los datos que envía el motor (Rust, serde). Mantener en sincronía
// con crates/aoe2-opt/src/search.rs, crates/aoe2-sim/src/timeline.rs y
// app/src-tauri/src/lib.rs.

export type Age = "Dark" | "Feudal" | "Castle" | "Imperial";

export interface Named {
  id: string;
  name: string;
}

export interface MapInfo extends Named {
  pools: { id: string; resource: "food" | "wood" | "gold" | "stone" }[];
}

export interface AppInfo {
  version: string;
  source_commit: string;
  maps: MapInfo[];
  goals: Named[];
  langs: Named[];
  labels: Record<string, string>;
  buildings: Record<string, string>;
  techs: Record<string, string>;
  units: Record<string, string>;
}

export interface Choice {
  pool: string;
  click?: string;
  stop?: boolean;
  hard?: boolean;
}

export interface Snapshot {
  time: number;
  tick: number;
  age: Age;
  food: number;
  wood: number;
  gold: number;
  stone: number;
  pop: number;
  pop_cap: number;
  villagers_by_task: Record<string, number>;
  tc_idle_seconds: number;
}

export interface Alternative extends Choice {
  time: number | null;
  delta: number | null;
}

export interface TreeNode extends Choice {
  index: number;
  villager: number;
  time: number;
  state: Snapshot;
  alternatives: Alternative[];
}

export interface Plan {
  time: number | null;
  choices: Choice[];
}

export interface Summary {
  end_time: number;
  villagers: number;
  age_clicks: Partial<Record<Age, number>>;
  age_reached: Partial<Record<Age, number>>;
  tc_idle_seconds: Record<string, number>;
  villager_idle_seconds: number;
  gathered: { food: number; wood: number; gold: number; stone: number };
  buildings: [string, number][];
}

export interface OptResult {
  goal: { id: string };
  map: string;
  best: Plan;
  order_toml: string;
  timeline: { ticks: Snapshot[]; summary: Summary };
  tree: TreeNode[];
  plans: Plan[];
  simulations: number;
}

export interface TextEvent {
  time: number;
  text: string;
  kind: string;
}

export interface OptimizeView {
  result: OptResult;
  events: TextEvent[];
  millis: number;
}

export const clock = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
