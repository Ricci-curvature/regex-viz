// Mirror of src/dfa.rs (serde JSON shape). Separate from trace.ts because
// subset construction ships both the source NFA and a growing DFA snapshot
// per step — the existing `Trace`/`Step` pair can't express that without
// breaking its format pin (CLAUDE.md rule 6).

import type { Nfa } from "./trace";

export type DfaState = {
  id: number;
  subset: number[]; // sorted NFA state ids
  is_accept: boolean;
};

export type DfaTransition = {
  from: number;
  to: number;
  label: string; // single char; serde emits `char` as a 1-length string
};

export type ConstructionStep = {
  description: string;
  dfa_states: DfaState[];
  dfa_transitions: DfaTransition[];
  focus_dfa_state: number | null;
  focus_nfa_subset: number[];
};

export type ConstructionTrace = {
  regex: string;
  nfa: Nfa;
  alphabet: string[]; // sorted, deduplicated
  steps: ConstructionStep[];
};
