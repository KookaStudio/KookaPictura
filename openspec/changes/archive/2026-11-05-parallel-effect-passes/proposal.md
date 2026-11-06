# Proposal

## Why

Cold banded composites serialize expensive layer-effect builds across layers,
even when independent styled layers could use separate workers (#1179).
The current composite plan resolves passes before any band runs, but its
sequential layer traversal preserves the reported bottleneck. The initial
scheduling fix still took about 7.7 seconds. The user expanded the goal to
100–1000 ms cold compositing while preserving the exact CPU output.

## What Changes

- Make independent layers in the banded compositor's plan eligible for
  concurrent resolution on the existing Rayon pool, retaining document order
  and one resolution per visible layer. Actual overlap depends on scheduling
  and the work available; a controlled fixture will prove concurrency capability.
- Build whole-layer effect passes through the existing content-keyed cache and
  retain them in the plan before rendering any band, including visible children
  of groups. Hidden subtrees remain unresolved.
- Keep the new scheduling confined to banded composites; ordinary whole-canvas,
  short-region, native-depth, and render-session plan entry points retain their
  current scheduling unless they already dispatch through the banded compositor.
- Combine with the exact Gaussian and bevel geometry optimizations from
  dependency PR #1221 (#1178), then profile and optimize remaining measured
  kernel or replay bottlenecks until the cold goal is met.
- Add exact-output, distinct-key build-count, controlled concurrency, shared-key
  completion, fallback, and profiling checks with per-call preparation selection.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/composite-plan`: Require independent layer-effect builds to be
  eligible for concurrent banded preparation while preserving ordered replay,
  existing shared-key cache semantics, visibility, and completion on one worker.
- `compositing/cpu-kernel-parallelism`: Resolve effect mask gates once, write
  matte buffers in bulk, and specialize exact row replay without altering pixels.

## Impact

- Engine-only changes in `crates/pictura-render/src/composite_plan.rs` and
  `composite_region.rs`; corresponding Rust tests and profiling checks.
- Existing source-cache synchronization, eviction, content keys, effect math,
  and blend order remain authoritative. No public API or dependency additions.
- Concurrent builds can increase peak temporary memory. Speedup depends on
  available workers and contention with each effect's existing parallel work.
- Coordinate benchmark baselines with #1178, which separately reduces the cost
  within one layer's bevel renderer; this change now measures their combined
  effect and addresses remaining latency.
