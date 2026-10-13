// Tipos de los datos que envía el motor (Rust, serde). Mantener en sincronía
// con crates/aoe2-sim/src/timeline.rs, crates/aoe2-opt/src/search.rs y
// app/src-tauri/src/lib.rs.

export type Age = "Dark" | "Feudal" | "Castle" | "Imperial";
export type Resource = "food" | "wood" | "gold" | "stone";
export const RESOURCES: Resource[] = ["food", "wood", "gold", "stone"];

export interface Named {
  id: string;
  name: string;
}

export interface PoolInfo {
  id: string;
  resource: Resource;
  amount: number | null;
}

export interface MapInfo extends Named {
  test: boolean;
  pools: PoolInfo[];
}

export interface Cost {
  food: number;
  wood: number;
  gold: number;
  stone: number;
}

export interface Entity {
  key: string;
  id: number;
  name: string;
  age: Age;
  /** Solo unidades: infantry, archers, cav_archers, cavalry, siege, monks, ships, other. */
  category?: string;
  /** Solo unidades: primera unidad de su línea (para ordenar). */
  line?: number;
  cost: Cost;
  /** Segundos de entrenamiento, investigación o construcción. */
  time: number;
  hp: number;
}

export interface UnitRef {
  id: number;
  key: string;
  name: string;
  age: Age;
  bonus?: number;
}

export interface UnitCard {
  id: number;
  name: string;
  age: Age;
  category: string;
  produced_at: string | null;
  cost: Cost;
  train_time: number;
  hp: number;
  speed: number;
  attack: number;
  melee_armor: number;
  pierce_armor: number;
  range: number;
  reload: number;
  strong_vs: { class: string; amount: number }[];
  weak_vs: UnitRef[];
  line: UnitRef[];
  upgrades: { id: number; key: string; name: string; age: Age; recommended: boolean; cost: Cost }[];
}

export interface Goal {
  id: string;
  category?: string | null;
  age?: Age | null;
  villagers?: number | null;
  units: { unit: string; count: number }[];
  techs: string[];
  buildings: { building: string; count: number }[];
  allow_tc_idle: boolean;
}

export interface AppInfo {
  version: string;
  source_commit: string;
  maps: MapInfo[];
  presets: { id: string; name: string; description: string; category: string; category_name: string; goal: Goal }[];
  langs: Named[];
  labels: Record<string, string>;
  catalog: { units: Entity[]; techs: Entity[]; buildings: Entity[] };
  tips: Tip[];
  benchmarks: Benchmark[];
  benchmarks_source: string;
}

/** Aldeanos de referencia para una edad objetivo. */
export interface Benchmark {
  age: Age;
  max: number;
  high: number;
}

export interface Tip {
  id: string;
  /** Momentos de la orden a los que aplica: start, villager, idle, house, lure, farm, age, drop_site, scout, groups. */
  context: string[];
  keys: string[];
  text: string;
}

export interface Choice {
  pool: string;
  click?: string;
  stop?: boolean;
  hard?: boolean;
  build?: string;
  builders?: number;
}

export interface Snapshot {
  time: number;
  age: Age;
  food: number;
  wood: number;
  gold: number;
  stone: number;
  pop: number;
  pop_cap: number;
  villagers_by_task: Record<string, number>;
}

export interface Alternative extends Choice {
  time: number | null;
  delta: number | null;
  /** Segundos que el CU queda parado con esta alternativa. */
  tc_idle: number;
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

export type Task =
  | { kind: "gather"; pool: string }
  | { kind: "lure"; pool: string }
  | { kind: "build"; building: string; placement: number }
  | { kind: "wait"; building: string }
  | { kind: "idle"; reason: IdleReason };

export type IdleReason = "exhausted" | "not_arrived" | "waiting_lure" | "no_wood_for_farm" | "other";

export type SimEvent =
  | { type: "villager_created"; villager: number; assignment: string }
  | { type: "build_started"; building: string; builders: number[]; placement: number }
  | { type: "build_finished"; building: string }
  | { type: "research_started"; tech: string }
  | { type: "research_finished"; tech: string }
  | { type: "age_reached"; age: Age }
  | { type: "task_changed"; villager: number; task: Task }
  | { type: "spent"; what: string; cost: Record<Resource, number>; stock: number[] }
  | { type: "tc_idle"; cause: string; seconds: number; tc: number }
  | { type: "unit_trained"; unit: string; count: number }
  | { type: "pool_exhausted"; pool: string }
  | { type: "lured"; source: string; villager: number }
  | { type: "moved" | "decision" | "step_blocked" };

export interface Series {
  food: number[];
  wood: number[];
  gold: number[];
  stone: number[];
  pop: number[];
  pop_cap: number[];
  tc_busy: number[];
  tcs: number[];
}

export interface VillagerInfo {
  id: number;
  born: number;
  born_tc: number | null;
  gathered: [number, number, number, number];
}

export interface Summary {
  end_time: number;
  villagers: number;
  age_clicks: Partial<Record<Age, number>>;
  age_reached: Partial<Record<Age, number>>;
  tc_idle_seconds: Record<string, number>;
  gathered: Record<Resource, number>;
  spent: Record<Resource, number>;
  buildings: [string, number][];
  /** Al terminar: aldeanos por pool ("build", "idle"). */
  final_villagers: Record<string, number>;
  /** Al terminar: ingreso por minuto con ese reparto. */
  final_income_per_min: Record<Resource, number>;
}

export interface Timeline {
  events: { time: number; event: SimEvent }[];
  series: Series;
  villagers: VillagerInfo[];
  summary: Summary;
}

export interface OptResult {
  goal: Goal;
  best: Plan;
  order_toml: string;
  timeline: Timeline;
  tree: TreeNode[];
  plans: Plan[];
  simulations: number;
  relaxed: boolean;
  /** Con civilización: tiempo del mismo objetivo en el escenario neutro. */
  neutral_time: number | null;
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

export interface PlanView {
  time: number | null;
  timeline: Timeline;
  events: TextEvent[];
}

export interface WhatIf {
  builders: number;
  build_seconds: number | null;
  goal_time: number | null;
}

export interface CalcResult {
  gathered: number;
  carrying: number;
  rate_per_villager: number;
  gather_rate: number;
  carry_capacity: number;
  distance_tiles: number;
  trip_seconds: number;
  walk_seconds_per_trip: number;
  trips: number;
  resource: Resource;
  depleted: boolean;
}

export const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.round(s) % 60).padStart(2, "0")}`;

export interface CivView {
  id: string;
  name: string;
  description: string;
  icon: string;
  /** Bonos que el simulador aplica (vacío = solo árbol tecnológico). */
  simulated: string[];
  /** Bonos del texto oficial que no afectan a la economía simulada. */
  not_simulated: string[];
  units: number[];
  techs: number[];
  buildings: number[];
}
