# Design

## Context

See proposal.md for why this change exists. These are the parts of the current
code that shape the approach.

- **`paint_timing`** (`crates/pictura-app/src/cxxqt_object/helpers.rs:691`):
  - a thread-local phase accumulator scoped to one stroke session, gated by
    `PICTURA_PAINT_TIMING` and read once into a `OnceLock`;
  - about 60 call sites outside tests, each written as
    `let t = Instant::now(); …; paint_timing::record(label, t.elapsed())`.
- **`pictura_debug_timing.h`**: the Qt-side `ScopedTimer`. It prints
  `[paint-timing]` lines and does not reach the Rust accumulator.
- **The New Layer path from the panel**:
  1. `LayersPanel::addLayerAt` calls `clearSolo()`, then `add_layer_in`.
  2. `edit("New Layer")` runs with `Edit::Structural`, which calls
     `composite_full()` (edit.rs:78). That composites, stores the composite,
     and runs `reset_pyramid`, which rebuilds the view pyramid and the
     mask-solo source and bumps `frame_revision`.
  3. `session.record()` captures history.
  4. `changed()` makes the panel queue a `refresh()` (layers_panel.cpp:491).
  5. `addLayerAt` then calls `refresh()` synchronously as well, so the snapshot
     and thumbnails are built twice.
- **`Effect::Notify` and `Effect::Rebuild`** both only emit `changed()` in
  C++. The canvas re-presents when `frame_revision` changes.
- **Clipping** (composite_clipping.rs:31): a clipped run attaches to the
  nearest unclipped sibling below it. A base with a run of zero composites
  directly. A base with a run of one or more composites through an isolated
  group, so the first clipped layer changes how the base is computed even when
  that layer is empty.
- **`current_buffer`** returns the stored composite when the document has no
  layers (helpers_composite.rs:210).
- **`transparent_layer`** allocates W×H×4 zeroed bytes per new layer. This
  change does not touch it (see Non-Goals).

The suspects for #379, none measured yet:

```
A composite_full ............ cost scales with the document and the layer count
B session.record ............ history capture
C reset_pyramid ............. view pyramid rebuild, at full resolution
D double panel refresh ...... snapshot and thumbnails, built twice
E transparent_layer ......... W×H×4 zeroed allocation
```

## Goals / Non-Goals

**Goals:**
- Metrics that print and retain nothing when off and never change output.
- Remove A, C, and D for content-neutral insertions, and measure B and E.
- A profiler scenario that shows the breakdown before and after.

**Non-Goals:**
- Empty-rect new layers (fixing E). That changes the layer model, PSD output,
  and paint, fill, and transform. If the profile shows E dominating, it becomes
  its own change with its own specs.
- ARCH-013 frame-budget instrumentation, TileCache, scratch paging, the tile
  scheduler, and filter cancellation. Each gets a follow-up issue.
- Hard latency assertions in CI. The tests assert structure instead: no
  composite ran, the frame revision did not change, the bytes are identical.

## Decisions

### D1: One `metrics` module in the app, always-on counters in render

`cxxqt_object/metrics.rs` owns the metrics state. `paint_timing` stays in
`helpers.rs`, and its `record` forwards every phase to `metrics::record_phase`,
so its existing call sites compile unchanged:

- The level is `max(PICTURA_PAINT_TIMING, PICTURA_METRICS)`.
- The per-stroke report prints only under `PICTURA_PAINT_TIMING`.
- When `PICTURA_METRICS` is on, the module also keeps:
  - process-wide phase totals in a thread-local, because every edit runs on the
    GUI thread;
  - a ring of the last 32 edit records;
  - the currently open edit record.

`pictura-render::metrics` holds `static AtomicU64` counters: resident hits and
misses, one decline count per `GpuError` kind, and the largest canvas, kept with
`fetch_max`. They always count, because the cost is one relaxed atomic add per
composite or upload, never per pixel. That keeps the existing
`resident_uploads_are_reused_and_never_stale` test working without enabling
anything; it reads the new counter in place of `RESIDENT_HITS`. Tests compare
before and after with `>=`, because the counters are process-wide and tests run
in parallel.

*Alternative:* the `tracing` crate. Rejected: it is a new dependency for the same
data.

### D2: The edit record wraps the transaction

