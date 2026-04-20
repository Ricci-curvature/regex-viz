// TraceViewer — step slider + NfaGraph. Works for both Build traces (NFA
// grows, `active` is empty) and Run traces (NFA is fixed, `active` moves).

import { useState } from "react";

import { NfaGraph } from "./NfaGraph";
import type { Trace } from "./trace";

export type TraceViewerProps = {
  trace: Trace;
  className?: string;
};

export function TraceViewer({ trace, className }: TraceViewerProps) {
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
      <div style={{ display: "flex", justifyContent: "center" }}>
        <NfaGraph nfa={step.nfa} active={step.active} />
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
          {step.nfa.states.length}s · {step.nfa.transitions.length}t
        </span>
      </div>

      {trace.kind === "run" && trace.input != null && (
        <InputStrip input={trace.input} pos={step.input_pos} />
      )}
    </div>
  );
}

function InputStrip({ input, pos }: { input: string; pos: number | null }) {
  const chars = Array.from(input);
  return (
    <div style={{ fontFamily: "ui-monospace, monospace", fontSize: 14, textAlign: "center" }}>
      {chars.map((c, i) => (
        <span
          key={i}
          style={{
            padding: "2px 4px",
            margin: "0 1px",
            background: i === pos ? "#ffd866" : "transparent",
            borderBottom: i === pos ? "2px solid #c79500" : "2px solid transparent",
          }}
        >
          {c}
        </span>
      ))}
    </div>
  );
}
