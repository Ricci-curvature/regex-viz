// DfaGraph — dagre layout + SVG, tuned for subset-construction output.
// Different from NfaGraph in three ways:
//   1. Multiple accept states (double-circle drawn on every `is_accept`).
//   2. No ε-transitions; every edge has a single-char label.
//   3. Each node can show the underlying NFA subset as a caption, which helps
//      the reader connect D-state to the NFA pane on the left.

import { useMemo } from "react";
import dagre from "dagre";

import type { DfaState, DfaTransition } from "./construction";

const NODE_R = 22;
const NODE_D = NODE_R * 2;

type Point = { x: number; y: number };
type LaidOutEdge = {
  from: number;
  to: number;
  label: string;
  points: Point[];
};

export type DfaGraphProps = {
  states: DfaState[];
  transitions: DfaTransition[];
  /** When set, this node is outlined in the focus color. */
  focus?: number | null;
  /** Show the NFA subset beneath each D-node label (default: true). */
  showSubset?: boolean;
  /** Per-node fill override. Takes precedence over focus/accept defaults.
   * Stage 5 uses this to color nodes by partition block. */
  blockFill?: Record<number, string>;
  /** State ids that should get a thick, focus-colored outline without
   * changing fill. Stage 5 draws the splitter block's members this way. */
  outlineIds?: number[];
  /** If set, the node with this id is rendered as the implicit sink:
   * dashed outline, grey fill, `∅` label, no subset caption. */
  sinkId?: number | null;
  /** Custom caption renderer beneath each D-node label. Falls back to
   * `{subset}` when absent. Return empty string to hide the caption. */
  subsetLabel?: (state: DfaState) => string;
  className?: string;
};

export function DfaGraph({
  states,
  transitions,
  focus = null,
  showSubset = true,
  blockFill,
  outlineIds,
  sinkId = null,
  subsetLabel,
  className,
}: DfaGraphProps) {
  const outlineSet = outlineIds ? new Set(outlineIds) : null;
  const layout = useMemo(
    () => computeLayout(states, transitions),
    [states, transitions],
  );

  // Empty state — dagre would otherwise return a 0×0 graph which degrades to
  // an invisible SVG. Render a neutral placeholder so step 0 ("announce Σ,
  // no DFA state yet") still looks intentional.
  if (states.length === 0) {
    return (
      <svg
        className={className}
        viewBox="0 0 200 80"
        width={200}
        height={80}
        style={{ maxWidth: "100%", height: "auto", fontFamily: "ui-monospace, monospace" }}
      >
        <text
          x={100}
          y={44}
          textAnchor="middle"
          fontSize="13"
          fill="#888"
        >
          (no DFA state yet)
        </text>
      </svg>
    );
  }

  return (
    <svg
      className={className}
      viewBox={`0 0 ${layout.width} ${layout.height}`}
      width={layout.width}
      height={layout.height}
      style={{ maxWidth: "100%", height: "auto", fontFamily: "ui-monospace, monospace" }}
    >
      <defs>
        <marker
          id="regex-viz-dfa-arrow"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#555" />
        </marker>
      </defs>

      {layout.edges.map((e, i) => {
        const mid = midpoint(e.points);
        return (
          <g key={i}>
            <path
              d={pointsToPath(e.points)}
              fill="none"
              stroke="#555"
              strokeWidth={1.5}
              markerEnd="url(#regex-viz-dfa-arrow)"
            />
            <text
              x={mid.x}
              y={mid.y - 5}
              textAnchor="middle"
              fontSize="12"
              fill="#222"
              stroke="white"
              strokeWidth="3"
              paintOrder="stroke"
            >
              {e.label}
            </text>
          </g>
        );
      })}

      {states.map((s) => {
        const n = layout.nodes[s.id];
        if (!n) return null;
        const isFocus = s.id === focus;
        const isOutlined = outlineSet?.has(s.id) ?? false;
        const isSink = sinkId != null && s.id === sinkId;
        const overrideFill = blockFill?.[s.id];
        const fill = isSink
          ? "#ececec"
          : overrideFill ??
            (isFocus ? "#ffd866" : s.is_accept ? "#d3e4ff" : "#ffffff");
        const strokeColor = isFocus || isOutlined ? "#c79500" : "#222";
        const strokeWidth = isFocus || isOutlined ? 2.5 : 1.5;
        const dashed = isSink ? "3 3" : undefined;
        const label = isSink ? "∅" : `D${s.id}`;
        const caption = isSink
          ? ""
          : subsetLabel
            ? subsetLabel(s)
            : `{${s.subset.join(",")}}`;
        return (
          <g key={s.id}>
            {s.is_accept && !isSink && (
              <circle
                cx={n.x}
                cy={n.y}
                r={NODE_R + 3}
                fill="none"
                stroke="#222"
                strokeWidth={1.5}
              />
            )}
            <circle
              cx={n.x}
              cy={n.y}
              r={NODE_R}
              fill={fill}
              stroke={strokeColor}
              strokeWidth={strokeWidth}
              strokeDasharray={dashed}
            />
            <text
              x={n.x}
              y={n.y + 4}
              textAnchor="middle"
              fontSize="13"
              fontWeight={600}
              fill={isSink ? "#777" : "#000"}
            >
              {label}
            </text>
            {showSubset && caption && (
              <text
                x={n.x}
                y={n.y + NODE_R + 14}
                textAnchor="middle"
                fontSize="11"
                fill="#555"
              >
                {caption}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

function computeLayout(states: DfaState[], transitions: DfaTransition[]) {
  const g = new dagre.graphlib.Graph({ multigraph: true });
  g.setGraph({ rankdir: "LR", nodesep: 36, ranksep: 64, marginx: 20, marginy: 24 });
  g.setDefaultEdgeLabel(() => ({}));

  states.forEach((s) => {
    g.setNode(String(s.id), { width: NODE_D, height: NODE_D });
  });
  transitions.forEach((t, i) => {
    g.setEdge(String(t.from), String(t.to), { label: t.label }, `e${i}`);
  });

  dagre.layout(g);

  const graph = g.graph();
  const width = graph.width ?? 100;
  // Extra vertical slack so the subset caption (below the circle) is not
  // clipped by the viewBox. dagre plans for node height only.
  const height = (graph.height ?? 100) + 18;

  const nodes: Record<number, Point> = {};
  states.forEach((s) => {
    const n = g.node(String(s.id)) as { x: number; y: number };
    nodes[s.id] = { x: n.x, y: n.y };
  });
  const edges: LaidOutEdge[] = transitions.map((t, i) => {
    const e = g.edge(String(t.from), String(t.to), `e${i}`) as {
      points: Point[];
    };
    return { from: t.from, to: t.to, label: t.label, points: e.points ?? [] };
  });

  return { nodes, edges, width, height };
}

function pointsToPath(pts: Point[]): string {
  if (pts.length === 0) return "";
  const [first, ...rest] = pts;
  return `M ${first.x} ${first.y} ` + rest.map((p) => `L ${p.x} ${p.y}`).join(" ");
}

function midpoint(pts: Point[]): Point {
  if (pts.length === 0) return { x: 0, y: 0 };
  return pts[Math.floor(pts.length / 2)];
}
