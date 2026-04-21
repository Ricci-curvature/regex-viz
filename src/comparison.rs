//! Stage 4: NFA simulator vs DFA simulator, side by side on the same input.
//!
//! Both engines consume the input one char at a time. Each step carries the
//! NFA active set and the DFA current state **after** processing that prefix.
//! A `summary` asserts both engines reach the same verdict; a mismatch is a
//! bug in `matcher::simulate` or `dfa::construct`, not a feature.
//!
//! Shape: `1 init + N consume + 1 verdict` (N = `input.chars().count()`).
//! Step count is stable across engines so the React slider aligns cleanly.
//! Once an engine is stuck (NFA active becomes ∅ / DFA has no transition)
//! we keep emitting dead steps — the slider length stays predictable and
//! the viewer can still render the empty / `null` state distinctly.
//!
//! Why a dedicated trace type? A plain `Trace` covers one engine. Reusing
//! `Nfa` to hold the DFA would lie about structure (DFA has many accept
//! states). So we pair a source `Nfa` with the Stage 3 `DfaState` /
//! `DfaTransition` snapshot + alphabet, and carry per-step active sets for
//! both. Format is frozen once the first artifact lands (CLAUDE.md rule 6).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::dfa::{DfaState, DfaTransition, construct_from_nfa};
use crate::nfa::{EPSILON, build_trace};
use crate::parser;
use crate::trace::Nfa;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonTrace {
    pub regex: String,
    pub input: String,
    pub nfa: Nfa,
    pub alphabet: Vec<char>,
    /// Final DFA snapshot (same shape as `ConstructionStep.dfa_states`).
    pub dfa_states: Vec<DfaState>,
    /// Final DFA transitions (same shape as `ConstructionStep.dfa_transitions`).
    pub dfa_transitions: Vec<DfaTransition>,
    pub steps: Vec<ComparisonStep>,
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonStep {
    pub description: String,
    /// Input position after this step (0 = before reading anything).
    pub input_pos: usize,
    /// Live NFA state ids (sorted). Empty once the NFA is stuck.
    pub nfa_active: Vec<usize>,
    /// Current DFA state id. `None` once the DFA has no outgoing edge for
    /// the consumed char.
    pub dfa_current: Option<usize>,
}

/// Machine contract: the final accept verdict from each engine, plus whether
/// they agree. The viewer reads this to decide badge colors; tests assert
/// `verdicts_agree` for every pinned (regex, input) pair.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ComparisonSummary {
    /// NFA accepts iff the accept state is in the active set after consuming
    /// every char without getting stuck mid-input.
    pub nfa_accepted: bool,
    /// DFA accepts iff the current state is an accept state after consuming
    /// every char without dying.
    pub dfa_accepted: bool,
    /// `nfa_accepted == dfa_accepted`. Hoisted so consumers don't recompute.
    pub verdicts_agree: bool,
}

pub fn run_comparison(src: &str, input: &str) -> Result<ComparisonTrace, String> {
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

    Ok(simulate(
        src.to_string(),
        input.to_string(),
        nfa,
        ctrace.alphabet,
        final_construction.dfa_states,
        final_construction.dfa_transitions,
    ))
}

fn simulate(
    regex: String,
    input: String,
    nfa: Nfa,
    alphabet: Vec<char>,
    dfa_states: Vec<DfaState>,
    dfa_transitions: Vec<DfaTransition>,
) -> ComparisonTrace {
    let chars: Vec<char> = input.chars().collect();
    let mut steps: Vec<ComparisonStep> = Vec::new();

    let mut nfa_cur: HashSet<usize> = epsilon_closure(&nfa, [nfa.start]);
    // D0 is the start state by construction (`construct_from_nfa` adds the
    // ε-closure({nfa.start}) subset first, so its id is always 0).
    let mut dfa_cur: Option<usize> = if dfa_states.is_empty() { None } else { Some(0) };

    steps.push(ComparisonStep {
        description: format!(
            "start: NFA ε-closure({{{}}}) = {{{}}}, DFA {}",
            nfa.start,
            fmt_ids_set(&nfa_cur),
            fmt_dfa(dfa_cur),
        ),
        input_pos: 0,
        nfa_active: sorted(&nfa_cur),
        dfa_current: dfa_cur,
    });

    for (i, c) in chars.iter().enumerate() {
        // NFA advance: char_move then ε-closure. Empty set stays empty.
        nfa_cur = if nfa_cur.is_empty() {
            HashSet::new()
        } else {
            epsilon_closure(&nfa, char_move(&nfa, &nfa_cur, *c))
        };
        // DFA advance: find the unique out-edge labeled `c`; `None` if absent.
        dfa_cur = dfa_cur.and_then(|id| {
            dfa_transitions
                .iter()
                .find(|t| t.from == id && t.label == *c)
                .map(|t| t.to)
        });

        let pos = i + 1;
        steps.push(ComparisonStep {
            description: format!(
                "consume '{c}' at pos {i} → NFA {{{}}}, DFA {}",
                fmt_ids_set(&nfa_cur),
                fmt_dfa(dfa_cur),
            ),
            input_pos: pos,
            nfa_active: sorted(&nfa_cur),
            dfa_current: dfa_cur,
        });
    }

    let nfa_accepted = nfa_cur.contains(&nfa.accept);
    let dfa_accepted = dfa_cur.map(|id| dfa_states[id].is_accept).unwrap_or(false);
    let verdicts_agree = nfa_accepted == dfa_accepted;

    steps.push(ComparisonStep {
        description: format_verdict(nfa_accepted, dfa_accepted, verdicts_agree),
        input_pos: chars.len(),
        nfa_active: sorted(&nfa_cur),
        dfa_current: dfa_cur,
    });

    ComparisonTrace {
        regex,
        input,
        nfa,
        alphabet,
        dfa_states,
        dfa_transitions,
        steps,
        summary: ComparisonSummary {
            nfa_accepted,
            dfa_accepted,
            verdicts_agree,
        },
    }
}

