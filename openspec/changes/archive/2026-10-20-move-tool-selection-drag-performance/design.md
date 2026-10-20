# Design

## Context

The Move tool's selection drag has two phases with different bottlenecks, both
measured with `crates/pictura-render/tests/selection_move_perf.rs` (an ignored
profiling harness that times the real engine functions plus line-for-line copies
of the two app-private loops):

```bash
cargo test -p pictura-render --profile dev --test selection_move_perf -- --ignored --nocapture
cargo test -p pictura-render --release --test selection_move_perf -- --ignored --nocapture
```

The first profile is what a CMake `Debug` build links: Corrosion maps `Debug` to
cargo's `dev` profile, and the workspace `Cargo.toml` has no `[profile.dev]`
overrides, so every engine crate runs at opt-level 0.

| Work | dev | release |
|---|---|---|
| selection bounds scan | 334 ms | 54 ms |
| `Selection::translate` | 2.7 ms | 1.6 ms |
| `crop_selection` 4000² | 359 ms | 41 ms |
| `move_mask_selection_content` | 727 ms | 77 ms |
| selection contour retrace | 4.2 ms | 1.0 ms |
| full CPU composite | 4.49 s | 0.85 s |

Two conclusions drive the design: the opt-level override is necessary but not
sufficient (the bounds scan is still 54 ms per move at opt-level 3, over three
times the 16 ms budget), and the release cost is spread across four full-plane
passes, not just the mask move.

## Decisions

### D1 — Optimise the engine crates in the dev profile

Add `[profile.dev.package.<crate>] opt-level = 2` for `pictura-core`, `-codec`,
`-color`, `-adjust`, `-filters`, `-ops`, `-paint`, `-select`, and `-render`,
mirroring the existing `[profile.test.package.*]` block. Dependencies stay at 0
so the cold build does not recompile all 120+ crates optimised.

- **Not `[profile.dev] opt-level = 2`:** that would optimise the app crate and
  every dependency too, making Debug builds slow to produce and hard to step
  through.
