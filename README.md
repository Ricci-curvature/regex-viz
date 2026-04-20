# regex-viz

**A Rust library that makes regex automata legible.** Not another regex engine — an attempt to make the *process* of matching visible, one step at a time.

Existing tools (Regexper, debuggex, regex101) visualize regex as a graph — the structure of an NFA as a static diagram. This project visualizes regex as a *timeline*: how Thompson's construction builds an NFA piece by piece, how active state sets propagate across an input, how catastrophic backtracking explodes, how subset construction collapses NFA states into DFA ones.

## Thesis

> Rust computes. MDX explains. SVG convinces.

Three layers, each with its own discipline:

- **Rust crate + CLI** — regex parsing, NFA/DFA construction, matching. No `regex` crate, no parser combinators. Bottom-up, because the point is to make internals visible.
- **Structured `Trace` JSON** — every stage emits a trace: a sequence of steps, each carrying an NFA snapshot and an active-state set. Build traces (how the NFA was assembled) and run traces (how a match proceeds) share one format.
- **React `TraceViewer` component** — ~50 lines, reads a trace JSON, renders a dagre-laid-out SVG with a step slider. The reader scrubs through time.
- **MDX blog posts on riccilab.dev** — narrative explanation with `<TraceViewer trace={...} />` embedded inline. Stage by stage.

## Architecture

```
regex-viz/                       ← this repo
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── parser.rs                ← regex string → AST (recursive descent, ~100 loc)
│   ├── nfa.rs                   ← AST → NFA (Thompson construction)
│   ├── dfa.rs                   ← subset construction (Stage 3)
│   ├── matcher.rs               ← NFA sim + backtracker (Stage 2 / 4)
│   └── trace.rs                 ← Trace / Step / Nfa serde types
├── examples/
│   ├── 01_build_nfa.rs          ← cargo run --example 01_build_nfa "a|b*c"
│   ├── 02_run_nfa.rs
│   ├── 03_to_dfa.rs
│   ├── 04_backtrack.rs
│   └── 05_bit_parallel.rs
├── artifacts/                   ← committed JSON output
│   ├── stage01/*.json
│   ├── stage02/*.json
│   └── ...
├── viz/                         ← React component (vendored into blog repo initially)
│   ├── TraceViewer.tsx
│   └── NfaGraph.tsx
└── tools/
    └── build_artifacts.sh       ← regenerate every trace in artifacts/

riccilab.dev (separate repo)
└── content/regex-viz/01-thompson.mdx
      ├── import trace from '.../artifacts/stage01/a_or_b.json'
      └── <TraceViewer trace={trace} />
```

## Trace Format

One Rust type, serialized as JSON, consumed by one React component. Build traces carry a full NFA snapshot per step (the automaton itself is changing); run traces carry the same NFA repeatedly with a different `active` set (the automaton is fixed, time is moving).

```rust
pub struct Trace {
    pub kind: TraceKind,         // Build | Run
    pub input: Option<String>,   // Some(..) for Run
    pub steps: Vec<Step>,
}

pub struct Step {
    pub description: String,     // "ε-closure of {0,1}", "read 'a'", "literal NFA", …
    pub nfa: Nfa,
    pub active: Vec<usize>,      // highlighted states (Run)
    pub input_pos: Option<usize>,
}

pub struct Nfa {
    pub states: Vec<usize>,
    pub transitions: Vec<Transition>,
    pub start: usize,
    pub accept: usize,
}

pub struct Transition { pub from: usize, pub to: usize, pub label: String }  // "a" | "ε" | "."
```

## Stage Roadmap

| Stage | Topic | Axis | Main artifact |
|---|---|---|---|
| 1 | Thompson construction | process (build) | 7 build traces: `a`, `ab`, `a\|b`, `a*`, `a+`, `(a\|b)*`, `a\|b*c` |
| 2 | NFA matching trace | process (run) | run traces — 3–5 regex × 2–3 input each |
| 3 | Subset construction → DFA | process (build) | NFA→DFA mapping trace, side-by-side static snapshot |
| 4 | Backtracking vs NFA, catastrophic cases | comparison | two-engine parallel trace — `(a+)+b` on `aaaaaaaaaa!` |
| 5 | Bit-parallel matching (Myers / Baeza-Yates) | alt representation | bitmask-as-state visualization |
| 6 | *(optional)* RE2-style bytecode VM | comparison | compiled opcodes + VM step trace |

Target: 5–6 stages, 5–6 blog posts, ~2–3 months. Repo freezes when the narrative arc closes (same discipline as the C++26 validator project).

## Scope Discipline

**In:** concat, alternation (`|`), `*`, `+`, `?`, grouping `(…)` (non-capturing), character classes `[…]`, `.`.

**Out:** capture groups, backreferences, lookaround, Unicode properties, named groups, flags. One later stage may *visualize why* backreferences break DFA construction — but regex-viz itself doesn't implement them.

## Build

```bash
cargo run --example 01_build_nfa "a|b*c" > artifacts/stage01/example.json

# regenerate every pinned artifact:
bash tools/build_artifacts.sh
```

CI (once added) verifies artifacts are up-to-date — regeneration stays local. Artifacts ship in the repo so every blog post is pinned to a reproducible commit.

## Requirements

- Rust 2024 edition (stable)
- No external regex dependencies

## Status

Stage 0 — planning complete. Stage 1 pending.
