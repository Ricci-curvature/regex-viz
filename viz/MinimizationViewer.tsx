// MinimizationViewer — Stage 5 viewer for Hopcroft DFA minimization.
// Left pane: the source DFA (from Stage 3) plus a dashed sink node, with
// every state colored by the block it currently belongs to. The splitter
// block's members get a thick outline. Right pane: the final minimized
// DFA (static across all steps — it is the "destination" that the left
// pane's blocks collapse into).
//
// The shared slider walks through each (splitter, symbol) pair that caused
// a block to split. Steps that didn't cause a split are not emitted by the
// Rust side, so the slider only stops on interesting frames. The first
// frame is the initial {accept} | {non-accept} partition and the last is
// the fixed-point verdict.

import { useMemo, useState } from "react";

import type { DfaState, DfaTransition } from "./construction";
import { DfaGraph } from "./DfaGraph";
import type { MinimizationTrace } from "./minimization";

export type MinimizationViewerProps = {
  trace: MinimizationTrace;
  className?: string;
};

// Block palette — pastels with enough contrast for colorblind safety on
// adjacent blocks. Colors are chosen by smallest element of each block so
// that when a block splits into (child_in containing the smallest) and
// (child_out), the surviving half keeps its color and only the new half
// picks up a fresh one.
const PALETTE = [
  "#fde4cf", // peach
  "#c9e4de", // mint
  "#c6def1", // powder
  "#dbcdf0", // lilac
  "#f7d6e0", // rose
  "#faedcb", // cream
  "#cddafd", // periwinkle
  "#ead7c3", // tan
];

function colorForSmallest(smallest: number): string {
  return PALETTE[smallest % PALETTE.length];
}

export function MinimizationViewer({ trace, className }: MinimizationViewerProps) {
  const [i, setI] = useState(0);
  const last = trace.steps.length - 1;
  const clamped = Math.min(i, last);
  const step = trace.steps[clamped];

  // Build a totalized source-DFA view: original states + synthetic sink +
  // the original transitions + every missing (state, symbol) filled with a
  // transition to the sink. This is what Hopcroft actually operates on, so
  // the viewer should show it too — otherwise the "blocks" on the left
  // pane appear to include phantom members.
  const totalized = useMemo(
    () => totalize(trace),
    [trace],
  );

  // Per-node fill color: look up the block containing the node in the
  // current partition, color by its smallest element.
  const blockFill = useMemo(() => {
    const fill: Record<number, string> = {};
    for (const block of step.partition) {
      const color = colorForSmallest(block[0]);
      for (const id of block) {
        fill[id] = color;
      }
    }
    return fill;
  }, [step.partition]);

  const splitterOutline = step.splitter_block ?? [];

  return (
    <div
      className={className}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 10,
        alignItems: "stretch",
        padding: 12,
        border: "1px solid #e2e2e2",
        borderRadius: 6,
        background: "#fafafa",
      }}
    >
      <Header
        regex={trace.regex}
        alphabet={trace.alphabet}
        sourceCount={trace.source_dfa_states.length}
        minimizedVisible={visibleCount(trace)}
      />

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 12,
          alignItems: "start",
        }}
      >
        <Pane label="source DFA (totalized, colored by block)">
          <DfaGraph
            states={totalized.states}
            transitions={totalized.transitions}
            blockFill={blockFill}
            outlineIds={splitterOutline}
            sinkId={trace.sink_id}
            subsetLabel={() => ""}
          />
        </Pane>
        <Pane label="minimized DFA (destination)">
          <DfaGraph
            states={minimizedAsSourceShape(trace)}
            transitions={trace.minimized.transitions}
            subsetLabel={(s) =>
              // Show which source ids collapsed into this minimized state.
              // Skip sink: it's hidden via sinkId anyway.
              trace.minimized.states[s.id].is_sink
                ? ""
                : `{${trace.minimized.states[s.id].block.join(",")}}`
            }
            sinkId={trace.minimized.sink_block}
          />
        </Pane>
      </div>

      <PartitionSnapshot partition={step.partition} sinkId={trace.sink_id} />

      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <button
          type="button"
          onClick={() => setI(Math.max(0, clamped - 1))}
          disabled={clamped === 0}
          aria-label="previous step"
        >
          ◀
        </button>
        <input
          type="range"
          min={0}
          max={last}
          value={clamped}
          onChange={(e) => setI(Number(e.target.value))}
          style={{ flex: 1 }}
          aria-label="step"
        />
        <button
          type="button"
          onClick={() => setI(Math.min(last, clamped + 1))}
          disabled={clamped === last}
          aria-label="next step"
        >
          ▶
        </button>
      </div>

      <div
        style={{
          fontFamily: "ui-monospace, monospace",
          fontSize: 13,
          color: "#333",
          display: "flex",
          justifyContent: "space-between",
          gap: 12,
        }}
      >
        <span>
          step {clamped + 1} / {trace.steps.length}
        </span>
        <span style={{ flex: 1, textAlign: "center" }}>{step.description}</span>
        <span>
          {step.partition.length} block{step.partition.length === 1 ? "" : "s"}
        </span>
      </div>
    </div>
  );
}

