import { useMemo, useState } from "react";
import { Labels } from "./labels";
import GoalTimeline from "./GoalTimeline";
import { goalProgress } from "./goalProgress";
import { MapLayout } from "./layout";
import { AppInfo, OptResult, RESOURCES, Resource, Timeline, Tip, clock } from "./types";

const W = 900; // ancho lógico de las gráficas (se escalan con el contenedor)
const H = 220;
const PAD = { l: 44, r: 92, t: 14, b: 26 };
const STEP = 5; // segundos entre muestras de la gráfica de aldeanos
type Layer = Resource | "build";
const LAYERS: Layer[] = [...RESOURCES, "build"];

/** Aldeanos por recurso cada STEP segundos, a partir de los tramos del mapa. */
function villagersByResource(layout: MapLayout, labels: Labels): { t: number; v: Record<Layer, number> }[] {
  const out: { t: number; v: Record<Layer, number> }[] = [];
  const times: number[] = [];
  for (let t = 0; t < layout.end; t += STEP) times.push(t);
  times.push(Math.max(0, layout.end - 1)); // último segundo con tramos activos
  for (const t of times) {
    const v: Record<Layer, number> = { food: 0, wood: 0, gold: 0, stone: 0, build: 0 };
    for (const s of layout.segments) {
      if (s.start > t || t >= s.end) continue;
      const k = s.task;
      if (k.kind === "gather" || k.kind === "lure") v[labels.poolResource(k.pool)] += 1;
      else if (k.kind === "build" && k.building === "farm") v.food += 1;
      else v.build += 1;
    }
    out.push({ t, v });
  }
  return out;
}

/** Contextos de consejos presentes en esta orden. */
export function planContexts(timeline: Timeline): Set<string> {
  const ctx = new Set(["start", "villager", "scout", "groups"]);
  for (const { event: e } of timeline.events) {
    if (e.type === "build_started") {
      if (e.building === "house") ctx.add("house");
      if (["lumber_camp", "mining_camp", "mill"].includes(e.building)) ctx.add("drop_site");
      if (e.building === "farm") ctx.add("farm");
    }
    if (e.type === "lured") ctx.add("lure");
    if (e.type === "research_started" && e.tech.endsWith("_age")) ctx.add("age");
    if (e.type === "task_changed" && e.task.kind === "idle") ctx.add("idle");
  }
  return ctx;
}

export function TipCard({ tip }: { tip: Tip }) {
  return (
    <div className="tip">
      {tip.keys.length > 0 && (
        <div className="tip-keys">
          {tip.keys.map((k) => (
            <kbd key={k}>{k}</kbd>
          ))}
        </div>
      )}
      <p>{tip.text}</p>
    </div>
  );
}

function Tile({ icon, label, value, note, tone }: { icon: string; label: string; value: string; note?: string; tone?: string }) {
  return (
    <div className="tile">
      <div className="tile-head">
        {icon.startsWith("/") ? <img src={icon} alt="" className="icon small" /> : <span className="tile-glyph">{icon}</span>}
        <span>{label}</span>
      </div>
      <div className={`tile-value${tone ? ` ${tone}` : ""}`}>{value}</div>
      {note && <div className="tile-note">{note}</div>}
    </div>
  );
}