- **Not `pictura-app`:** after this change the app-side hot loops are bulk
  copies (`crop_selection`, `store_composite`, `diff_into`'s memcmp) whose cost
  does not depend on codegen, and keeping the app crate unoptimised keeps the
  UI debuggable. The issue leaves this optional; the measurements say it is not
  needed once the loops are bulk.
- **Cost:** a slower first `Debug` build. Incremental builds are unaffected
  because the engine crates change far less often than the app.

### D2 — Row-based `Selection::bounds()`

`pictura-select` gains `Selection::bounds() -> Option<PsdRect>`, modelled on the
existing `coverage_bounds` in `pictura-render`. It iterates rows, uses
`position`/`rposition` to find each row's first and last covered pixel, and
updates the accumulated rectangle. Empty rows cost one pass over the row, not a
`%` and `/` per pixel; the top and bottom padding of a selection costs nothing
beyond the row scan.

`PictureView::selection_bounds_extents` calls it instead of its own loop.

**Not caching the bounds on `Selection`.** `Selection` has public fields and is
constructed all over the engine; a cached field would need every construction
path to maintain it and every mutation to invalidate it. The row scan is cheap
enough (a few ms at opt-level 0, sub-millisecond at 2) that the cache buys
little outside a drag — and inside a drag the offset replaces it (D3).

### D3 — A drag stores the offset, not a translated plane

`PictureViewRust` gains `selection_move_delta: Option<(i32, i32)>`:

- `begin_selection_move` stores the origin clone and `Some((0, 0))`.
- `preview_selection_move(dx, dy)` stores the delta and returns. It does not
  touch `selection`.
- `commit_selection_move` builds `origin.translate(dx, dy)` exactly once,
  assigns it, records one state, and clears both fields. A zero delta records
  nothing and leaves the origin in place.
- `cancel_selection_move` clears both fields; `selection` was never moved.
- `move_selection_content` clears the delta on success (it already replaces the
  selection with the translated plane).

Consumers audited for the drag window:

| Consumer | Behaviour during a drag |
|---|---|
| `selection_bounds_extents` | origin bounds + delta, clipped (D2) |
| `selection_coverage` | samples the origin at `(x - dx, y - dy)` |
| canvas outline | already driven by `setSelectionContourOffset(dx, dy)` |
| duplicate preview | reads `selection_move_origin` |
| content move commit | reads `selection_move_origin` |
| history | nothing is recorded until commit |
| Info panel size block | bounds + delta |

The plane is still document-sized and still allocated once per drag start (the
origin clone); what disappears is one 16 MB allocation and a full-plane copy per
pointer event.

### D4 — Bound the mask content move to the coverage

`move_mask_selection_content` keeps its signature (a mask-sized coverage plane)
but stops walking the mask:

1. `coverage_bounds(coverage, w, h)` gives the source rectangle; `None` replaces
   the current full-plane "is the selection empty" scan and returns `false`.
2. Clear the source: for each row in the bounds, blend `background` into the
   covered span. The row is sliced to `[left, right)`; no per-pixel bounds check.
3. Composite the destination: for each row in the bounds shifted by `dy` and
   clipped to the mask, blend the source span shifted by `dx` and clipped to the
   mask. Row offsets are computed once per row.

The snapshot and output planes are still full-mask clones (one allocation each),
because the result is the mask plane; only the traversal shrinks from
`w × h` twice to the selection's area twice.

Equivalence is proved against a test-only reference implementation of the old
full-plane loop, on a fixed-seed randomized coverage plane (D-test below).

The layer path already bounds its pixel work (`lifted_copy` uses
`coverage_bounds`); what it lacked was a bulk `clear_layer`. That loop becomes
row-sliced over the layer's rectangle intersected with the selection bounds.

### D5 — `crop_selection` and `clear_layer` copy rows

Both currently index with `y * w + x` and a bounds check per pixel. Both become:
clip the destination x-range once per row, then `copy_from_slice` the contiguous
span. Same result, bulk memory throughput instead of a byte loop, so the cost no
longer depends on the opt level. `crop_selection` still copies the full mask
rectangle; at memcpy speed that is a few milliseconds even at opt-level 0, so
the engine API does not need a sub-rectangle coverage type.

### D6 — Commit through the regional refresh

`PictureView::recomposite_region(rect)` composites only `rect` through
`refresh_region_buffer` (which also patches `doc.composite`), folds the result
into level 0, the damage account, and the pyramid through
`apply_refreshed_region`, and then emits `changed()` so every shell fan-out
(title, registry, Info panel, thumbnail) still runs.

A content move passes the union of the selection's source and destination
bounds, clipped to the canvas — the same rule
`compositing/described-damage` already states for a layer move. It emits
`changed`, not `region_blitted`, because the commit changes history and the
document's dirty flag, which the region blit does not carry.

When the union covers the whole canvas the region path costs what the full
composite did; the contract allows that (a canvas-sized region still uses the
region path).

### D7 — Cursor-only Info panel refresh

`InfoPanel::setCursorPosition` calls `refresh(false)`, which updates only the
position, colour, and selection-size blocks. `refresh(true)` also rebuilds the
document-size line and the sampler list, and runs on document, selection,
ruler, and tool changes. No `stat` runs on a pointer move, and no Rust-side cache is needed, so
there is no staleness after a save.

## Risks

- **A consumer reads `selection` mid-drag and now sees the origin.** Mitigated by
  the audit table in D3 and by a unit test that asserts the plane is unchanged
  after a preview and that coverage hit tests follow the offset.
- **The bounded mask move diverges at the edges.** Mitigated by an equivalence
  test against the old loop with a fixed seed, covering partial coverage values,
  offsets in both directions, duplicate mode, and a selection touching each mask
  edge.
- **The regional commit misses a pixel the full composite would have changed.**
  Mitigated by the existing region-vs-full equivalence pattern: compare
  `doc.composite` after a content move with `current_buffer` on a fresh document
  built from the same layers.
- **The profile override lengthens cold builds.** Accepted; documented in the
  proposal and measurable with `cargo build` timing before/after.

## Alternatives considered

- **Cache the bounds in `PictureView` on every selection mutation.** Rejected:
  invalidation must be threaded through ~20 mutation sites, and the row scan is
  already fast outside a drag.
- **Pass a sub-rectangle coverage to the engine to skip `crop_selection`.** Not
  needed once the copy is bulk; it would add a type and change every caller.
- **Preview at reduced resolution (Krita Instant Preview).** Out of scope: the
  remaining per-move work after this change is the offset bookkeeping, not
  rasterisation.
- **Move the release work to a worker thread.** Larger change (threading the
  document), and the bounded release fits the frame budget on the measured case.
