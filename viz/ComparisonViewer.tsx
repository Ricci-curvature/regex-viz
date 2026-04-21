// ComparisonViewer — two-pane viewer for NFA vs DFA on the same input.
// Left pane: the source NFA, with the current step's `nfa_active` set
// highlighted. Right pane: the full DFA, with the current step's
// `dfa_current` outlined. Shared slider + one InputStrip driving both.
//
// Stage 4 analogue of TraceViewer (which only shows one engine) and
// ConstructionViewer (which shows NFA + growing DFA but has no input).
// Kept as its own component because the payload (`ComparisonTrace`) is
// distinct and the verdict badge is specific to this stage.

import { useState } from "react";

import type { ComparisonTrace } from "./comparison";
import { DfaGraph } from "./DfaGraph";
import { NfaGraph } from "./NfaGraph";

export type ComparisonViewerProps = {
  trace: ComparisonTrace;
  className?: string;
};

export function ComparisonViewer({ trace, className }: ComparisonViewerProps) {
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
      <Header
        regex={trace.regex}
        input={trace.input}
        alphabet={trace.alphabet}
        summary={trace.summary}
      />

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 12,
          alignItems: "start",
        }}
      >
        <Pane label="NFA simulator">
          <NfaGraph nfa={trace.nfa} active={step.nfa_active} />
        </Pane>
        <Pane label="DFA simulator">
          <DfaGraph
            states={trace.dfa_states}
            transitions={trace.dfa_transitions}
            focus={step.dfa_current}
          />
        </Pane>
      </div>

      <InputStrip input={trace.input} pos={step.input_pos} />

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
          {formatDfaCurrent(step.dfa_current)} · NFA {step.nfa_active.length}
        </span>
      </div>
    </div>
  );
}

function formatDfaCurrent(cur: number | null): string {
  return cur == null ? "DFA ∅" : `DFA D${cur}`;
}

function Header({
  regex,
  input,
  alphabet,
  summary,
}: {
  regex: string;
  input: string;
  alphabet: string[];
  summary: {
    nfa_accepted: boolean;
    dfa_accepted: boolean;
    verdicts_agree: boolean;
  };
}) {
  // Verdict badge: green when both engines agree on accept, red on disagree,
  // neutral grey when both agree on reject. Reject-agree is the common case
  // for "no-match" pins, so it shouldn't shout.
  const badgeStyle = summary.verdicts_agree
    ? summary.nfa_accepted
      ? { background: "#d4f4dd", color: "#1a6b2e", border: "1px solid #1a6b2e" }
      : { background: "#eee", color: "#555", border: "1px solid #999" }
    : { background: "#ffd6d6", color: "#a00", border: "1px solid #a00" };
  const badgeText = summary.verdicts_agree
    ? summary.nfa_accepted
      ? "match (agree)"
      : "reject (agree)"
    : "BUG: engines disagree";

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
      <span>
        input: <code>{JSON.stringify(input)}</code>
      </span>
      <span>Σ = {`{${alphabet.join(",")}}`}</span>
      <span
        style={{
          padding: "2px 8px",
          borderRadius: 4,
          fontSize: 12,
          ...badgeStyle,
        }}
      >
        {badgeText}
      </span>
    </div>
  );
}

function Pane({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
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
      <div style={{ display: "flex", justifyContent: "center" }}>
        {children}
      </div>
    </div>
  );
}

function InputStrip({ input, pos }: { input: string; pos: number }) {
  const chars = Array.from(input);
  // `pos === chars.length` at the verdict step — no character to highlight,
  // but we still render the strip so the layout doesn't jump.
  return (
    <div
      style={{
        fontFamily: "ui-monospace, monospace",
        fontSize: 14,
        textAlign: "center",
      }}
    >
      {chars.length === 0 ? (
        <span style={{ color: "#888" }}>(empty input)</span>
      ) : (
        chars.map((c, i) => (
          <span
            key={i}
            style={{
              padding: "2px 4px",
              margin: "0 1px",
              background: i === pos ? "#ffd866" : "transparent",
              borderBottom:
                i === pos ? "2px solid #c79500" : "2px solid transparent",
            }}
          >
            {c}
          </span>
        ))
      )}
    </div>
  );
}
