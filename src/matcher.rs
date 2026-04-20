//! NFA matcher: subset-construction advance with no backtracking.
//!
//! Drives the final NFA (last snapshot of `nfa::build_trace`) one input char
//! at a time: ε-closure of the start, then for each character a `char_move`
//! followed by another ε-closure. Every subset transition is recorded as a
//! `Step`, so the React viewer can highlight the active set as the slider
//! scrubs input positions.

use std::collections::HashSet;

use crate::nfa::{EPSILON, build_trace};
use crate::parser;
use crate::trace::{Nfa, Step, Trace, TraceKind};

pub fn run_trace(src: &str, input: &str) -> Result<Trace, String> {
    let ast = parser::parse(src)?;
    let built = build_trace(&ast);
    let nfa = built
        .steps
        .last()
        .ok_or("build trace produced no steps")?
        .nfa
        .clone();
    Ok(simulate(nfa, input))
}

pub fn simulate(nfa: Nfa, input: &str) -> Trace {
    let chars: Vec<char> = input.chars().collect();
    let mut steps: Vec<Step> = Vec::new();

    let mut current = epsilon_closure(&nfa, [nfa.start]);
    steps.push(Step {
        description: format!("start: ε-closure({{{}}})", nfa.start),
        nfa: nfa.clone(),
        active: sorted(&current),
        input_pos: Some(0),
    });

    let mut stuck_at: Option<usize> = None;
    for (i, c) in chars.iter().enumerate() {
        if current.is_empty() {
            stuck_at = Some(i);
            break;
        }
        let moved = char_move(&nfa, &current, *c);
        current = epsilon_closure(&nfa, moved);
        steps.push(Step {
            description: format!("consume '{c}' at pos {i} → {}", i + 1),
            nfa: nfa.clone(),
            active: sorted(&current),
            input_pos: Some(i + 1),
        });
    }

    let consumed = stuck_at.unwrap_or(chars.len());
    let matched = stuck_at.is_none() && current.contains(&nfa.accept);
    let verdict = if matched {
        format!(
            "match: accept {} active after consuming {consumed} chars",
            nfa.accept
        )
    } else if let Some(p) = stuck_at {
        let remaining: String = chars[p..].iter().collect();
        format!("mismatch: no live states at pos {p}; remaining {remaining:?}")
    } else {
        format!("mismatch: accept {} not active at end of input", nfa.accept)
    };
    steps.push(Step {
        description: verdict,
        nfa,
        active: sorted(&current),
        input_pos: Some(consumed),
    });

    Trace {
        kind: TraceKind::Run,
        input: Some(input.to_string()),
        steps,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(regex: &str, input: &str) -> Trace {
        run_trace(regex, input).unwrap()
    }

    fn accepts(t: &Trace) -> bool {
        let last = t.steps.last().unwrap();
        let expected_end = t.input.as_deref().unwrap_or("").chars().count();
        last.input_pos == Some(expected_end) && last.active.contains(&last.nfa.accept)
    }

    #[test]
    fn single_char() {
        assert!(accepts(&run("a", "a")));
        assert!(!accepts(&run("a", "b")));
        assert!(!accepts(&run("a", "")));
    }

    #[test]
    fn concat() {
        assert!(accepts(&run("ab", "ab")));
        assert!(!accepts(&run("ab", "a")));
        assert!(!accepts(&run("ab", "abc")));
    }

    #[test]
    fn alternation() {
        assert!(accepts(&run("a|b", "a")));
        assert!(accepts(&run("a|b", "b")));
        assert!(!accepts(&run("a|b", "c")));
    }

    #[test]
    fn star() {
        assert!(accepts(&run("a*", "")));
        assert!(accepts(&run("a*", "a")));
        assert!(accepts(&run("a*", "aaaa")));
        assert!(!accepts(&run("a*", "aab")));
    }

    #[test]
    fn plus() {
        assert!(!accepts(&run("a+", "")));
        assert!(accepts(&run("a+", "a")));
        assert!(accepts(&run("a+", "aaa")));
    }

    #[test]
    fn question() {
        assert!(accepts(&run("a?", "")));
        assert!(accepts(&run("a?", "a")));
        assert!(!accepts(&run("a?", "aa")));
    }

    #[test]
    fn grouped_star_concat() {
        assert!(accepts(&run("(a|b)*c", "c")));
        assert!(accepts(&run("(a|b)*c", "aabc")));
        assert!(accepts(&run("(a|b)*c", "abababc")));
        assert!(!accepts(&run("(a|b)*c", "abab")));
        assert!(!accepts(&run("(a|b)*c", "aabcd")));
    }

    #[test]
    fn step_shape() {
        // 1 init + N consume + 1 verdict
        let t = run("ab", "ab");
        assert_eq!(t.steps.len(), 1 + 2 + 1);
        assert!(t.steps.last().unwrap().description.starts_with("match"));

        let t = run("ab", "a");
        assert!(t.steps.last().unwrap().description.starts_with("mismatch"));
    }

    #[test]
    fn every_step_carries_same_nfa() {
        // Run trace must ship a stable NFA — the viewer draws one graph and
        // only recolors active states across steps.
        let t = run("(a|b)*c", "aabc");
        let first = &t.steps[0].nfa;
        for s in &t.steps {
            assert_eq!(s.nfa.states, first.states);
            assert_eq!(s.nfa.start, first.start);
            assert_eq!(s.nfa.accept, first.accept);
            assert_eq!(s.nfa.transitions.len(), first.transitions.len());
        }
    }

    #[test]
    fn input_pos_monotonic() {
        let t = run("(a|b)*c", "aabc");
        let positions: Vec<usize> = t.steps.iter().filter_map(|s| s.input_pos).collect();
        for w in positions.windows(2) {
            assert!(w[0] <= w[1], "input_pos regressed: {:?}", positions);
        }
    }
}
