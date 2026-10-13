import { Age, OptimizeView, Plan, Snapshot, clock } from "./types";
import { Labels } from "./labels";

const AGES: Age[] = ["Feudal", "Castle", "Imperial"];
const RES = ["food", "wood", "gold", "stone"] as const;

function Resources({ s, labels }: { s: { food: number; wood: number; gold: number; stone: number }; labels: Labels }) {
  return (
    <div className="resources">
      {RES.map((r) => (
        <span key={r} title={labels.t(`resources.${r}`)}>
          <img src={`/img/resource/${r}.png`} alt="" className="icon small" />
          {Math.round(s[r])}
        </span>
      ))}
    </div>
  );
}

function StateView({ s, labels }: { s: Snapshot; labels: Labels }) {
  const tasks = Object.entries(s.villagers_by_task).sort((a, b) => b[1] - a[1]);
  const max = Math.max(1, ...tasks.map(([, n]) => n));
  return (
    <>
      <div className="kv">
        <span>{labels.t(`ages.${s.age}`)}</span>
        <span>
          {labels.ui.population} {s.pop}/{s.pop_cap}
        </span>
      </div>
      <Resources s={s} labels={labels} />
      <h4>{labels.ui.byTask}</h4>
      {tasks.map(([task, n]) => (
        <div className="bar" key={task}>
          <img src={labels.poolIcon(task)} alt="" className="icon small" />
          <span className="bar-label">{labels.pool(task)}</span>
          <span className="bar-track">
            <span className="bar-fill" style={{ width: `${(n / max) * 100}%` }} />
          </span>
          <span className="bar-n">{n}</span>
        </div>
      ))}
    </>
  );
}

export default function SidePanel({
  view,
  labels,
  goalName,
  selected,
  plan,
  onPlan,
}: {
  view: OptimizeView;
  labels: Labels;
  goalName: string;
  selected: number | null;
  plan: Plan;
  onPlan: (p: Plan) => void;
}) {
  const r = view.result;
  const s = r.timeline.summary;
  const idle = Object.entries(s.tc_idle_seconds).filter(([c]) => c !== "no_order");
  const idleTotal = idle.reduce((a, [, n]) => a + n, 0);
  const houses = s.buildings.filter(([b]) => b === "house");
  const farms = s.buildings.filter(([b]) => b === "farm").length;
  const node = selected !== null ? r.tree[selected] : null;
  const best = r.best.time;

  return (
    <aside className="side">
      <section>
        <h3>{labels.ui.summary}</h3>
        <div className="goal-name">{goalName}</div>
        {r.relaxed && <p className="notice">{labels.ui.relaxed}</p>}
        <div className="total">
          <span className="muted">{labels.ui.totalTime}</span>
          <strong>{best !== null ? clock(best) : "—"}</strong>
        </div>
        <div className="kv">
          <span>{labels.ui.villagers}</span>
          <span>{s.villagers}</span>
        </div>
        <div className="kv">
          <span>{labels.ui.tcIdle}</span>
          <span className={idleTotal === 0 ? "good" : "warn"}>
            {idleTotal} s{idle.length > 0 && idleTotal > 0 ? ` (${idle.map(([c, n]) => `${labels.t(`idle.${c}`)} ${n} s`).join(", ")})` : " ✓"}
          </span>
        </div>
        {AGES.filter((a) => s.age_reached[a] !== undefined).map((a) => (
          <div className="kv" key={a}>
            <span>
              <img src={`/img/age/${a}.png`} alt="" className="icon small" /> {labels.t(`ages.${a}`)}
            </span>
            <span>
              {labels.ui.ageClick} {clock(s.age_clicks[a] ?? 0)} · {labels.ui.ageReached} {clock(s.age_reached[a]!)}
            </span>
          </div>
        ))}
        <div className="kv">
          <span>
            <img src="/img/building/house.png" alt="" className="icon small" /> {labels.ui.houses} ({houses.length})
          </span>
          <span className="small-list">{houses.map(([, t]) => clock(t)).join(" · ")}</span>
        </div>
        <div className="kv">
          <span>
            <img src="/img/building/farm.png" alt="" className="icon small" /> {labels.ui.farms}
          </span>
          <span>{farms}</span>
        </div>
        <h4>{labels.ui.gathered}</h4>
        <Resources s={s.gathered} labels={labels} />
        <p className="muted tiny">
          {r.simulations.toLocaleString()} {labels.ui.simulations} · {(view.millis / 1000).toFixed(1)} s · {labels.ui.approx}
        </p>
      </section>

      <section>
        <h3>{labels.ui.plans}</h3>
        <ul className="plans">
          {r.plans.map((p, i) => {
            const d = p.time !== null && best !== null ? p.time - best : null;
            return (
              <li key={i}>
                <button className={p === plan ? "active" : ""} onClick={() => onPlan(p)}>
                  <span>{i === 0 ? labels.ui.bestPlan : `#${i + 1}`}</span>
                  <span>{p.time !== null ? clock(p.time) : "—"}</span>
                  <span className="muted">{i === 0 ? "" : d !== null ? `+${d} s` : ""}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      <section>
        <h3>{labels.ui.node}</h3>
        {node ? (
          <>
            <div className="kv">
              <strong>
                {clock(node.time)} · {labels.ui.villager} {node.villager} → {labels.pool(node.pool)}
              </strong>
            </div>
            <StateView s={node.state} labels={labels} />
            <h4>{labels.ui.alternatives}</h4>
            <ul className="alts">
              {node.alternatives.map((a, i) => (
                <li key={i}>
                  <img src={labels.poolIcon(a.pool)} alt="" className="icon small" />
                  {labels.pool(a.pool)}
                  {a.click ? ` + ${labels.ui.click} ${labels.tech(a.click)}` : ""}
                  <span className={a.delta === null ? "fail" : a.delta === 0 ? "same" : a.delta <= 20 ? "warn" : "bad"}>
                    {a.delta === null ? labels.ui.impossible : a.delta === 0 ? `= ${labels.ui.same}` : `+${a.delta} s`}
                  </span>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <p className="muted">{labels.ui.noNode}</p>
        )}
      </section>
    </aside>
  );
}