export default function Dashboard({ result, timeline, layout, labels, info, goalTime }: { result: OptResult; timeline: Timeline; layout: MapLayout; labels: Labels; info: AppInfo; goalTime: number | null }) {
  const L = labels.ui;
  const s = timeline.summary;
  const end = Math.max(layout.end, 1);
  const x = (t: number) => PAD.l + (t / end) * (W - PAD.l - PAD.r);
  const [hover, setHover] = useState<number | null>(null);

  const stack = useMemo(() => villagersByResource(layout, labels), [layout, labels]);
  const goalRows = useMemo(() => goalProgress(result.goal, timeline, info, labels), [result.goal, timeline, info, labels]);
  const maxV = Math.max(1, ...stack.map((p) => LAYERS.reduce((a, k) => a + p.v[k], 0)));
  const yv = (n: number) => PAD.t + (1 - n / maxV) * (H - PAD.t - PAD.b);
  const areas = useMemo(() => {
    const paths: Record<Layer, string> = { food: "", wood: "", gold: "", stone: "", build: "" };
    const base = stack.map(() => 0);
    for (const k of LAYERS) {
      const top = stack.map((p, i) => base[i] + p.v[k]);
      const upper = stack.map((p, i) => `${x(p.t).toFixed(1)},${yv(top[i]).toFixed(1)}`);
      const lower = stack.map((p, i) => `${x(p.t).toFixed(1)},${yv(base[i]).toFixed(1)}`).reverse();
      paths[k] = `M ${upper.join(" L ")} L ${lower.join(" L ")} Z`;
      top.forEach((v, i) => (base[i] = v));
    }
    return paths;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stack, maxV, end]);

  const series = timeline.series;
  const maxR = Math.max(1, ...RESOURCES.flatMap((r) => series[r]));
  const yr = (n: number) => PAD.t + (1 - n / maxR) * (H - PAD.t - PAD.b);
  const lines = useMemo(() => {
    const step = Math.max(1, Math.floor(series.food.length / 450));
    const out: Record<Resource, string> = { food: "", wood: "", gold: "", stone: "" };
    for (const r of RESOURCES) {
      let d = "";
      for (let t = 0; t < series[r].length; t += step) d += `${t === 0 ? "M" : "L"} ${x(t).toFixed(1)} ${yr(series[r][t]).toFixed(1)} `;
      out[r] = d;
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [series, maxR, end]);

  const idle = Object.entries(s.tc_idle_seconds).filter(([c]) => c !== "no_order").reduce((a, [, n]) => a + n, 0);
  const tcs = Math.max(1, ...series.tcs);
  const sum = (o: Record<Resource, number>) => Math.round(RESOURCES.reduce((a, r) => a + o[r], 0));
  const ages = (["Feudal", "Castle", "Imperial"] as const).filter((a) => s.age_reached[a] !== undefined);
  const lastAge = ages[ages.length - 1];
  const milestones = useMemo(() => {
    const out: { t: number; icon: string; label: string }[] = [];
    for (const { time, event: e } of timeline.events) {
      if (e.type === "build_started" && e.building !== "farm")
        out.push({ t: time, icon: labels.icon("building", e.building), label: labels.name("building", e.building) });
      if (e.type === "age_reached") out.push({ t: time, icon: `/img/age/${e.age}.png`, label: labels.t(`ages.${e.age}`) });
    }
    return out;
  }, [timeline, labels]);
  const ctx = useMemo(() => planContexts(timeline), [timeline]);
  const tips = info.tips.filter((t) => t.context.some((c) => ctx.has(c)));

  const ticks = Array.from({ length: Math.floor(end / 60) + 1 }, (_, i) => i * 60).filter((t) => t % (end > 900 ? 120 : 60) === 0);
  const hoverAt = (e: React.MouseEvent<SVGSVGElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const px = ((e.clientX - r.left) / r.width) * W;
    const t = Math.round(((px - PAD.l) / (W - PAD.l - PAD.r)) * end);
    setHover(t >= 0 && t <= end ? t : null);
  };
  const hv = hover !== null ? stack[Math.min(stack.length - 1, Math.floor(hover / STEP))] : null;
  const layerName = (k: Layer) => (k === "build" ? L.build : labels.t(`resources.${k}`));

  const axis = (
    <g className="axis">
      {ticks.map((t) => (
        <g key={t}>
          <line x1={x(t)} x2={x(t)} y1={PAD.t} y2={H - PAD.b} />
          <text x={x(t)} y={H - 8} textAnchor="middle">
            {clock(t)}
          </text>
        </g>
      ))}
      {ages.map((a) => (
        <g key={a} className="age-mark">
          <line x1={x(s.age_reached[a]!)} x2={x(s.age_reached[a]!)} y1={PAD.t} y2={H - PAD.b} />
          <image href={`/img/age/${a}.png`} x={x(s.age_reached[a]!) - 8} y={0} width={16} height={16} />
        </g>
      ))}
    </g>
  );

  return (
    <div className="dash">
      <div className="tiles">
        <Tile
          icon={lastAge ? `/img/age/${lastAge}.png` : "🏁"}
          label={L.totalTime}
          value={goalTime !== null ? clock(goalTime) : "—"}
          note={ages.map((a) => `${labels.t(`ages.${a}`)} ${clock(s.age_reached[a]!)}`).join(" · ")}
        />
        <Tile icon="/img/unit/83.png" label={L.villagers} value={String(timeline.villagers.length)} note={`${L.townCenters}: ${tcs}`} />
        <Tile icon="⏱" label={L.tcIdle} value={`${idle} s`} tone={idle === 0 ? "good" : "warn"} note={idle === 0 ? L.tcNeverStops : L.tcStops} />
        <Tile icon="/img/resource/food.png" label={L.gathered} value={String(sum(s.gathered))} note={RESOURCES.map((r) => `${labels.t(`resources_short.${r}`)} ${Math.round(s.gathered[r])}`).join(" · ")} />
        <Tile icon="/img/resource/wood.png" label={L.spent} value={String(sum(s.spent))} note={RESOURCES.map((r) => `${labels.t(`resources_short.${r}`)} ${Math.round(s.spent[r])}`).join(" · ")} />
        {result.neutral_time !== null && goalTime !== null && (
          <Tile
            icon="⚖"
            label={L.vsNeutral}
            value={`${goalTime - result.neutral_time > 0 ? "+" : ""}${goalTime - result.neutral_time} s`}
            tone={goalTime <= result.neutral_time ? "good" : "bad"}
            note={`${clock(result.neutral_time)} → ${clock(goalTime)}`}
          />
        )}
      </div>

      {goalRows.some((r) => r.kind !== "age") && (
        <section className="chart-card">
          <h3>{L.goalChart}</h3>
          <p className="muted tiny">{L.goalChartNote}</p>
          <GoalTimeline rows={goalRows} end={end} goalTime={goalTime} labels={labels} />
        </section>
      )}

      <section className="chart-card">
        <h3>{L.chartVillagers}</h3>
        <div className="legend-row">
          {LAYERS.map((k) => (
            <span key={k}>
              <i className={`swatch ${k}`} />
              {k !== "build" && <img src={`/img/resource/${k}.png`} alt="" className="icon small" />}
              {layerName(k)}
            </span>
          ))}
        </div>
        <svg viewBox={`0 0 ${W} ${H}`} className="chart" onMouseMove={hoverAt} onMouseLeave={() => setHover(null)}>
          {axis}
          {[0, Math.round(maxV / 2), maxV].map((n) => (
            <text key={n} x={PAD.l - 6} y={yv(n) + 4} textAnchor="end" className="ylabel">
              {n}
            </text>
          ))}
          {LAYERS.map((k) => (
            <path key={k} d={areas[k]} className={`area ${k}`} />
          ))}
          {/* Etiquetas directas al final */}
          {(() => {
            const last = stack[stack.length - 1];
            let acc = 0;
            return LAYERS.map((k) => {
              const v = last.v[k];
              const mid = acc + v / 2;
              acc += v;
              return v > 0 ? (
                <text key={k} x={W - PAD.r + 6} y={yv(mid) + 4} className="direct">
                  {layerName(k)} {v}
                </text>
              ) : null;
            });
          })()}
          {hover !== null && hv && (
            <g className="crosshair">
              <line x1={x(hover)} x2={x(hover)} y1={PAD.t} y2={H - PAD.b} />
            </g>
          )}
        </svg>
        {hover !== null && hv && (
          <div className="chart-tip">
            <strong>{clock(hv.t)}</strong>
            {LAYERS.map((k) => (
              <span key={k}>
                <i className={`swatch ${k}`} /> {layerName(k)} {hv.v[k]}
              </span>
            ))}
          </div>
        )}
      </section>

      <section className="chart-card">
        <h3>{L.chartStock}</h3>
        <div className="legend-row">
          {RESOURCES.map((r) => (
            <span key={r}>
              <i className={`swatch ${r}`} />
              <img src={`/img/resource/${r}.png`} alt="" className="icon small" />
              {labels.t(`resources.${r}`)}
            </span>
          ))}
        </div>
        <svg viewBox={`0 0 ${W} ${H}`} className="chart" onMouseMove={hoverAt} onMouseLeave={() => setHover(null)}>
          {axis}
          {[0, 0.5, 1].map((f) => (
            <text key={f} x={PAD.l - 6} y={yr(maxR * f) + 4} textAnchor="end" className="ylabel">
              {Math.round(maxR * f)}
            </text>
          ))}
          {RESOURCES.map((r) => (
            <path key={r} d={lines[r]} className={`line ${r}`} />
          ))}
          {RESOURCES.map((r) => {
            const v = series[r][series[r].length - 1];
            return (
              <text key={r} x={W - PAD.r + 6} y={yr(v) + 4} className="direct">
                {labels.t(`resources.${r}`)} {Math.round(v)}
              </text>
            );
          })}
          {hover !== null && (
            <g className="crosshair">
              <line x1={x(hover)} x2={x(hover)} y1={PAD.t} y2={H - PAD.b} />
              {RESOURCES.map((r) => {
                const v = series[r][Math.min(hover, series[r].length - 1)];
                return <circle key={r} cx={x(hover)} cy={yr(v)} r={4} className={`dot ${r}`} />;
              })}
            </g>
          )}
        </svg>
        {hover !== null && (
          <div className="chart-tip">
            <strong>{clock(hover)}</strong>
            {RESOURCES.map((r) => (
              <span key={r}>
                <i className={`swatch ${r}`} /> {labels.t(`resources.${r}`)} {Math.round(series[r][Math.min(hover, series[r].length - 1)])}
              </span>
            ))}
          </div>
        )}
      </section>

      <section className="chart-card">
        <h3>{L.milestones}</h3>
        <div className="milestones">
          <div className="ms-track" />
          {milestones.map((m, i) => (
            <div key={i} className="ms" style={{ left: `${(m.t / end) * 100}%`, top: `${(i % 3) * 30}px` }} title={`${clock(m.t)} · ${m.label}`}>
              <img src={m.icon} alt="" />
              <span>{clock(m.t)}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="chart-card">
        <h3>{L.tipsTitle}</h3>
        <p className="muted tiny">{L.tipsNote}</p>
        <div className="tips">
          {tips.map((t) => (
            <TipCard key={t.id} tip={t} />
          ))}
        </div>
      </section>
    </div>
  );
}
