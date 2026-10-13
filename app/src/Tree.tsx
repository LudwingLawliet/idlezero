import { memo, useMemo } from "react";
import {
  Background,
  Controls,
  Edge,
  Handle,
  MarkerType,
  Node,
  NodeProps,
  Position,
  ReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Alternative, Choice, OptResult, Plan, TreeNode, clock } from "./types";
import { Labels } from "./labels";

const COL = 168; // separación horizontal entre decisiones
const ALT_Y = 132; // primera fila de alternativas (debajo de la ruta)
const ALT_ROW = 64;
const LANE_Y = -150; // camino alternativo seleccionado (encima)
const MAX_ALTS = 3;

type SpineData = { node: TreeNode; labels: Labels; selected: boolean };
type AltData = { alt: Alternative; labels: Labels };
type EdgeData = { labels: Labels };
type EndData = { title: string; time: string; ok: boolean; lane?: boolean };
type LaneData = { choice: Choice; differs: boolean; labels: Labels };

function ChoiceBody({ choice, labels }: { choice: Choice; labels: Labels }) {
  return (
    <>
      <div className="choice">
        <img src="/img/unit/villager.png" alt="" className="icon" />
        <span className="arrow">→</span>
        <img src={labels.poolIcon(choice.pool)} alt="" className="icon" />
        <span className="pool">{labels.pool(choice.pool)}</span>
      </div>
      {choice.click && (
        <div className="badge click" title={choice.hard ? labels.ui.hardClick : undefined}>
          <img src={`/img/tech/${choice.click}.png`} alt="" className="icon small" />
          {labels.ui.click} {labels.tech(choice.click)}
          {choice.hard ? " ⏸" : ""}
        </div>
      )}
      {choice.stop && <div className="badge stop">{labels.ui.stop}</div>}
    </>
  );
}

const SpineNode = memo(({ data }: NodeProps<Node<SpineData>>) => {
  const { node, labels, selected } = data;
  return (
    <div className={`tnode spine${selected ? " selected" : ""}`}>
      <Handle type="target" position={Position.Left} />
      <div className="meta">
        <span>{clock(node.time)}</span>
        <span className="muted">
          {labels.ui.villager} {node.villager}
        </span>
      </div>
      <ChoiceBody choice={node} labels={labels} />
      <Handle type="source" position={Position.Right} />
      <Handle type="source" position={Position.Bottom} id="alts" />
    </div>
  );
});

function deltaClass(d: number | null) {
  if (d === null) return "fail";
  if (d === 0) return "same";
  if (d <= 20) return "warn";
  return "bad";
}

function deltaText(alt: Alternative, labels: Labels) {
  if (alt.delta === null) return labels.ui.impossible;
  if (alt.delta === 0) return `= ${labels.ui.same}`;
  return `${alt.delta > 0 ? "+" : ""}${alt.delta} s`;
}

const AltNode = memo(({ data }: NodeProps<Node<AltData>>) => {
  const { alt, labels } = data;
  return (
    <div className={`tnode alt ${deltaClass(alt.delta)}`}>
      <Handle type="target" position={Position.Top} />
      <div className="choice">
        <img src={labels.poolIcon(alt.pool)} alt="" className="icon small" />
        <span className="pool">{labels.pool(alt.pool)}</span>
        {alt.click && <img src={`/img/tech/${alt.click}.png`} alt="" className="icon small" />}
      </div>
      <div className="delta">{deltaText(alt, labels)}</div>
    </div>
  );
});

const EndNode = memo(({ data }: NodeProps<Node<EndData>>) => (
  <div className={`tnode end${data.ok ? " ok" : " fail"}${data.lane ? " lane" : ""}`}>
    <Handle type="target" position={Position.Left} />
    <div className="muted">{data.title}</div>
    <div className="big">{data.time}</div>
  </div>
));

const StartNode = memo(({ data }: NodeProps<Node<{ title: string }>>) => (
  <div className="tnode start">
    <div className="big">0:00</div>
    <div className="muted">{data.title}</div>
    <Handle type="source" position={Position.Right} />
  </div>
));

const LaneNode = memo(({ data }: NodeProps<Node<LaneData>>) => (
  <div className={`tnode lane${data.differs ? " differs" : ""}`}>
    <Handle type="target" position={Position.Left} />
    <div className="choice">
      <img src={data.labels.poolIcon(data.choice.pool)} alt="" className="icon small" />
      <span className="pool">{data.labels.pool(data.choice.pool)}</span>
      {data.choice.click && <img src={`/img/tech/${data.choice.click}.png`} alt="" className="icon small" />}
    </div>
    <Handle type="source" position={Position.Right} />
  </div>
));

