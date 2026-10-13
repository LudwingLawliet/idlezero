import { useState } from "react";
import { OptimizeView, clock } from "./types";
import { Labels } from "./labels";

const TICK = 25;
// Eventos que aportan a la lista (las granjas y movimientos automáticos sobran).
const SHOWN = new Set(["build_started", "build_finished", "research_started", "age_reached", "unit_trained", "pool_exhausted", "lured"]);

interface Row {
  tick: number;
  lines: { time: number; text: string; strong?: boolean }[];
}

export function buildRows(view: OptimizeView, labels: Labels): Row[] {
  const rows = new Map<number, Row>();
  const row = (time: number) => {
    const tick = Math.floor(time / TICK);
    if (!rows.has(tick)) rows.set(tick, { tick, lines: [] });
    return rows.get(tick)!;
  };
  for (const n of view.result.tree) {
    let text = `${labels.ui.villager} ${n.villager} → ${labels.pool(n.pool)}`;
    if (n.click) text += ` · ${labels.ui.click} ${labels.tech(n.click)}`;
    if (n.stop) text += ` · ${labels.ui.stop}`;
    row(n.time).lines.push({ time: n.time, text, strong: true });
  }
  for (const e of view.events) {
    if (SHOWN.has(e.kind)) row(e.time).lines.push({ time: e.time, text: e.text });
  }
  return [...rows.values()]
    .sort((a, b) => a.tick - b.tick)
    .map((r) => ({ ...r, lines: r.lines.sort((a, b) => a.time - b.time) }));
}

export function rowsToText(rows: Row[], labels: Labels, title: string): string {
  const out = [title, ""];
  for (const r of rows) {
    out.push(`${labels.ui.tick} ${r.tick} (${clock(r.tick * TICK)})`);
    for (const l of r.lines) out.push(`  ${clock(l.time)}  ${l.text}`);
  }
  return out.join("\n");
}

export default function ListView({ view, labels, title }: { view: OptimizeView; labels: Labels; title: string }) {
  const rows = buildRows(view, labels);
  const [copied, setCopied] = useState<string | null>(null);
  const copy = (what: string, text: string) =>
    navigator.clipboard.writeText(text).then(() => {
      setCopied(what);
      setTimeout(() => setCopied(null), 1500);
    });
  return (
    <div className="list">
      <div className="list-actions">
        <button onClick={() => copy("text", rowsToText(rows, labels, title))}>
          {copied === "text" ? labels.ui.copied : labels.ui.copy}
        </button>
        <button onClick={() => copy("toml", view.result.order_toml)}>
          {copied === "toml" ? labels.ui.copied : labels.ui.exportOrder}
        </button>
      </div>
      {rows.map((r) => (
        <div className="tick-row" key={r.tick}>
          <div className="tick-head">
            {labels.ui.tick} {r.tick}
            <span className="muted"> · {clock(r.tick * TICK)}</span>
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
