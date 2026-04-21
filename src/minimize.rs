//! Stage 5: Hopcroft DFA minimization.
//!
//! # Sink semantics (중요)
//!
//! Stage 3 의 subset construction 은 **partial DFA** 를 만듭니다. "현재 state
//! 에서 이 symbol 로 나가는 transition 이 없음" 을 그냥 transition 생략으로
//! 표현합니다. 고전 DFA minimization 은 total DFA 위에서 정의되므로, 이
//! module 은 **계산을 시작하기 전에 DFA 를 totalize** 합니다.
//!
//! - 원본 DFA 의 state id 는 `0..num_source_states`.
//! - `sink_id = num_source_states` 로 sink state 를 하나 추가합니다.
//! - 각 (state, symbol) 에 원래 transition 이 없었다면 그 자리에 sink 로 가는
//!   transition 을 채웁니다. sink 자신은 모든 symbol 로 self-loop.
//! - 이 total DFA 위에서 Hopcroft partition refinement 를 돌립니다.
//!
//! 결과 partition 에는 **sink 가 들어있는 block 이 반드시 하나** 있습니다.
//! 이 block 에 원본 DFA state 가 섞여 들어올 수도 있습니다 (예: accept 를
//! 지나쳐 더 이상 어디로도 못 가는 state 는 sink 와 같은 block 으로
//! 합쳐집니다). 뷰어는 이 block 을 `sink_block_id` 로 식별해 hide 하거나
//! dashed 로 렌더할 수 있습니다.
//!
//! # 알고리즘: 고전 Hopcroft, narration 은 splitter × symbol 단위
//!
//! 내부 자료구조는 Hopcroft 의 표준 worklist-of-splitters 입니다. 다만
//! `MinimizationStep` 은 "worklist 에서 splitter B 를 꺼내 symbol c 를 훑은
//! 결과" 하나씩 emit 해서, Moore 스타일의 round-by-round 가독성을 확보합니다.
//! 실제로 block 이 쪼개지지 않는 (splitter, symbol) 조합은 step 을 만들지
//! 않습니다 — slider 가 "변화가 있는 장면" 에만 멈추도록.

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::dfa::{DfaState, DfaTransition, construct_from_nfa};
use crate::nfa::build_trace;
use crate::parser;
use crate::trace::Nfa;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimizationTrace {
    pub regex: String,
    pub nfa: Nfa,
    pub alphabet: Vec<char>,
    /// The partial DFA from Stage 3 subset construction (unchanged).
    pub source_dfa_states: Vec<DfaState>,
    pub source_dfa_transitions: Vec<DfaTransition>,
    /// Added sink's id in the totalized view. Always `source_dfa_states.len()`.
    /// All partition snapshots and the minimized block contents reference ids
    /// in `0..=sink_id`, where `sink_id` is the sink.
    pub sink_id: usize,
    pub steps: Vec<MinimizationStep>,
    pub minimized: MinimizedDfa,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimizationStep {
    pub description: String,
    /// Current partition. Each inner Vec is a sorted block of state ids
    /// (source ids 0..num_source, plus sink_id). The blocks themselves are
    /// ordered by their smallest element for stable diff.
    pub partition: Vec<Vec<usize>>,
    /// The block used as splitter this step, referenced by its content (not
    /// index — indices shift across steps). `None` for the init step and the
    /// final verdict step.
    pub splitter_block: Option<Vec<usize>>,
    /// Symbol examined this step. `None` for init/verdict.
    pub symbol: Option<char>,
    /// When a split happened: the (parent_block, child1, child2) that were
    /// produced. Blocks listed by content for the same stability reason.
    /// `None` for init / verdict. Steps that don't split are not emitted at
    /// all (see module docstring).
    pub split: Option<SplitEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitEvent {
    pub parent: Vec<usize>,
    pub child_in: Vec<usize>,  // parent ∩ predecessors
    pub child_out: Vec<usize>, // parent - predecessors
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimizedDfa {
    pub states: Vec<MinimizedDfaState>,
    pub transitions: Vec<DfaTransition>,
    /// `mapping[original_id] = minimized_id` for every id in `0..=sink_id`
    /// (so index `sink_id` tells you which minimized state the sink belongs
    /// to).
    pub mapping: Vec<usize>,
    /// Id of the minimized state whose block contains the sink. Viewer may
    /// choose to hide this state and its incoming transitions.
    pub sink_block: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimizedDfaState {
    pub id: usize,
    /// Source DFA state ids (sorted) that collapsed into this minimized
    /// state. For the sink block this includes `sink_id`; it may also
    /// include real source states that ended up equivalent to sink.
    pub block: Vec<usize>,
    pub is_accept: bool,
    /// `true` iff this minimized state is the block containing the added
    /// sink. Same as `id == minimized.sink_block`.
    pub is_sink: bool,
}

pub fn minimize(src: &str) -> Result<MinimizationTrace, String> {
    let ast = parser::parse(src)?;
    let built = build_trace(&ast);
    let nfa = built
        .steps
        .last()
        .ok_or("build trace produced no steps")?
        .nfa
        .clone();
    let ctrace = construct_from_nfa(src.to_string(), nfa.clone());
    let final_construction = ctrace
        .steps
        .last()
        .expect("subset construction always emits a verdict step")
        .clone();

    Ok(run_hopcroft(
        src.to_string(),
        nfa,
        ctrace.alphabet,
        final_construction.dfa_states,
        final_construction.dfa_transitions,
    ))
}

fn run_hopcroft(
    regex: String,
    nfa: Nfa,
    alphabet: Vec<char>,
    source_states: Vec<DfaState>,
    source_transitions: Vec<DfaTransition>,
) -> MinimizationTrace {
    let num_source = source_states.len();
    let sink_id = num_source;
    let total_states = num_source + 1;

    // Totalize: build a dense transition table [state][alphabet_idx] -> target.
    // Missing transitions point at `sink_id`; sink self-loops on every symbol.
    let alpha_index: std::collections::HashMap<char, usize> =
        alphabet.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let mut table: Vec<Vec<usize>> = vec![vec![sink_id; alphabet.len()]; total_states];
    for t in &source_transitions {
        let ai = alpha_index[&t.label];
        table[t.from][ai] = t.to;
    }
    // Sink self-loops — already the default `sink_id` fill; explicit for clarity.
    for ai in 0..alphabet.len() {
        table[sink_id][ai] = sink_id;
    }

    // Accept-ness for all totalized states. Sink is non-accept by construction.
    let mut is_accept: Vec<bool> = vec![false; total_states];
    for s in &source_states {
        is_accept[s.id] = s.is_accept;
    }

    // Initial partition: {accepts} | {non-accepts}. Sink always in non-accept.
    let accepts: Vec<usize> = (0..total_states).filter(|&i| is_accept[i]).collect();
    let non_accepts: Vec<usize> = (0..total_states).filter(|&i| !is_accept[i]).collect();
    let mut partition: Vec<Vec<usize>> = Vec::new();
    if !accepts.is_empty() {
        partition.push(sorted_vec(accepts.clone()));
    }
    partition.push(sorted_vec(non_accepts.clone()));
    partition = stable_sort_blocks(partition);

    let mut steps: Vec<MinimizationStep> = Vec::new();
    steps.push(MinimizationStep {
        description: format!(
            "initial partition: {{accept}} | {{non-accept}} = {}",
            fmt_partition(&partition),
        ),
        partition: partition.clone(),
        splitter_block: None,
        symbol: None,
        split: None,
    });

    // Worklist of splitter blocks. Hopcroft's optimization: start with the
    // smaller of the two initial blocks. If partition has only 1 block (all
    // states are accept or all are non-accept) there's nothing to split.
    let mut worklist: VecDeque<Vec<usize>> = VecDeque::new();
    if partition.len() >= 2 {
        if accepts.len() <= non_accepts.len() {
            worklist.push_back(sorted_vec(accepts));
        } else {
            worklist.push_back(sorted_vec(non_accepts));
        }
    }

    while let Some(splitter) = worklist.pop_front() {
        // `splitter` is a snapshot — the current partition may have split it
        // further since we enqueued. We use its *contents* to compute
        // predecessors; the snapshot remains a valid "states that all behave
        // equivalently on this symbol" check because any refinement of the
        // snapshot is still a refinement.
        let splitter_set: HashSet<usize> = splitter.iter().copied().collect();

        for (ai, &c) in alphabet.iter().enumerate() {
            // Predecessors X on symbol c: all states whose c-transition lands
            // inside `splitter`.
            let predecessors: HashSet<usize> = (0..total_states)
                .filter(|&s| splitter_set.contains(&table[s][ai]))
                .collect();
            if predecessors.is_empty() {
                continue;
            }

            // Walk the current partition looking for blocks split by X.
            let mut new_partition: Vec<Vec<usize>> = Vec::with_capacity(partition.len() + 2);
            let mut emitted_this_symbol: Vec<SplitEvent> = Vec::new();

            for block in &partition {
                let (inside, outside): (Vec<usize>, Vec<usize>) =
                    block.iter().partition(|id| predecessors.contains(id));
                if inside.is_empty() || outside.is_empty() {
                    new_partition.push(block.clone());
                    continue;
                }
                // Split! Record event + both children are the new blocks.
                let inside = sorted_vec(inside);
                let outside = sorted_vec(outside);
                emitted_this_symbol.push(SplitEvent {
                    parent: block.clone(),
                    child_in: inside.clone(),
                    child_out: outside.clone(),
                });
                new_partition.push(inside.clone());
                new_partition.push(outside.clone());

                // Worklist update: if `block` was in the worklist, replace
                // with both halves; else push the smaller half (Hopcroft's
                // O(n log n) trick).
                let was_queued = worklist.iter().any(|b| b == block);
                if was_queued {
                    worklist.retain(|b| b != block);
                    worklist.push_back(inside.clone());
                    worklist.push_back(outside.clone());
                } else if inside.len() <= outside.len() {
                    worklist.push_back(inside.clone());
                } else {
                    worklist.push_back(outside.clone());
                }
            }

            if emitted_this_symbol.is_empty() {
                // Nothing split on this (splitter, symbol). Don't emit a step.
                continue;
            }

            partition = stable_sort_blocks(new_partition);

            // We emit one MinimizationStep per (splitter, symbol) that
            // actually caused at least one split. If multiple blocks split
            // on the same (splitter, symbol), the description lists them
            // all and `split` carries the first — viewer reads the partition
            // snapshot to see the full picture.
            let description = if emitted_this_symbol.len() == 1 {
                let ev = &emitted_this_symbol[0];
                format!(
                    "splitter {{{}}} on '{c}': predecessors split {{{}}} → {{{}}} | {{{}}}",
                    fmt_ids(&splitter),
                    fmt_ids(&ev.parent),
                    fmt_ids(&ev.child_in),
                    fmt_ids(&ev.child_out),
                )
            } else {
                format!(
                    "splitter {{{}}} on '{c}': split {} blocks ({})",
                    fmt_ids(&splitter),
                    emitted_this_symbol.len(),
                    emitted_this_symbol
                        .iter()
                        .map(|e| format!(
                            "{{{}}}→{{{}}}|{{{}}}",
                            fmt_ids(&e.parent),
                            fmt_ids(&e.child_in),
                            fmt_ids(&e.child_out),
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            steps.push(MinimizationStep {
                description,
                partition: partition.clone(),
                splitter_block: Some(splitter.clone()),
                symbol: Some(c),
                split: Some(emitted_this_symbol.into_iter().next().unwrap()),
            });
        }
    }

    // Build minimized DFA from the final partition.
    let minimized = build_minimized(&partition, &table, &alphabet, &is_accept, sink_id);

    let verdict = format!(
        "fixed point: {} block(s) → {}-state minimal DFA{}",
        partition.len(),
        minimized.states.len(),
        if minimized
            .states
            .iter()
            .any(|s| s.is_sink && s.block.len() > 1)
        {
            " (sink block absorbed non-live source states)"
        } else {
            ""
        },
    );
    steps.push(MinimizationStep {
        description: verdict,
        partition: partition.clone(),
        splitter_block: None,
        symbol: None,
        split: None,
    });

    MinimizationTrace {
        regex,
        nfa,
        alphabet,
        source_dfa_states: source_states,
        source_dfa_transitions: source_transitions,
        sink_id,
        steps,
        minimized,
    }
}

fn build_minimized(
    partition: &[Vec<usize>],
    table: &[Vec<usize>],
    alphabet: &[char],
    is_accept: &[bool],
    sink_id: usize,
) -> MinimizedDfa {
    // Determine the canonical order of blocks in the minimized DFA: the block
    // containing the original start state (id 0) gets id 0, then the rest in
    // partition order. This keeps the minimized DFA "start at 0" consistent
    // with the source DFA convention.
    let start_block = partition
        .iter()
        .position(|b| b.contains(&0))
        .expect("start state must be in some block");

    let mut block_order: Vec<usize> = Vec::with_capacity(partition.len());
    block_order.push(start_block);
    for i in 0..partition.len() {
        if i != start_block {
            block_order.push(i);
        }
    }

    // Mapping: original_id -> minimized_id.
    let total_states = table.len();
    let mut mapping: Vec<usize> = vec![0; total_states];
    for (new_id, &block_idx) in block_order.iter().enumerate() {
        for &orig in &partition[block_idx] {
            mapping[orig] = new_id;
        }
    }

    // Sink block in minimized id space.
    let sink_block = mapping[sink_id];

    // Build states.
    let mut states: Vec<MinimizedDfaState> = Vec::with_capacity(partition.len());
    for (new_id, &block_idx) in block_order.iter().enumerate() {
        let block = partition[block_idx].clone();
        // "is_accept" is the same for every member by construction (refinement
        // never merges accept with non-accept). Pick any member.
        let accept_flag = block.iter().any(|&id| is_accept[id]);
        debug_assert!(
            block.iter().all(|&id| is_accept[id] == accept_flag),
            "block {block:?} mixes accept and non-accept — minimization bug",
        );
        states.push(MinimizedDfaState {
            id: new_id,
            block,
            is_accept: accept_flag,
            is_sink: new_id == sink_block,
        });
    }

    // Build transitions. For each minimized state (= representative block),
    // each symbol has a single target (determinism preserved). Skip
    // transitions from the sink block entirely — the viewer hides them.
    // Transitions INTO the sink are also skipped for the visible graph; they
    // are implicit. Consumers who need the full totalized minimized DFA can
    // reconstruct them from `mapping`.
    let mut transitions: Vec<DfaTransition> = Vec::new();
    for (new_id, &block_idx) in block_order.iter().enumerate() {
        if new_id == sink_block {
            continue;
        }
        // Pick any representative from the block (first element after sort).
        let rep = partition[block_idx][0];
        for (ai, &c) in alphabet.iter().enumerate() {
            let target_orig = table[rep][ai];
            let target_new = mapping[target_orig];
            if target_new == sink_block {
                // Missing transition in the visible minimized DFA.
                continue;
            }
            transitions.push(DfaTransition {
                from: new_id,
                to: target_new,
                label: c,
            });
        }
    }

    MinimizedDfa {
        states,
        transitions,
        mapping,
        sink_block,
    }
}

fn stable_sort_blocks(mut blocks: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    // Each block is already sorted by add_state convention; we sort blocks
    // among themselves by their smallest element. Keeps diffs between step
    // partitions readable.
    blocks.sort_by_key(|b| b.first().copied().unwrap_or(usize::MAX));
    blocks
}

fn sorted_vec(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v.dedup();
    v
}

fn fmt_ids(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn fmt_partition(partition: &[Vec<usize>]) -> String {
    partition
        .iter()
        .map(|b| format!("{{{}}}", fmt_ids(b)))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(src: &str) -> MinimizationTrace {
        minimize(src).unwrap()
    }

    fn visible_count(t: &MinimizationTrace) -> usize {
        // Count of minimized states excluding the sink block iff sink block
        // contains only the added sink (no real source states absorbed).
        let sink = &t.minimized.states[t.minimized.sink_block];
        if sink.block == vec![t.sink_id] {
            t.minimized.states.len() - 1
        } else {
            t.minimized.states.len()
        }
    }

    #[test]
    fn literal_already_minimal() {
        // `a` — source DFA has 2 states; after totalization+minimization we
        // expect 3 states {start} | {accept} | {sink}. Visible = 2.
        let t = build("a");
        assert_eq!(t.minimized.states.len(), 3);
        assert_eq!(visible_count(&t), 2);
    }

    #[test]
    fn alt_accepts_merge() {
        // `a|b`: source DFA 3 states (D0 non-accept, D1 accept, D2 accept).
        // D1 and D2 are both dead-end accepts → merge. Visible = 2.
        let t = build("a|b");
        assert_eq!(visible_count(&t), 2);
        // Mapping: D1 and D2 should point at the same minimized state.
        assert_eq!(t.minimized.mapping[1], t.minimized.mapping[2]);
    }

    #[test]
    fn concat_already_minimal() {
        // `ab`: 3 distinct states, nothing to merge.
        let t = build("ab");
        assert_eq!(visible_count(&t), 3);
    }

    #[test]
    fn star_collapses_to_single_accept_loop() {
        // `a*` — source DFA has 2 accept states (D0 start-accept, D1 after-a
        // accept, both self-looping on 'a'). They are equivalent (both accept,
        // both loop to an accept), so they merge. Visible minimal DFA = 1
        // self-looping accept state.
        let t = build("a*");
        assert_eq!(visible_count(&t), 1);
        // D0 and D1 collapse to the same minimized state.
        assert_eq!(t.minimized.mapping[0], t.minimized.mapping[1]);
    }

    #[test]
    fn plus_already_minimal() {
        // `a+` — source DFA has D0 (non-accept) and D1 (accept, self-loop).
        // Not equivalent (different accept status). Visible = 2.
        let t = build("a+");
        assert_eq!(visible_count(&t), 2);
    }

    #[test]
    fn grouped_star_concat_merges_loop_states() {
        // `(a|b)*c` — source DFA 4 states. D1, D2 are symmetric loop states
        // and should merge. D0 is the initial loop state, also equivalent.
        // Expected visible: start-loop block + accept = 2.
        // The blog discusses the 4→2 vs 4→3 distinction (implicit sink).
        let t = build("(a|b)*c");
        assert_eq!(visible_count(&t), 2);
        // Accept is alone in its block (D3).
        assert_eq!(t.minimized.mapping[3], 1.min(t.minimized.states.len() - 1));
    }

    #[test]
    fn aa_or_ab_merges_leaf_accepts() {
        // `aa|ab`: D0 --a--> D1 --a--> D_accept, D1 --b--> D_accept2.
        // The two accept leaves merge. Visible = 3 (start, middle, merged-accept).
        let t = build("aa|ab");
        assert_eq!(visible_count(&t), 3);
    }

    #[test]
    fn abc_or_axc_merges_symmetric_prefix_states() {
        // `abc|axc`: states after reading "ab" and after reading "ax" are
        // symmetric (both lead to accept on 'c'). They should merge. Visible
        // count should be 4: start, after-a, merged-after-ab-or-ax, accept.
        let t = build("abc|axc");
        assert_eq!(visible_count(&t), 4);
    }

    #[test]
    fn ab_twice_minimizes_by_length() {
        // `(a|b)(a|b)` — strings of length exactly 2 over {a,b}. Minimal DFA
        // has 3 visible states (length 0 / length 1 / length 2 = accept).
        let t = build("(a|b)(a|b)");
        assert_eq!(visible_count(&t), 3);
    }

    #[test]
    fn sink_block_always_exists() {
        // After totalization+Hopcroft, some block contains the sink id.
        for src in ["a", "ab", "a|b", "a*", "a+", "(a|b)*c", "aa|ab", "abc|axc"] {
            let t = build(src);
            let sink_state = &t.minimized.states[t.minimized.sink_block];
            assert!(
                sink_state.block.contains(&t.sink_id),
                "{src}: sink block {:?} does not contain sink_id {}",
                sink_state.block,
                t.sink_id,
            );
            assert!(
                !sink_state.is_accept,
                "{src}: sink block must be non-accept"
            );
        }
    }

    #[test]
    fn mapping_preserves_acceptance() {
        // Every source state s has is_accept == minimized.states[mapping[s]].is_accept.
        for src in ["a", "ab", "a|b", "(a|b)*c", "aa|ab", "abc|axc"] {
            let t = build(src);
            for s in &t.source_dfa_states {
                let mapped = t.minimized.mapping[s.id];
                assert_eq!(
                    t.minimized.states[mapped].is_accept, s.is_accept,
                    "{src}: source state {} accept={} but minimized state {} accept={}",
                    s.id, s.is_accept, mapped, t.minimized.states[mapped].is_accept,
                );
            }
        }
    }

    #[test]
    fn minimized_is_deterministic() {
        for src in [
            "a",
            "ab",
            "a|b",
            "(a|b)*c",
            "aa|ab",
            "abc|axc",
            "(a|b)(a|b)",
        ] {
            let t = build(src);
            let mut seen: HashSet<(usize, char)> = HashSet::new();
            for tr in &t.minimized.transitions {
                assert!(
                    seen.insert((tr.from, tr.label)),
                    "{src}: minimized state {} has two '{}' transitions",
                    tr.from,
                    tr.label,
                );
            }
        }
    }

    #[test]
    fn partition_refinement_only_splits() {
        // Every non-init step must produce a partition that is a refinement
        // of the previous step's partition (never coarser).
        for src in ["a", "a|b", "(a|b)*c", "aa|ab", "abc|axc", "(a|b)(a|b)"] {
            let t = build(src);
            for w in t.steps.windows(2) {
                assert!(
                    w[1].partition.len() >= w[0].partition.len(),
                    "{src}: block count decreased from {} to {}",
                    w[0].partition.len(),
                    w[1].partition.len(),
                );
            }
        }
    }

    #[test]
    fn step_sequence_starts_with_init_ends_with_verdict() {
        for src in ["a", "(a|b)*c", "aa|ab"] {
            let t = build(src);
            assert!(t.steps[0].splitter_block.is_none());
            assert!(t.steps[0].symbol.is_none());
            let last = t.steps.last().unwrap();
            assert!(last.splitter_block.is_none());
            assert!(last.description.starts_with("fixed point"));
        }
    }
}