function Header({
  regex,
  alphabet,
  sourceCount,
  minimizedVisible,
}: {
  regex: string;
  alphabet: string[];
  sourceCount: number;
  minimizedVisible: number;
}) {
  return (
    <div
      style={{
        fontFamily: "ui-monospace, monospace",
        fontSize: 13,
        color: "#444",
        display: "flex",
        justifyContent: "space-between",
        gap: 12,
        alignItems: "center",
      }}
    >
      <span>
        regex: <code>{regex}</code>
      </span>
      <span>Σ = {`{${alphabet.join(",")}}`}</span>
      <span>
        {sourceCount} → {minimizedVisible} state{minimizedVisible === 1 ? "" : "s"}
      </span>
    </div>
  );
}

function Pane({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 6,
        alignItems: "stretch",
      }}
    >
      <div
        style={{
          fontFamily: "ui-monospace, monospace",
          fontSize: 12,
          color: "#666",
          textAlign: "center",
        }}
      >
        {label}
      </div>
      <div style={{ display: "flex", justifyContent: "center" }}>{children}</div>
    </div>
  );
}

function PartitionSnapshot({
  partition,
  sinkId,
}: {
  partition: number[][];
  sinkId: number;
}) {
  return (
    <div
      style={{
        fontFamily: "ui-monospace, monospace",
        fontSize: 12,
        display: "flex",
        flexWrap: "wrap",
        gap: 6,
        justifyContent: "center",
      }}
    >
      {partition.map((block, i) => (
        <span
          key={i}
          style={{
            padding: "2px 8px",
            borderRadius: 4,
            background: colorForSmallest(block[0]),
            border: "1px solid #bbb",
          }}
        >
          {"{"}
          {block
            .map((id) => (id === sinkId ? "∅" : `D${id}`))
            .join(",")}
          {"}"}
        </span>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------
// Helpers — build view-only derived data from the raw trace.
// ---------------------------------------------------------------------

function totalize(trace: MinimizationTrace): {
  states: DfaState[];
  transitions: DfaTransition[];
} {
  const states: DfaState[] = [...trace.source_dfa_states];
  // Add the synthetic sink as a real DfaState for layout purposes. It has
  // no underlying NFA subset; DfaGraph hides the caption when sinkId matches.
  states.push({ id: trace.sink_id, subset: [], is_accept: false });

  const transitions: DfaTransition[] = [...trace.source_dfa_transitions];

  // Fill missing (state, symbol) transitions by routing to the sink. Without
  // this the viewer would show blocks whose members are colored identically
  // but "look like" they should be differentiable — hiding the sink is a
  // rendering choice, not a semantic one.
  const have = new Set<string>(
    trace.source_dfa_transitions.map((t) => `${t.from}|${t.label}`),
  );
  for (const s of trace.source_dfa_states) {
    for (const c of trace.alphabet) {
      if (!have.has(`${s.id}|${c}`)) {
        transitions.push({ from: s.id, to: trace.sink_id, label: c });
      }
    }
  }
  // Sink self-loops on every symbol.
  for (const c of trace.alphabet) {
    transitions.push({ from: trace.sink_id, to: trace.sink_id, label: c });
  }

  return { states, transitions };
}

// The right pane's DfaGraph expects DfaState[] with subset/is_accept. The
// minimized side stores `block` instead of `subset`. Adapt: treat `block`
// as the subset caption so DfaGraph can render it.
function minimizedAsSourceShape(trace: MinimizationTrace): DfaState[] {
  return trace.minimized.states.map((s) => ({
    id: s.id,
    subset: s.block,
    is_accept: s.is_accept,
  }));
}

function visibleCount(trace: MinimizationTrace): number {
  // If the sink block contains only the synthetic sink, it's hidden → don't
  // count it. If real source states were absorbed into the sink block (dead
  // states), the whole block is still one minimized state and we show it.
  const sink = trace.minimized.states[trace.minimized.sink_block];
  return sink.block.length === 1 && sink.block[0] === trace.sink_id
    ? trace.minimized.states.length - 1
    : trace.minimized.states.length;
}
