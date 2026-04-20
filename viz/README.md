# viz — React components for trace visualization

Three files, source only. Consuming sites (riccilab.dev blog) vendor these
directly; this repo does not ship its own `package.json`, bundler, or build
step. Runtime contract: React 18+, `dagre` (or `@dagrejs/dagre`) in scope.

```
trace.ts        — TypeScript mirror of src/trace.rs (serde JSON shape).
NfaGraph.tsx    — dagre layout + hand-rolled SVG. ~120 loc.
TraceViewer.tsx — step slider + NfaGraph composition. ~90 loc.
```

## Vendor into the blog

Copy (or symlink) these three files into the blog repo's component tree, then:

```mdx
import traceAorBStarC from "../path/to/artifacts/stage01/a_or_b_star_c.json";
import { TraceViewer } from "../path/to/viz/TraceViewer";

<TraceViewer trace={traceAorBStarC} />
```

The blog's bundler resolves `dagre` from its own `node_modules`. If the blog
uses `@dagrejs/dagre` instead, alias the import in `NfaGraph.tsx` accordingly
— API surface is identical.

## Contract

- Build trace: `active` is empty in every step, `input_pos` is `null`.
  The NFA itself changes across steps.
- Run trace: NFA is identical across steps. `active` carries the current
  state set; `input_pos` points at the next unread char.
- Both render through the same `<TraceViewer />`. `InputStrip` only appears
  for `kind === "run"`.

## Verification markers

Two `// TODO: verify` markers in `NfaGraph.tsx`:
1. Whether `dagre` is imported as default (`import dagre from "dagre"`) in
   the blog's bundler, or as named (`import * as dagre from …`).
2. Whether multigraph edge lookup via `g.edge(v, w, name)` (3-arg form) still
   works on the current dagre release — parallel ε-edges depend on it.

Both will be pinned by the first successful blog render.
