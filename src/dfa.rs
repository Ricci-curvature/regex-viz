//! Subset construction: NFA → DFA, step by step.
//!
//! Classical Rabin-Scott. A DFA state is a subset of NFA states; the initial
//! DFA state is ε-closure({nfa.start}). For each alphabet symbol c we take
//! ε-closure(char_move(subset, c)); a brand-new subset becomes a new DFA state
//! and gets enqueued. BFS terminates when the worklist drains.
//!
//! Why not reuse `Trace`? A `Step` carries a single `Nfa` with one accept
//! state. Subset construction needs BOTH the source NFA and the growing DFA,
//! and a DFA has potentially many accept states (every subset containing
//! `nfa.accept`). We ship a separate `ConstructionTrace` type so the existing
//! artifact format stays frozen (CLAUDE.md rule 6).

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::nfa::{EPSILON, build_trace};
use crate::parser;
use crate::trace::Nfa;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionTrace {
    pub regex: String,
    pub nfa: Nfa,
    /// Input alphabet Σ: every non-ε label appearing on NFA transitions,
    /// sorted and deduplicated. Stable order keeps the trace deterministic.
    pub alphabet: Vec<char>,
    pub steps: Vec<ConstructionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionStep {
    pub description: String,
    /// Accumulated DFA-state snapshot after this step.
    pub dfa_states: Vec<DfaState>,
    /// Accumulated DFA-transition snapshot after this step.
    pub dfa_transitions: Vec<DfaTransition>,
    /// The DFA state this step is centered on (newly created, or the source
    /// of a new transition). `None` only for the initial "compute Σ" step.
    pub focus_dfa_state: Option<usize>,
    /// The NFA subset the viewer should highlight on the left pane.
    pub focus_nfa_subset: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfaState {
    pub id: usize,
    /// NFA state ids making up this subset (sorted).
    pub subset: Vec<usize>,
    /// `true` iff the subset contains the NFA's accept state.
    pub is_accept: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DfaTransition {
    pub from: usize,
    pub to: usize,
    pub label: char,
}

pub fn construct(src: &str) -> Result<ConstructionTrace, String> {
    let ast = parser::parse(src)?;
    let built = build_trace(&ast);
    let nfa = built
        .steps
        .last()
        .ok_or("build trace produced no steps")?
        .nfa
        .clone();
    Ok(construct_from_nfa(src.to_string(), nfa))
}

pub fn construct_from_nfa(regex: String, nfa: Nfa) -> ConstructionTrace {
    let alphabet = compute_alphabet(&nfa);

    let mut states: Vec<DfaState> = Vec::new();
    let mut transitions: Vec<DfaTransition> = Vec::new();
    let mut steps: Vec<ConstructionStep> = Vec::new();

    // Step 0: announce the alphabet. No DFA state yet.
    steps.push(ConstructionStep {
        description: format!("alphabet Σ = {{{}}}", fmt_chars(&alphabet)),
        dfa_states: states.clone(),
        dfa_transitions: transitions.clone(),
        focus_dfa_state: None,
        focus_nfa_subset: Vec::new(),
    });

    // Step 1: initial DFA state from ε-closure({start}).
    let start_subset = sorted(&epsilon_closure(&nfa, [nfa.start]));
    let start_id = add_state(&mut states, &nfa, start_subset.clone());
    steps.push(ConstructionStep {
        description: format!(
            "D{start_id} = ε-closure({{{}}}) = {{{}}}",
            nfa.start,
            fmt_ids(&start_subset),
        ),
        dfa_states: states.clone(),
        dfa_transitions: transitions.clone(),
        focus_dfa_state: Some(start_id),
        focus_nfa_subset: start_subset,
    });

    // BFS worklist over DFA-state ids. `processed` prevents reprocessing a
    // state we've already expanded against every c ∈ Σ.
    let mut queue: VecDeque<usize> = VecDeque::from([start_id]);
    let mut processed: HashSet<usize> = HashSet::new();

    while let Some(cur_id) = queue.pop_front() {
        if !processed.insert(cur_id) {
            continue;
        }
        let cur_subset: HashSet<usize> = states[cur_id].subset.iter().copied().collect();

        for &c in &alphabet {
            let moved = char_move(&nfa, &cur_subset, c);
            if moved.is_empty() {
                continue;
            }
            let target_set = epsilon_closure(&nfa, moved);
            let target_sorted = sorted(&target_set);

            let (target_id, is_new) = match find_state(&states, &target_sorted) {
                Some(id) => (id, false),
                None => {
                    let id = add_state(&mut states, &nfa, target_sorted.clone());
                    queue.push_back(id);
                    (id, true)
                }
            };

            transitions.push(DfaTransition {
                from: cur_id,
                to: target_id,
                label: c,
            });

            let description = if is_new {
                format!(
                    "D{cur_id} --{c}--> D{target_id} (new: ε-closure(move(D{cur_id}, {c})) = {{{}}})",
                    fmt_ids(&target_sorted),
                )
            } else {
                format!(
                    "D{cur_id} --{c}--> D{target_id} (existing subset {{{}}})",
                    fmt_ids(&target_sorted),
                )
            };
            steps.push(ConstructionStep {
                description,
                dfa_states: states.clone(),
                dfa_transitions: transitions.clone(),
                focus_dfa_state: Some(target_id),
                focus_nfa_subset: target_sorted,
            });
        }
    }

    // Final step: list accept states so the verdict is visible without the
    // reader having to scan every subset.
    let accepts: Vec<usize> = states
        .iter()
        .filter(|s| s.is_accept)
        .map(|s| s.id)
        .collect();
    let verdict = if accepts.is_empty() {
        format!(
            "done: {} DFA state(s), 0 accept — regex matches nothing",
            states.len()
        )
    } else {
        format!(
            "done: {} DFA state(s), accept = {{{}}}",
            states.len(),
            fmt_ids(&accepts),
        )
    };
    steps.push(ConstructionStep {
        description: verdict,
        dfa_states: states.clone(),
        dfa_transitions: transitions.clone(),
        focus_dfa_state: None,
        focus_nfa_subset: Vec::new(),
    });

    ConstructionTrace {
        regex,
        nfa,
        alphabet,
        steps,
    }
}

fn compute_alphabet(nfa: &Nfa) -> Vec<char> {
    let mut set: HashSet<char> = HashSet::new();
    for t in &nfa.transitions {
        if t.label == EPSILON {
            continue;
        }
        // Stage 1-2 labels are always single chars (literal atoms only). If
        // a future stage adds multi-char labels this assertion surfaces it.
        let mut chars = t.label.chars();
        let c = chars.next().expect("non-ε label is non-empty");
        debug_assert!(
            chars.next().is_none(),
            "multi-char label {:?} not supported yet",
            t.label
        );
        set.insert(c);
    }
    let mut v: Vec<char> = set.into_iter().collect();
    v.sort_unstable();
    v
}

fn add_state(states: &mut Vec<DfaState>, nfa: &Nfa, subset: Vec<usize>) -> usize {
    let id = states.len();
    let is_accept = subset.contains(&nfa.accept);
    states.push(DfaState {
        id,
        subset,
        is_accept,
    });
    id
}

fn find_state(states: &[DfaState], subset: &[usize]) -> Option<usize> {
    states.iter().find(|s| s.subset == subset).map(|s| s.id)
}

fn epsilon_closure<I: IntoIterator<Item = usize>>(nfa: &Nfa, seeds: I) -> HashSet<usize> {
    let mut out: HashSet<usize> = seeds.into_iter().collect();
    let mut stack: Vec<usize> = out.iter().copied().collect();
    while let Some(s) = stack.pop() {
        for t in &nfa.transitions {
            if t.from == s && t.label == EPSILON && out.insert(t.to) {
                stack.push(t.to);
            }
        }
    }
    out
}

fn char_move(nfa: &Nfa, active: &HashSet<usize>, c: char) -> HashSet<usize> {
    let label = c.to_string();
    let mut out = HashSet::new();
    for t in &nfa.transitions {
        if active.contains(&t.from) && t.label == label {
            out.insert(t.to);
        }
    }
    out
}

fn sorted(set: &HashSet<usize>) -> Vec<usize> {
    let mut v: Vec<usize> = set.iter().copied().collect();
    v.sort_unstable();
    v
}

fn fmt_ids(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn fmt_chars(chars: &[char]) -> String {
    chars
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(src: &str) -> ConstructionTrace {
        construct(src).unwrap()
    }

    fn accept_count(t: &ConstructionTrace) -> usize {
        let last = t.steps.last().unwrap();
        last.dfa_states.iter().filter(|s| s.is_accept).count()
    }

    fn total_states(t: &ConstructionTrace) -> usize {
        t.steps.last().unwrap().dfa_states.len()
    }

    #[test]
    fn literal_two_dfa_states() {
        // `a`: D0 = {start, ...}, D1 = ε-closure of accept. Two states, one
        // transition, one accept.
        let t = build("a");
        assert_eq!(total_states(&t), 2);
        let last = t.steps.last().unwrap();
        assert_eq!(last.dfa_transitions.len(), 1);
        assert_eq!(last.dfa_transitions[0].label, 'a');
        assert_eq!(accept_count(&t), 1);
        assert_eq!(t.alphabet, vec!['a']);
    }

    #[test]
    fn concat_three_dfa_states() {
        // `ab` needs three DFA states: start, after-a, after-b (accept).
        let t = build("ab");
        assert_eq!(total_states(&t), 3);
        assert_eq!(accept_count(&t), 1);
        assert_eq!(t.alphabet, vec!['a', 'b']);
    }

    #[test]
    fn alt_has_three_dfa_states() {
        // `a|b`: D0 = {start, both branches}, D0 --a--> D1 (accept),
        // D0 --b--> D2 (accept). Three states total.
        let t = build("a|b");
        assert_eq!(total_states(&t), 3);
        assert_eq!(accept_count(&t), 2);
    }

    #[test]
    fn star_start_is_accept() {
        // `a*` matches ε, so the initial DFA state must already be accepting.
        let t = build("a*");
        let last = t.steps.last().unwrap();
        let d0 = &last.dfa_states[0];
        assert!(d0.is_accept, "D0 of a* must be accept (matches empty)");
    }

    #[test]
    fn plus_start_is_not_accept() {
        // `a+` requires at least one `a`, so D0 must not be accept.
        let t = build("a+");
        let last = t.steps.last().unwrap();
        assert!(!last.dfa_states[0].is_accept);
        // But some later state (after consuming an `a`) must be accept.
        assert!(last.dfa_states.iter().any(|s| s.is_accept));
    }

    #[test]
    fn grouped_star_concat() {
        // `(a|b)*c`: minimal DFA would have 2 states (loop + final), subset
        // construction may produce a few more before minimization. The shape
        // that matters: alphabet = {a, b, c}, at least one accept, start not
        // accept (needs at least a `c`), and determinism (see test below).
        let t = build("(a|b)*c");
        assert_eq!(t.alphabet, vec!['a', 'b', 'c']);
        assert!(accept_count(&t) >= 1);
        let last = t.steps.last().unwrap();
        assert!(!last.dfa_states[0].is_accept);
    }

    #[test]
    fn alphabet_is_sorted_and_deduplicated() {
        // `(a|b)*c` has labels a, b, c (each appearing once in the NFA,
        // already deduped, but sort order matters).
        let t = build("(a|b)*c");
        let mut expect = t.alphabet.clone();
        expect.sort_unstable();
        assert_eq!(t.alphabet, expect);
        // No duplicates.
        let unique: HashSet<char> = t.alphabet.iter().copied().collect();
        assert_eq!(unique.len(), t.alphabet.len());
    }

    #[test]
    fn result_is_deterministic() {
        // For every DFA state, for every symbol c ∈ Σ, at most one outgoing
        // transition labeled c. That's the whole point of "deterministic".
        for src in ["a", "ab", "a|b", "a*", "a+", "(a|b)*c", "a|b*c"] {
            let t = build(src);
            let last = t.steps.last().unwrap();
            let mut seen: HashSet<(usize, char)> = HashSet::new();
            for tr in &last.dfa_transitions {
                assert!(
                    seen.insert((tr.from, tr.label)),
                    "{src}: D{} has two {} transitions",
                    tr.from,
                    tr.label
                );
            }
        }
    }

    #[test]
    fn every_step_snapshot_is_monotonic() {
        // DFA states and transitions accumulate — never shrink.
        for src in ["a", "ab", "a|b", "a*", "(a|b)*c"] {
            let t = build(src);
            let mut prev_states = 0usize;
            let mut prev_trans = 0usize;
            for (i, step) in t.steps.iter().enumerate() {
                assert!(
                    step.dfa_states.len() >= prev_states,
                    "{src} step {i}: states regressed"
                );
                assert!(
                    step.dfa_transitions.len() >= prev_trans,
                    "{src} step {i}: transitions regressed"
                );
                prev_states = step.dfa_states.len();
                prev_trans = step.dfa_transitions.len();
            }
        }
    }

    #[test]
    fn ids_are_contiguous_from_zero() {
        let t = build("(a|b)*c");
        let last = t.steps.last().unwrap();
        for (i, s) in last.dfa_states.iter().enumerate() {
            assert_eq!(s.id, i);
        }
    }
}
