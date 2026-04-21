# viz — React components for trace visualization

Ten files, source only. Consuming sites (riccilab.dev blog) vendor these
directly; this repo does not ship its own `package.json`, bundler, or build
step. Runtime contract: React 18+, `dagre` (or `@dagrejs/dagre`) in scope.

```
trace.ts                — TypeScript mirror of src/trace.rs.
construction.ts         — TypeScript mirror of src/dfa.rs (Stage 3).
comparison.ts           — TypeScript mirror of src/comparison.rs (Stage 4).
minimization.ts         — TypeScript mirror of src/minimize.rs (Stage 5).
NfaGraph.tsx            — dagre layout + SVG for NFA. ~120 loc.
DfaGraph.tsx            — dagre layout + SVG for DFA. ~180 loc.
TraceViewer.tsx         — slider + NfaGraph (Stage 1/2). ~90 loc.
ConstructionViewer.tsx  — slider + NfaGraph + DfaGraph, 2-pane (Stage 3). ~130 loc.
ComparisonViewer.tsx    — slider + InputStrip + NfaGraph + DfaGraph (Stage 4). ~200 loc.
MinimizationViewer.tsx  — slider + totalized DfaGraph + minimized DfaGraph + partition snapshot (Stage 5). ~250 loc.
```

## Vendor into the blog

Copy (or symlink) these files into the blog repo's component tree, then:

```mdx
// Stage 1/2 — Build or Run trace
import traceAorBStarC from "../path/to/artifacts/stage01/a_or_b_star_c.json";
import { TraceViewer } from "../path/to/viz/TraceViewer";

<TraceViewer trace={traceAorBStarC} />

// Stage 3 — subset-construction trace (ConstructionTrace)
import subsetAorBStarC from "../path/to/artifacts/stage03/a_or_b_star_c.json";
import { ConstructionViewer } from "../path/to/viz/ConstructionViewer";

<ConstructionViewer trace={subsetAorBStarC} />

// Stage 4 — NFA vs DFA on the same input (ComparisonTrace)
import cmpAorBStarC from "../path/to/artifacts/stage04/a_or_b_star_c__match.json";
import { ComparisonViewer } from "../path/to/viz/ComparisonViewer";

<ComparisonViewer trace={cmpAorBStarC} />

// Stage 5 — Hopcroft DFA minimization (MinimizationTrace)
import minAorBStarC from "../path/to/artifacts/stage05/a_or_b_star_c.json";
import { MinimizationViewer } from "../path/to/viz/MinimizationViewer";

<MinimizationViewer trace={minAorBStarC} />
```

The blog's bundler resolves `dagre` from its own `node_modules`. If the blog
uses `@dagrejs/dagre` instead, alias the import in `NfaGraph.tsx` /
`DfaGraph.tsx` accordingly — API surface is identical.

## Contract

- Build trace (`kind: "build"`): `active` is empty in every step, `input_pos`
  is `null`. The NFA itself changes across steps.
- Run trace (`kind: "run"`): NFA is identical across steps. `active` carries
  the current state set; `input_pos` points at the next unread char.
- Both render through the same `<TraceViewer />`. `InputStrip` only appears
  for `kind === "run"`.
- Subset-construction trace (`ConstructionTrace`): carries source NFA +
  growing DFA. Rendered by `<ConstructionViewer />` — NFA on the left with
  `focus_nfa_subset` highlighted, DFA on the right with `focus_dfa_state`
  outlined. DFA states with `is_accept: true` get the double-circle marker;
  a single regex can therefore produce multiple accepts.
- Minimization trace (`MinimizationTrace`): carries the source DFA plus per
  (splitter, symbol) partition snapshots plus the final minimized DFA.
  Rendered by `<MinimizationViewer />` — totalized source DFA on the left
  (with synthetic sink node as dashed `∅`) colored by block, minimized DFA
  on the right (static, `sink_block` hidden). The partition strip below
  shows the current blocks as colored chips.

## Verification markers

Two `// TODO: verify` markers in `NfaGraph.tsx`:
1. Whether `dagre` is imported as default (`import dagre from "dagre"`) in
   the blog's bundler, or as named (`import * as dagre from …`).
2. Whether multigraph edge lookup via `g.edge(v, w, name)` (3-arg form) still
   works on the current dagre release — parallel ε-edges depend on it.

Both will be pinned by the first successful blog render. `DfaGraph.tsx`
uses the same dagre API and inherits whichever resolution lands there.
