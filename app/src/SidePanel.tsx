import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Labels } from "./labels";
import { MapLayout, tasksAt } from "./layout";
import { Selection } from "./EconomyMap";
import { Age, Goal, OptResult, Plan, RESOURCES, Timeline, WhatIf, clock } from "./types";

const AGES: Age[] = ["Feudal", "Castle", "Imperial"];

function Stock({ values, labels }: { values: number[]; labels: Labels }) {
  return (
    <div className="resources">
      {RESOURCES.map((r, i) => (
        <span key={r} title={labels.t(`resources.${r}`)}>
          <img src={`/img/resource/${r}.png`} alt="" className="icon small" />
          {Math.round(values[i] ?? 0)}
        </span>
      ))}
    </div>
  );
}

function Summary({ result, timeline, labels, goalTime, millis }: { result: OptResult; timeline: Timeline; labels: Labels; goalTime: number | null; millis: number }) {
  const L = labels.ui;
  const s = timeline.summary;
  const idle = Object.entries(s.tc_idle_seconds).filter(([c]) => c !== "no_order");
  const idleTotal = idle.reduce((a, [, n]) => a + n, 0);
  const tcs = Math.max(1, ...timeline.series.tcs);
  return (
    <section>
      <h3>{L.summary}</h3>
      {result.relaxed && <p className="notice">{L.relaxed}</p>}
      <div className="total">
        <span className="muted">{L.totalTime}</span>
        <strong>{goalTime !== null ? clock(goalTime) : "—"}</strong>
      </div>
      <div className="kv">
        <span>{L.villagers}</span>
        <span>{timeline.villagers.length}</span>
      </div>
      <div className="kv">
        <span>{L.townCenters}</span>
        <span>{tcs}</span>
      </div>
      <div className="kv">
        <span>{L.tcIdle}</span>
        <span className={idleTotal === 0 ? "good" : "warn"}>
          {idleTotal} s{idleTotal > 0 ? ` (${idle.map(([c, n]) => `${labels.t(`idle.${c}`)} ${n} s`).join(", ")})` : " ✓"}
        </span>
      </div>
      {AGES.filter((a) => s.age_reached[a] !== undefined).map((a) => (
        <div className="kv" key={a}>
          <span>
            <img src={`/img/age/${a}.png`} alt="" className="icon small" /> {labels.t(`ages.${a}`)}
          </span>
          <span>
            {L.click} {clock(s.age_clicks[a] ?? 0)} · {L.reached} {clock(s.age_reached[a]!)}
          </span>
        </div>
      ))}
      <h4>{L.gathered}</h4>
      <Stock values={RESOURCES.map((r) => s.gathered[r])} labels={labels} />
      <h4>{L.spent}</h4>
      <Stock values={RESOURCES.map((r) => s.spent[r])} labels={labels} />
      <p className="muted tiny">
        {result.simulations.toLocaleString()} {L.simulations} · {(millis / 1000).toFixed(1)} s · {L.approx}
      </p>
    </section>
  );
}

