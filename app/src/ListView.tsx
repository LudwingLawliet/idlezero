import { useState } from "react";
import { Labels } from "./labels";
import { TextEvent, Timeline, TreeNode, clock } from "./types";

const TICK = 25;
// Eventos que aportan a la lista (las granjas y los cambios de tarea sobran).
const SHOWN = new Set(["build_started", "build_finished", "research_started", "age_reached", "unit_trained", "pool_exhausted", "lured", "tc_idle"]);

interface Row {
  tick: number;
  lines: { time: number; text: string; strong?: boolean }[];
}

function buildRows(tree: TreeNode[], events: TextEvent[], labels: Labels): Row[] {
  const rows = new Map<number, Row>();
  const row = (time: number) => {
    const tick = Math.floor(time / TICK);
    if (!rows.has(tick)) rows.set(tick, { tick, lines: [] });
    return rows.get(tick)!;
  };
  const L = labels.ui;
  for (const n of tree) {
    let text = `${L.villager} ${n.villager} → `;
    if (n.build) text += `${labels.name("building", n.build)} (${n.builders ?? 1} ${L.builders}) → `;
    text += labels.pool(n.pool);
    if (n.click) text += ` · ${L.click} ${labels.name("tech", n.click)}`;
    if (n.stop) text += ` · ${L.stop}`;
    row(n.time).lines.push({ time: n.time, text, strong: true });
  }
  for (const e of events) if (SHOWN.has(e.kind)) row(e.time).lines.push({ time: e.time, text: e.text });
  return [...rows.values()].sort((a, b) => a.tick - b.tick).map((r) => ({ ...r, lines: r.lines.sort((a, b) => a.time - b.time) }));
}

export default function ListView({ tree, events, timeline, labels, title, orderToml }: { tree: TreeNode[]; events: TextEvent[]; timeline: Timeline; labels: Labels; title: string; orderToml: string }) {
  const L = labels.ui;
  const rows = buildRows(tree, events, labels);
  const [copied, setCopied] = useState<string | null>(null);
  const s = timeline.series;
  const stock = (t: number) => {
    const i = Math.min(t, s.food.length - 1);
    return `${L.stock}: ${["food", "wood", "gold", "stone"].map((r, k) => `${labels.t(`resources_short.${r}`)} ${Math.round([s.food, s.wood, s.gold, s.stone][k][i])}`).join(" ")}`;
  };
  const text = () => {
    const out = [title, ""];
    for (const r of rows) {
      out.push(`${L.tick} ${r.tick} (${clock(r.tick * TICK)}) · ${stock(r.tick * TICK)}`);
      for (const l of r.lines) out.push(`  ${clock(l.time)}  ${l.text}`);
    }
    return out.join("\n");
  };
  const copy = (what: string, value: string) =>
    navigator.clipboard.writeText(value).then(() => {
      setCopied(what);
      setTimeout(() => setCopied(null), 1500);
    });
  return (
    <div className="list">
      <div className="list-actions">
        <button onClick={() => copy("text", text())}>{copied === "text" ? L.copied : L.copy}</button>
        <button onClick={() => copy("toml", orderToml)}>{copied === "toml" ? L.copied : L.exportOrder}</button>
      </div>
      {rows.map((r) => (
        <div className="tick-row" key={r.tick}>
          <div className="tick-head">
            {L.tick} {r.tick}
            <span className="muted"> · {clock(r.tick * TICK)}</span>
            <div className="muted tiny">{stock(r.tick * TICK)}</div>
          </div>
          <ul>
            {r.lines.map((l, i) => (
              <li key={i} className={l.strong ? "strong" : ""}>
                <span className="time">{clock(l.time)}</span>
                {l.text}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
