// ConstructionViewer — two-pane viewer for subset-construction traces.
// Left pane: the source NFA, with the current step's `focus_nfa_subset`
// highlighted. Right pane: the DFA built so far, with the current step's
// focus D-state highlighted. One slider drives both.
//
// This is the Stage 3 analogue of TraceViewer: same slider UX, different
// payload. Kept as a separate component because the shape (`ConstructionTrace`
// vs `Trace`) is genuinely different — merging them would mean branching on
// shape at every render, which obscures intent.

import { useState } from "react";

import type { ConstructionTrace } from "./construction";
import { DfaGraph } from "./DfaGraph";
import { NfaGraph } from "./NfaGraph";

export type ConstructionViewerProps = {
  trace: ConstructionTrace;
  className?: string;
};

export function ConstructionViewer({ trace, className }: ConstructionViewerProps) {
  const [i, setI] = useState(0);
  const last = trace.steps.length - 1;
  const clamped = Math.min(i, last);
  const step = trace.steps[clamped];

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
      <Header regex={trace.regex} alphabet={trace.alphabet} />

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 12,
          alignItems: "start",
        }}
      >
        <Pane label="NFA (source)">
          <NfaGraph nfa={trace.nfa} active={step.focus_nfa_subset} />
        </Pane>
        <Pane label="DFA (under construction)">
          <DfaGraph
            states={step.dfa_states}
            transitions={step.dfa_transitions}
            focus={step.focus_dfa_state}
          />
        </Pane>
      </div>

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
          {step.dfa_states.length}D · {step.dfa_transitions.length}δ
        </span>
      </div>
    </div>
  );
}

function Header({ regex, alphabet }: { regex: string; alphabet: string[] }) {
  return (
    <div
      style={{
        fontFamily: "ui-monospace, monospace",
        fontSize: 13,
        color: "#444",
        display: "flex",
        justifyContent: "space-between",
        gap: 12,
      }}
    >
      <span>
        regex: <code>{regex}</code>
      </span>
      <span>
        Σ = {`{${alphabet.join(",")}}`}
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