function Moment({ t, timeline, layout, labels }: { t: number | null; timeline: Timeline; layout: MapLayout; labels: Labels }) {
  const L = labels.ui;
  if (t === null) {
    return (
      <section>
        <h3>{L.moment}</h3>
        <p className="muted">{L.momentHint}</p>
      </section>
    );
  }
  const s = timeline.series;
  const i = Math.min(t, s.food.length - 1);
  const counts = new Map<string, number>();
  for (const seg of tasksAt(layout, t).values()) {
    const key = seg.task.kind === "gather" || seg.task.kind === "lure" ? labels.pool(seg.task.pool) : labels.task(seg.task);
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  const rows = [...counts.entries()].sort((a, b) => b[1] - a[1]);
  const max = Math.max(1, ...rows.map(([, n]) => n));
  return (
    <section>
      <h3>
        {L.moment} · {clock(t)}
      </h3>
      <h4>{L.stock}</h4>
      <Stock values={[s.food[i], s.wood[i], s.gold[i], s.stone[i]]} labels={labels} />
      <div className="kv">
        <span>{L.population}</span>
        <span>
          {s.pop[i]}/{s.pop_cap[i]}
        </span>
      </div>
      <div className="kv">
        <span>{L.townCenters}</span>
        <span>
          {s.tc_busy[i]}/{s.tcs[i]} ✓
        </span>
      </div>
      <h4>{L.byTask}</h4>
      {rows.map(([task, n]) => (
        <div className="bar" key={task}>
          <span className="bar-label">{task}</span>
          <span className="bar-track">
            <span className="bar-fill" style={{ width: `${(n / max) * 100}%` }} />
          </span>
          <span className="bar-n">{n}</span>
        </div>
      ))}
    </section>
  );
}

function VillagerDetail({ id, result, timeline, layout, labels }: { id: number; result: OptResult; timeline: Timeline; layout: MapLayout; labels: Labels }) {
  const L = labels.ui;
  const v = timeline.villagers.find((x) => x.id === id);
  const segs = layout.segments.filter((s) => s.villager === id);
  const node = result.tree.find((n) => n.villager === id);
  if (!v) return null;
  return (
    <section>
      <h3>
        {L.villagerDetail} {id}
      </h3>
      <div className="kv">
        <span>{L.bornAt}</span>
        <span>
          {clock(v.born)} · {v.born_tc === null ? L.initial : `${L.bornFrom} ${v.born_tc + 1}`}
        </span>
      </div>
      <h4>{L.gathered}</h4>
      <Stock values={v.gathered} labels={labels} />
      <h4>{L.history}</h4>
      <ul className="history">
        {segs.map((s, i) => (
          <li key={i}>
            <span className="time">
              {clock(s.start)}–{clock(s.end)}
            </span>
            {labels.task(s.task)}
          </li>
        ))}
      </ul>
      {node && node.alternatives.length > 0 && (
        <>
          <h4>{L.alternatives}</h4>
          <ul className="alts">
            {node.alternatives.map((a, i) => (
              <li key={i}>
                <img src={labels.poolIcon(a.pool)} alt="" className="icon small" />
                {a.build ? `${labels.name("building", a.build)} → ` : ""}
                {labels.pool(a.pool)}
                {a.click ? ` + ${L.click}` : ""}
                <span className={a.delta === null ? "fail" : a.delta === 0 ? "same" : a.delta > 0 ? "bad" : "good"}>
                  {a.delta === null ? L.impossible : a.delta === 0 ? `= ${L.same}` : `${a.delta > 0 ? "+" : ""}${a.delta} s`}
                  {a.tc_idle > 0 && <small className="muted"> · {L.tcStopped} {a.tc_idle} s</small>}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

function BuildingDetail({ placement, layout, labels, map, goal, orderToml }: { placement: number; layout: MapLayout; labels: Labels; map: string; goal: Goal; orderToml: string }) {
  const L = labels.ui;
  const b = layout.buildings.find((x) => x.placement === placement);
  const [whatIf, setWhatIf] = useState<WhatIf[] | null>(null);
  useEffect(() => {
    setWhatIf(null);
    invoke<WhatIf[]>("builders_what_if", { map, goal, orderToml, placement }).then(setWhatIf, () => setWhatIf([]));
  }, [placement, map, goal, orderToml]);
  if (!b) return null;
  const base = whatIf?.find((w) => w.builders === b.builders.length)?.goal_time ?? null;
  return (
    <section>
      <h3>
        <img src={labels.icon("building", b.building)} alt="" className="icon small" /> {labels.name("building", b.building)}
      </h3>
      <div className="kv">
        <span>{L.builtBy}</span>
        <span>{b.builders.map((v) => `a${v}`).join(", ")}</span>
      </div>
      <div className="kv">
        <span>{L.duration}</span>
        <span>
          {clock(b.start)} → {clock(b.end)} ({b.end - b.start} s)
        </span>
      </div>
      <h4>{L.whatIf}</h4>
      {whatIf === null ? (
        <p className="muted">{L.calculating}</p>
      ) : (
        <table className="whatif">
          <tbody>
            {whatIf.map((w) => {
              const d = w.goal_time !== null && base !== null ? w.goal_time - base : null;
              return (
                <tr key={w.builders} className={w.builders === b.builders.length ? "current" : ""}>
                  <td>
                    {w.builders} {w.builders === 1 ? L.builder : L.builders}
                  </td>
                  <td>{w.build_seconds !== null ? `${w.build_seconds} s` : "—"}</td>
                  <td>
                    {L.goalAt} {w.goal_time !== null ? clock(w.goal_time) : "—"}
                  </td>
                  <td className={d === null ? "fail" : d === 0 ? "same" : d > 0 ? "bad" : "good"}>
                    {d === null ? "" : d === 0 ? "=" : `${d > 0 ? "+" : ""}${d} s`}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </section>
  );
}

export default function SidePanel(props: {
  result: OptResult;
  timeline: Timeline;
  layout: MapLayout;
  labels: Labels;
  goalTime: number | null;
  millis: number;
  cursor: number | null;
  selected: Selection;
  map: string;
  goal: Goal;
  plan: Plan;
  planBusy: boolean;
  onPlan: (p: Plan) => void;
}) {
  const { result, timeline, layout, labels, selected } = props;
  const L = labels.ui;
  const best = result.best.time;
  return (
    <aside className="side">
      {selected?.kind === "villager" && <VillagerDetail id={selected.id} result={result} timeline={timeline} layout={layout} labels={labels} />}
      {selected?.kind === "building" && props.plan === result.best && (
        <BuildingDetail placement={selected.placement} layout={layout} labels={labels} map={props.map} goal={props.goal} orderToml={result.order_toml} />
      )}
      {!selected && (
        <section>
          <p className="muted">{L.nothingSelected}</p>
        </section>
      )}
      <Summary result={result} timeline={timeline} labels={labels} goalTime={props.goalTime} millis={props.millis} />
      <section>
        <h3>{L.plans}</h3>
        <ul className="plans">
          {result.plans.map((p, i) => {
            const d = p.time !== null && best !== null ? p.time - best : null;
            return (
              <li key={i}>
                <button className={p === props.plan ? "active" : ""} onClick={() => props.onPlan(p)} disabled={props.planBusy}>
                  <span>{i === 0 ? L.bestPlan : `#${i + 1}`}</span>
                  <span>{p.time !== null ? clock(p.time) : "—"}</span>
                  <span className="muted">{i === 0 ? "" : d !== null ? `+${d} s` : ""}</span>
                </button>
              </li>
            );
          })}
        </ul>
        {props.planBusy && <p className="muted tiny">{L.loadingPlan}</p>}
      </section>
      <Moment t={props.cursor} timeline={timeline} layout={layout} labels={labels} />
    </aside>
  );
}
