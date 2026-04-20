//! Thompson's construction with a build trace.
//!
//! Each AST node, after being turned into its NFA fragment, emits one `Step`
//! capturing the full NFA so far. A reader scrubbing the slider watches the
//! automaton grow piece by piece: literal → operator → literal → …

use crate::parser::Ast;
use crate::trace::{Nfa, Step, Trace, TraceKind, Transition};

pub const EPSILON: &str = "ε";

#[derive(Debug, Clone, Copy)]
pub struct Fragment {
    pub start: usize,
    pub accept: usize,
}

pub struct Builder {
    next_id: usize,
    transitions: Vec<Transition>,
    steps: Vec<Step>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            transitions: Vec::new(),
            steps: Vec::new(),
        }
    }

    fn new_state(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn add_transition(&mut self, from: usize, to: usize, label: &str) {
        self.transitions.push(Transition {
            from,
            to,
            label: label.to_string(),
        });
    }

    /// Snapshot: all states allocated so far, all transitions so far, with the
    /// current fragment's endpoints as start/accept. Accumulating (not
    /// per-subtree) so sibling branches stay visible as the parent operator is
    /// introduced.
    fn snapshot(&self, frag: Fragment) -> Nfa {
        Nfa {
            states: (0..self.next_id).collect(),
            transitions: self.transitions.clone(),
            start: frag.start,
            accept: frag.accept,
        }
    }

    fn emit(&mut self, description: impl Into<String>, frag: Fragment) {
        let nfa = self.snapshot(frag);
        self.steps.push(Step {
            description: description.into(),
            nfa,
            active: Vec::new(),
            input_pos: None,
        });
    }

    pub fn build(&mut self, ast: &Ast) -> Fragment {
        match ast {
            Ast::Literal(c) => {
                let s = self.new_state();
                let a = self.new_state();
                self.add_transition(s, a, &c.to_string());
                let frag = Fragment { start: s, accept: a };
                self.emit(format!("literal '{}'", c), frag);
                frag
            }
            Ast::Concat(items) => {
                let mut iter = items.iter();
                let mut cur = self.build(iter.next().expect("concat is non-empty"));
                for item in iter {
                    let next = self.build(item);
                    self.add_transition(cur.accept, next.start, EPSILON);
                    cur = Fragment {
                        start: cur.start,
                        accept: next.accept,
                    };
                    self.emit("concat (ε-link)", cur);
                }
                cur
            }
            Ast::Alt(branches) => {
                let frags: Vec<Fragment> = branches.iter().map(|b| self.build(b)).collect();
                let new_start = self.new_state();
                let new_accept = self.new_state();
                for f in &frags {
                    self.add_transition(new_start, f.start, EPSILON);
                    self.add_transition(f.accept, new_accept, EPSILON);
                }
                let frag = Fragment {
                    start: new_start,
                    accept: new_accept,
                };
                self.emit("alternation", frag);
                frag
            }
            Ast::Star(inner) => {
                let f = self.build(inner);
                let new_start = self.new_state();
                let new_accept = self.new_state();
                self.add_transition(new_start, f.start, EPSILON);
                self.add_transition(new_start, new_accept, EPSILON);
                self.add_transition(f.accept, f.start, EPSILON);
                self.add_transition(f.accept, new_accept, EPSILON);
                let frag = Fragment {
                    start: new_start,
                    accept: new_accept,
                };
                self.emit("star (a*)", frag);
                frag
            }
            Ast::Plus(inner) => {
                let f = self.build(inner);
                let new_accept = self.new_state();
                self.add_transition(f.accept, f.start, EPSILON);
                self.add_transition(f.accept, new_accept, EPSILON);
                let frag = Fragment {
                    start: f.start,
                    accept: new_accept,
                };
                self.emit("plus (a+)", frag);
                frag
            }
            Ast::Question(inner) => {
                let f = self.build(inner);
                let new_start = self.new_state();
                let new_accept = self.new_state();
                self.add_transition(new_start, f.start, EPSILON);
                self.add_transition(new_start, new_accept, EPSILON);
                self.add_transition(f.accept, new_accept, EPSILON);
                let frag = Fragment {
                    start: new_start,
                    accept: new_accept,
                };
                self.emit("question (a?)", frag);
                frag
            }
        }
    }

    pub fn into_trace(self) -> Trace {
        Trace {
            kind: TraceKind::Build,
            input: None,
            steps: self.steps,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_trace(ast: &Ast) -> Trace {
    let mut b = Builder::new();
    b.build(ast);
    b.into_trace()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn trace_for(src: &str) -> Trace {
        build_trace(&parser::parse(src).unwrap())
    }

    #[test]
    fn literal_one_step() {
        let t = trace_for("a");
        assert_eq!(t.steps.len(), 1);
        let last = t.steps.last().unwrap();
        assert_eq!(last.nfa.states.len(), 2);
        assert_eq!(last.nfa.transitions.len(), 1);
        assert_eq!(last.nfa.transitions[0].label, "a");
    }

    #[test]
    fn concat_emits_per_link() {
        // "ab" → literal a, literal b, concat
        let t = trace_for("ab");
        assert_eq!(t.steps.len(), 3);
    }

    #[test]
    fn alt_final_has_two_epsilon_pairs() {
        // "a|b" → lit a, lit b, alt. Alt adds 4 ε-transitions (2 in, 2 out).
        let t = trace_for("a|b");
        let last = t.steps.last().unwrap();
        let eps = last
            .nfa
            .transitions
            .iter()
            .filter(|t| t.label == EPSILON)
            .count();
        assert_eq!(eps, 4);
        assert_eq!(last.nfa.states.len(), 6); // 2 per literal + 2 for alt frame
    }

    #[test]
    fn star_adds_loopback_and_bypass() {
        // "a*" → lit a, star. Star adds 4 ε-transitions.
        let t = trace_for("a*");
        let last = t.steps.last().unwrap();
        let eps = last
            .nfa
            .transitions
            .iter()
            .filter(|t| t.label == EPSILON)
            .count();
        assert_eq!(eps, 4);
    }

    #[test]
    fn plus_adds_loopback_no_bypass() {
        // "a+" → lit a, plus. Plus adds 2 ε-transitions (loopback + exit).
        let t = trace_for("a+");
        let last = t.steps.last().unwrap();
        let eps = last
            .nfa
            .transitions
            .iter()
            .filter(|t| t.label == EPSILON)
            .count();
        assert_eq!(eps, 2);
    }

    #[test]
    fn every_step_snapshot_is_self_consistent() {
        // Every step's start/accept must be in its states list, and every
        // transition's endpoints must be too.
        let traces = ["a", "ab", "a|b", "a*", "a+", "(a|b)*", "a|b*c"];
        for src in traces {
            let t = trace_for(src);
            for (i, step) in t.steps.iter().enumerate() {
                let states: std::collections::HashSet<_> =
                    step.nfa.states.iter().copied().collect();
                assert!(
                    states.contains(&step.nfa.start),
                    "{src} step {i}: start {} not in states",
                    step.nfa.start
                );
                assert!(
                    states.contains(&step.nfa.accept),
                    "{src} step {i}: accept {} not in states",
                    step.nfa.accept
                );
                for tr in &step.nfa.transitions {
                    assert!(
                        states.contains(&tr.from) && states.contains(&tr.to),
                        "{src} step {i}: transition {} -> {} references unknown state",
                        tr.from,
                        tr.to
                    );
                }
            }
        }
    }
}
