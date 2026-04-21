// Mirror of src/minimize.rs (serde JSON shape). Stage 5 takes the Stage 3
// subset-construction DFA, totalizes it against an implicit dead sink, and
// runs Hopcroft partition refinement. Each step is one (splitter, symbol)
// pair that caused at least one block to split.
//
// Sink semantics — must match the Rust module docstring:
// - Missing transitions in the source DFA are treated as going to a sink
//   state. Sink's id is `sink_id = source_dfa_states.length`.
// - Every partition / block / mapping entry in this trace references ids in
//   `0..=sink_id`.
// - `minimized.sink_block` tells the viewer which minimized state to hide
//   (or render dashed) and which incoming transitions to drop.

import type { DfaState, DfaTransition } from "./construction";
import type { Nfa } from "./trace";

export type SplitEvent = {
  parent: number[];
  child_in: number[]; // parent ∩ predecessors
  child_out: number[]; // parent - predecessors
};

export type MinimizationStep = {
  description: string;
  /** Sorted blocks, each a sorted list of totalized state ids. Stable
   * ordering by smallest element — blocks keep their visual slot across
   * steps when possible. */
  partition: number[][];
  /** The block popped from the worklist as this step's splitter. `null` at
   * the init step and the final verdict step. */
  splitter_block: number[] | null;
  /** Input symbol inspected this step. `null` at init/verdict. */
  symbol: string | null;
  /** The first split produced by this (splitter, symbol) pair. `null` at
   * init/verdict. When one step splits multiple blocks the viewer reads the
   * full `partition` snapshot; `split` just gives a representative for the
   * animation/caption. */
  split: SplitEvent | null;
};

export type MinimizedDfaState = {
  id: number;
  block: number[]; // totalized state ids absorbed into this minimized state
  is_accept: boolean;
  is_sink: boolean;
};

export type MinimizedDfa = {
  states: MinimizedDfaState[];
  transitions: DfaTransition[];
  /** `mapping[original_id] = minimized_id` for every id in `0..=sink_id`. */
  mapping: number[];
  /** Minimized state whose block contains the sink. Hide this (and edges
   * into it) to render the visible, partial minimal DFA. */
  sink_block: number;
};

export type MinimizationTrace = {
  regex: string;
  nfa: Nfa;
  alphabet: string[];
  source_dfa_states: DfaState[];
  source_dfa_transitions: DfaTransition[];
  /** Id of the added sink in the totalized view. Equal to
   * `source_dfa_states.length`. Convenient for viewers that want to highlight
   * which id is "the sink" in partition snapshots. */
  sink_id: number;
  steps: MinimizationStep[];
  minimized: MinimizedDfa;
};
