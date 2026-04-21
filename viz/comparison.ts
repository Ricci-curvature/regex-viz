// Mirror of src/comparison.rs (serde JSON shape). Stage 4 pairs an NFA
// simulator with a DFA simulator on the same input; each step records the
// active set / current state for both engines so the viewer can render them
// side by side with a single slider.

import type { DfaState, DfaTransition } from "./construction";
import type { Nfa } from "./trace";

export type ComparisonStep = {
  description: string;
  /** Input position after this step (0 = before reading anything). */
  input_pos: number;
  /** Live NFA state ids (sorted). Empty once the NFA is stuck. */
  nfa_active: number[];
  /** Current DFA state id. `null` once the DFA has no outgoing edge. */
  dfa_current: number | null;
};

export type ComparisonSummary = {
  nfa_accepted: boolean;
  dfa_accepted: boolean;
  verdicts_agree: boolean;
};

export type ComparisonTrace = {
  regex: string;
  input: string;
  nfa: Nfa;
  alphabet: string[]; // sorted, deduplicated (serde emits `char` as 1-length string)
  dfa_states: DfaState[];
  dfa_transitions: DfaTransition[];
  steps: ComparisonStep[];
  summary: ComparisonSummary;
};