`apply_edit` opens a record (the label, and a swap of the render crate's
largest-canvas value to 0). `apply_edit` sets `composited` whenever it runs
`composite_full` or a region refresh. The record is closed when `edit` returns.
Phases recorded while it is open are copied into it as well as into the totals.

### D3: Qt-side phases go through the bridge

A new invokable, `PictureView::record_metric_phase(label, micros)`, feeds the
same accumulator. `pictura_debug_timing.h` gains `MetricsTimer(view, label)`,
which reads the clock only when `metricsEnabled()` (the env var, read once).
`LayersPanel::refresh` is wrapped as `layers_panel_refresh`.

### D4: The `Edit::Inserted` outcome

`Edit::Inserted` maps to `Effect::Notify`, so no new `Effect` is needed.
`apply_edit` and `preview_edit`:

- clear `link_sets` and `solo_snapshot`, as `Structural` does;
- call `rebuild_mask_solo_source()` and bump `frame_revision` only when
  `mask_solo` is set;
- record one state;
- skip `composite_full`.

The stored `doc.composite` and the pyramid stay valid because the tree change
is content-neutral.

### D5: The neutrality predicate lives in the render crate

`pictura_render::insertion_is_content_neutral(doc, created_path)` is evaluated
after the insertion. It walks the container and applies the four rules in the
spec. `doc.layers.len() > 1` stands in for "had a layer before". It also
requires `is_group`, or the layer's alpha channel to be absent or all zero, so a
dialog neutral fill or any future fill makes it false. Every creation handler
(`add_layer`, `add_group`, `add_layer_in`, `add_group_in`, `new_layer_dialog`,
`new_group_dialog`) returns `Edit::Inserted` when the predicate holds and
`Edit::Structural` otherwise.

*Alternative:* compare against a recomposite. Rejected: that is the cost being
removed.

### D6: Coalesce the panel refresh

`LayersPanel` keeps a `refreshQueued_` flag. The `changed()` handler queues
only when no refresh is queued, `refresh()` clears the flag on entry, and the
queued lambda checks the flag. A synchronous refresh after the edit (as in
`addLayerAt`) therefore consumes the queued one.

### D6b: One refresh from the frame's retarget

Measured on a 24 MP document, one New Layer rebuilt the Layers panel snapshot
three times, at about 0.5 s each:

1. the panel's own queued `changed()` refresh;
2. `PicturaMainWindow::retargetDock`, run by the 120 ms panel debounce, which
   calls `LayersPanel::setView`, and `setView` ends in `refresh()`;
3. `retargetDock` then called `refresh()` again.

The third call is removed. The second stays, because the debounce also picks up
changes that emit no `changed()`, such as region blits. Merging the first two is
left to a follow-up.

### D7: GPU timestamps only with metrics on and device support

`request_device` adds `Features::TIMESTAMP_QUERY` when `metrics::enabled()` and
`adapter.features()` contains it. A composite then writes a beginning timestamp
on its first dispatch and an end timestamp on every dispatch, so the last one
wins. Both go into one two-entry query set. The set is resolved together with
the canvas readback, and the device time is published to a render-side slot
that the app reads into the open edit record. The device time is `Option<f64>`
in milliseconds.

### D8: Control server and profiler

- `metrics` returns the snapshot as JSON, or `refused` when metrics are off.
- `layer_op add_group` calls `view->add_group(index)`.
- `profile-tools.py --metrics` sets `PICTURA_METRICS=1` on the child process,
  calls `metrics` with `reset: true` before each scenario, and calls `metrics`
  again after it.

## Risks / Trade-offs

- [The neutrality predicate misses a case and leaves a stale composite] → A
  Rust property test inserts through every creation path into stacks with
  clipping, groups, pass-through, knockout, and Blend If, and compares the
  stored composite with `composite_rgba` (CPU, `gpu_compute = false`).
- [An undo after an `Inserted` edit restores a stale pyramid] → Undo already
  calls `reset_pyramid` from the restored snapshot, so it does not depend on the
  insert's pyramid.
- [Coalescing hides a refresh that was needed] → `refresh()` always reads
  current state, so skipping a queued refresh after a later synchronous one
  loses nothing.
- [`impl_core.rs` (1178 lines) and `gpu/mod.rs` (1103 lines) reach the
  1200-line cap] → The new code goes in `metrics.rs`, `render/metrics.rs`, and
  `layer_ops/neutral.rs`. Tests go in sibling test files.
