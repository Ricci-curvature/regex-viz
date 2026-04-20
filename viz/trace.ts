// Mirror of src/trace.rs (serde JSON shape). Keep in sync with the Rust side —
// any field change here must be paired with a regenerate of artifacts/ on the
// Rust side, and vice versa.

export type TraceKind = "build" | "run";

export type Transition = {
  from: number;
  to: number;
  label: string;
};

export type Nfa = {
  states: number[];
  transitions: Transition[];
  start: number;
  accept: number;
};

export type Step = {
  description: string;
  nfa: Nfa;
  active: number[];
  input_pos: number | null;
};

export type Trace = {
  kind: TraceKind;
  input: string | null;
  steps: Step[];
};

export const EPSILON = "ε";