const nodeTypes = { spine: SpineNode, alt: AltNode, end: EndNode, start: StartNode, lane: LaneNode };

const sameChoice = (a: Choice, b: Choice) =>
  a.pool === b.pool && (a.click ?? null) === (b.click ?? null) && !!a.stop === !!b.stop;

export default function Tree({
  result,
  labels,
  selected,
  onSelect,
  plan,
}: {
  result: OptResult;
  labels: Labels;
  selected: number | null;
  onSelect: (i: number) => void;
  plan: Plan | null;
}) {
  const { nodes, edges } = useMemo(() => {
    const nodes: Node[] = [];
    const edges: Edge<EdgeData>[] = [];
    const accent = "var(--accent)";
    nodes.push({ id: "start", type: "start", position: { x: -COL, y: 6 }, data: { title: labels.ui.start }, draggable: false });
    let prev = "start";
    result.tree.forEach((n, i) => {
      const id = `d${i}`;
      nodes.push({
        id,
        type: "spine",
        position: { x: i * COL, y: 0 },
        data: { node: n, labels, selected: selected === i },
        draggable: false,
      });
      edges.push({
        id: `e-${prev}-${id}`,
        source: prev,
        target: id,
        animated: true,
        style: { stroke: accent, strokeWidth: 3 },
      });
      prev = id;
      n.alternatives.slice(0, MAX_ALTS).forEach((alt, k) => {
        const aid = `${id}-a${k}`;
        nodes.push({
          id: aid,
          type: "alt",
          position: { x: i * COL + 10, y: ALT_Y + k * ALT_ROW },
          data: { alt, labels },
          draggable: false,
        });
        edges.push({
          id: `e-${id}-${aid}`,
          source: id,
          sourceHandle: "alts",
          target: aid,
          type: "smoothstep",
          style: { stroke: "var(--branch)", strokeDasharray: "4 4" },
        });
      });
    });
    const t = result.best.time;
    nodes.push({
      id: "end",
      type: "end",
      position: { x: result.tree.length * COL, y: 4 },
      data: { title: t !== null ? labels.ui.goalReached : labels.ui.notReached, time: t !== null ? clock(t) : "—", ok: t !== null },
      draggable: false,
    });
    edges.push({
      id: `e-${prev}-end`,
      source: prev,
      target: "end",
      style: { stroke: accent, strokeWidth: 3 },
      markerEnd: { type: MarkerType.ArrowClosed, color: "var(--accent)" },
    });

    // Camino alternativo seleccionado, como carril encima de la ruta óptima.
    if (plan && plan !== result.best) {
      let lprev = "start";
      plan.choices.forEach((c, i) => {
        const id = `l${i}`;
        const best = result.best.choices[i];
        nodes.push({
          id,
          type: "lane",
          position: { x: i * COL + 14, y: LANE_Y },
          data: { choice: c, differs: !best || !sameChoice(c, best), labels },
          draggable: false,
        });
        edges.push({ id: `el-${lprev}-${id}`, source: lprev, target: id, style: { stroke: "var(--lane)", strokeWidth: 2 } });
        lprev = id;
      });
      const pt = plan.time;
      nodes.push({
        id: "lend",
        type: "end",
        position: { x: plan.choices.length * COL + 14, y: LANE_Y - 6 },
        data: { title: labels.ui.plans, time: pt !== null ? clock(pt) : "—", ok: pt !== null, lane: true },
        draggable: false,
      });
      edges.push({ id: `el-${lprev}-lend`, source: lprev, target: "lend", style: { stroke: "var(--lane)", strokeWidth: 2 } });
    }
    return { nodes, edges };
  }, [result, labels, selected, plan]);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodeClick={(_, n) => {
        if (n.id.startsWith("d") && !n.id.includes("-")) onSelect(Number(n.id.slice(1)));
      }}
      fitView
      // Al abrir, encuadra el inicio y las primeras decisiones a tamaño legible.
      fitViewOptions={{ padding: 0.12, maxZoom: 1, nodes: [{ id: "start" }, ...[0, 1, 2, 3, 4, 5].map((i) => ({ id: `d${i}` }))] }}
      minZoom={0.15}
      maxZoom={1.6}
      nodesConnectable={false}
    >
      <Background gap={24} color="var(--grid)" />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