fn format_verdict(nfa_ok: bool, dfa_ok: bool, agree: bool) -> String {
    let nfa_word = if nfa_ok { "match" } else { "reject" };
    let dfa_word = if dfa_ok { "match" } else { "reject" };
    let tail = if agree {
        "agree"
    } else {
        "BUG: engines disagree"
    };
    format!("verdict: NFA {nfa_word}, DFA {dfa_word} — {tail}")
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

fn fmt_ids_set(set: &HashSet<usize>) -> String {
    let v = sorted(set);
    v.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn fmt_dfa(cur: Option<usize>) -> String {
    match cur {
        Some(id) => format!("D{id}"),
        None => "∅".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str, input: &str) -> ComparisonTrace {
        run_comparison(src, input).unwrap()
    }

    /// Canonical Stage 2 pin set — we reuse it here so a mismatch with the
    /// NFA-only matcher surfaces immediately.
    const PINS: &[(&str, &str)] = &[
        ("a", "a"),
        ("a", "b"),
        ("ab", "ab"),
        ("ab", "a"),
        ("ab", "abc"),
        ("a|b", "a"),
        ("a|b", "c"),
        ("a*", ""),
        ("a*", "aaa"),
        ("a*", "aab"),
        ("(a|b)*c", "aabc"),
        ("(a|b)*c", "abab"),
    ];

    #[test]
    fn engines_agree_on_every_stage2_pin() {
        for (regex, input) in PINS {
            let t = run(regex, input);
            assert!(
                t.summary.verdicts_agree,
                "{regex} × {input:?}: NFA {} vs DFA {}",
                t.summary.nfa_accepted, t.summary.dfa_accepted,
            );
        }
    }

    #[test]
    fn single_char_match_and_miss() {
        let m = run("a", "a");
        assert!(m.summary.nfa_accepted && m.summary.dfa_accepted);
        let n = run("a", "b");
        assert!(!n.summary.nfa_accepted && !n.summary.dfa_accepted);
    }

    #[test]
    fn concat_extra_char_rejects_on_both() {
        let t = run("ab", "abc");
        assert!(!t.summary.nfa_accepted);
        assert!(!t.summary.dfa_accepted);
        assert!(t.summary.verdicts_agree);
    }

    #[test]
    fn step_shape_is_init_plus_len_plus_verdict() {
        let t = run("(a|b)*c", "aabc");
        assert_eq!(t.steps.len(), 1 + 4 + 1);
        let t = run("a*", "");
        assert_eq!(t.steps.len(), 1 + 0 + 1);
    }

    #[test]
    fn step_count_is_stable_even_when_stuck_midway() {
        // `a*` on "aab" dies on 'b' (pos 2) but we keep emitting dead steps.
        let t = run("a*", "aab");
        assert_eq!(t.steps.len(), 1 + 3 + 1);
        // After the killing char, nfa_active is empty and dfa_current is None.
        let after_b = &t.steps[3]; // init(0) + 'a'(1) + 'a'(2) + 'b'(3)
        assert!(after_b.nfa_active.is_empty());
        assert!(after_b.dfa_current.is_none());
    }

    #[test]
    fn input_pos_monotonic_and_ends_at_len() {
        let t = run("(a|b)*c", "aabc");
        let positions: Vec<usize> = t.steps.iter().map(|s| s.input_pos).collect();
        for w in positions.windows(2) {
            assert!(w[0] <= w[1], "input_pos regressed: {positions:?}");
        }
        assert_eq!(*positions.last().unwrap(), "aabc".chars().count());
    }

    #[test]
    fn verdict_description_contains_agree_when_engines_agree() {
        let t = run("ab", "ab");
        assert!(t.summary.verdicts_agree);
        let tail = &t.steps.last().unwrap().description;
        assert!(tail.contains("match"), "description: {tail}");
        assert!(tail.contains("agree"), "description: {tail}");
    }

    #[test]
    fn verdict_description_calls_out_bug_when_disagree() {
        // We can't naturally produce a bug, so test the formatter directly.
        assert!(format_verdict(true, false, false).contains("BUG"));
        assert!(format_verdict(false, true, false).contains("BUG"));
        assert!(!format_verdict(true, true, true).contains("BUG"));
    }

    #[test]
    fn alphabet_and_dfa_snapshot_come_from_construction() {
        let t = run("(a|b)*c", "aabc");
        assert_eq!(t.alphabet, vec!['a', 'b', 'c']);
        // Start-state (D0) ID reference is consistent with Stage 3 contract.
        assert_eq!(t.steps[0].dfa_current, Some(0));
        // At least one accept exists (otherwise the regex would match nothing).
        assert!(t.dfa_states.iter().any(|s| s.is_accept));
    }

    #[test]
    fn comparison_matches_matcher_verdict_on_every_pin() {
        // Stronger check: our NFA arm in comparison must agree with the
        // standalone `matcher::simulate` verdict on the same pin.
        use crate::matcher;
        for (regex, input) in PINS {
            let cmp = run(regex, input);
            let tr = matcher::run_trace(regex, input).unwrap();
            let last = tr.steps.last().unwrap();
            let matcher_accepted = last.input_pos == Some(input.chars().count())
                && last.active.contains(&last.nfa.accept);
            assert_eq!(
                cmp.summary.nfa_accepted, matcher_accepted,
                "{regex} × {input:?}: comparison NFA={} vs matcher={}",
                cmp.summary.nfa_accepted, matcher_accepted,
            );
        }
    }
}
