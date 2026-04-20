// NfaGraph — dagre computes layout, SVG is rendered by hand.
// Deliberately unstyled beyond minimal inline styles so the consuming site's
// theme (riccilab.dev) can override via CSS class selectors on the wrapper.

import { useMemo } from "react";
// TODO: verify — in the blog repo we may end up aliasing to '@dagrejs/dagre'
// (modern fork). The API surface (graphlib.Graph, layout) is the same.
import dagre from "dagre";

import { EPSILON, type Nfa } from "./trace";

const NODE_R = 18;
const NODE_D = NODE_R * 2;

type Point = { x: number; y: number };
type LaidOutEdge = {
  from: number;
  to: number;
  label: string;
  points: Point[];
};

export type NfaGraphProps = {
  nfa: Nfa;
  active?: number[];
  className?: string;
};

export function NfaGraph({ nfa, active = [], className }: NfaGraphProps) {
  const layout = useMemo(() => computeLayout(nfa), [nfa]);
  const activeSet = new Set(active);

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
          id="regex-viz-arrow"
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
        const isEps = e.label === EPSILON;
        const mid = midpoint(e.points);
        return (
          <g key={i}>
            <path
              d={pointsToPath(e.points)}
              fill="none"
              stroke={isEps ? "#b8b8b8" : "#555"}
              strokeWidth={1.5}
              strokeDasharray={isEps ? "4 3" : undefined}
              markerEnd="url(#regex-viz-arrow)"
            />
            <text
              x={mid.x}
              y={mid.y - 5}
              textAnchor="middle"
              fontSize="12"
              fill={isEps ? "#888" : "#222"}
              stroke="white"
              strokeWidth="3"
              paintOrder="stroke"
            >
              {e.label}
            </text>
          </g>
        );
      })}

      {layout.nodes.map((n) => {
        const isStart = n.id === nfa.start;
        const isAccept = n.id === nfa.accept;
        const isActive = activeSet.has(n.id);
        const fill = isActive ? "#ffd866" : isStart ? "#d3e4ff" : "#ffffff";
        return (
          <g key={n.id}>
            {isAccept && (
              <circle
                cx={n.x}
                cy={n.y}
                r={NODE_R + 3}
                fill="none"
                stroke="#222"
                strokeWidth={1.5}
              />
            )}
            <circle cx={n.x} cy={n.y} r={NODE_R} fill={fill} stroke="#222" strokeWidth={1.5} />
            <text
              x={n.x}
              y={n.y + 4}
              textAnchor="middle"
              fontSize="13"
              fontWeight={500}
            >
              {n.id}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

function computeLayout(nfa: Nfa) {
  // TODO: verify — dagre's multigraph edge key + `g.edge(v, w, name)` lookup
  // is correct on current @dagrejs/dagre. Parallel ε-edges require distinct
  // keys, hence `e${i}`.
  const g = new dagre.graphlib.Graph({ multigraph: true });
  g.setGraph({ rankdir: "LR", nodesep: 24, ranksep: 48, marginx: 16, marginy: 16 });
  g.setDefaultEdgeLabel(() => ({}));

  nfa.states.forEach((s) => {
    g.setNode(String(s), { width: NODE_D, height: NODE_D });
  });
  nfa.transitions.forEach((t, i) => {
    g.setEdge(String(t.from), String(t.to), { label: t.label }, `e${i}`);
  });

  dagre.layout(g);

  const graph = g.graph();
  const width = graph.width ?? 100;
  const height = graph.height ?? 100;

  const nodes = nfa.states.map((s) => {
    const n = g.node(String(s)) as { x: number; y: number };
    return { id: s, x: n.x, y: n.y };
  });
  const edges: LaidOutEdge[] = nfa.transitions.map((t, i) => {
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
