# Proposal

## Why

Creating an empty layer takes about a second (#379), and nothing in the
application can say where that second goes. Two timing tools exist:
`paint_timing` in Rust and `pictura_debug_timing.h` in C++. Both are scoped to
paint strokes, print free text to stderr, and are not reachable from the
control server. ARCH-013's metrics instrumentation also does not exist (#651,
parity gap XC-11). Without a way to measure, every performance fix is a guess,
and there is nothing to prove a fix worked.

## What Changes

- Grow `paint_timing` into an opt-in metrics facility, enabled by
  `PICTURA_METRICS`. It records:
  - named phase timings, from Rust and from the Qt side;
  - one record per committed edit;
  - GPU counters: resident-upload hits and misses, declines by reason, and
    device time where the device supports timestamp queries;
  - the largest composite canvas allocated.

  When metrics are off it prints nothing and output does not change.
- Report metrics as structured stderr lines. Add a control-server `metrics`
  method and a `layer_op` `add_group` op, so `scripts/profile-tools.py --metrics`
  can print a per-phase breakdown for new `new-layer` and `new-group` scenarios.
- Fix the costs the profile shows for New Layer:
  - A new transaction outcome makes an empty layer or group insertion that is
    provably content-neutral skip the compositor.
  - The Layers panel no longer rebuilds its snapshot twice per panel-driven
    insertion.

  The fix covers the whole layer-creation family.
- Mark XC-11 as partial (metrics shipped). Open follow-up issues for the
  ARCH-013 modules this change does not build: frame-budget instrumentation,
  TileCache, scratch paging, the tile scheduler, and filter cancellation.

## Capabilities

### New Capabilities

- `verification/performance-metrics`: opt-in, inert-when-off runtime metrics,
  reported on stderr and through the control server.

### Modified Capabilities

- `document/edit-transaction`: adds a content-neutral insertion outcome that
  clears positional links, captures one state, and notifies, without running
  the compositor.
- `compositing/described-damage`: defines content-neutral layer and group
  insertion and its no-composite refresh.
- `compositing/layer-management`: "Management commands recomposite and record
  one state" exempts content-neutral insertion from the recomposite.
- `interop/agentic-control`: adds the `metrics` method and the `layer_op`
  `add_group` op.
- `verification/tool-profiler`: adds layer-creation scenarios and a `--metrics`
  per-phase breakdown.

## Impact

- `crates/pictura-app/src/cxxqt_object/helpers.rs`: `paint_timing` moves to a
  new `metrics.rs` and keeps its per-stroke report. Its existing call sites keep
  compiling.
- `crates/pictura-app/src/cxxqt_object/edit.rs` and `impl_layers*.rs`: a new
  edit outcome.
- `crates/pictura-render`: a counters module, a content-neutrality predicate,
  and optional timestamp queries (`wgpu::Features::TIMESTAMP_QUERY`, requested
  only when metrics are on and the adapter supports it).
- `crates/pictura-app/cpp/control_server_actions.cpp`,
  `crates/pictura-app/cpp/panels/layers_panel*.cpp`, and
  `pictura_debug_timing.h`: the metrics method, the `add_group` op, a coalesced
  panel refresh, and a Qt-side phase timer that feeds the Rust metrics.
- `scripts/profile-tools.py`, `ROADMAP.md`, `docs/dev/STATE.md`, and
  `docs/dev/doc-code-parity-gaps.md`. The docs changes go in a separate commit
  marked `TASK-ALLOWS-DOCS`.
- No new dependency.
