# Proposal

## Why

The Rust↔Qt seam is one god module with no facade: a single `#[qobject] PictureView`
carries 254 `#[qinvokable]` methods and 46 `extern "Rust"` free-function bridge
modules (~430 free fns) over one ~60-field `PictureViewRust`. 142 production C++
files hold a raw `PictureView*` and call invokables directly; geometry and bounds
cross as space-separated strings re-parsed at dozens of call sites (only some use
`SkipEmptyParts`); the Layers panel issues ~20 crossings per row plus an N-effect
`layer_style_value` loop; paint ordering (`begin_paint` → `flush_present` →
`end_paint`, mid-stroke `image()` ban) is folklore. `docs/dev/refactor-tool-architecture.md`
Track 4 still prescribes a single bridge and no longer describes the code. Thirty-plus
open feature issues will add more crossings to an already over-cap bridge
(`cxxqt_object.rs` is allowlisted at 1220 LOC against a 1200 cap).

## What Changes

- **Typed C++ adapter namespaces** under `crates/pictura-app/cpp/api/`, each registered
  explicitly in `CMakeLists.txt`: `document_api`, `layers_api`, `selection_api`,
  `paint_api`, `transform_api`, `history_api`, `type_api`. Dialogs, panels, commands,
  tools, and the control server SHALL link only against these adapters for operations
  covered by a landed migration unit; generated cxx-qt headers become an implementation
  detail included solely by adapter translation units (except a named, temporary
  deferral of `frame_includes.h` mega-include lines until their owning unit lands).
- **Hybrid typed Rust calls** for crossings that earn their keep:
  - `layers_snapshot(thumb_size, entire_doc) -> Vec<LayerRow>` collapsing the per-row
    N+1 (and the per-row style-effect loop) into one call.
  - Typed `LayerRect`, `Quad`, `CanvasSize`, `SelectionContour`, `SelectionBounds`,
    and the 9-coefficient preview matrix replacing string re-parsing at the adapter
    boundary; `display_level_size` and `composite_bounds` are covered by the same
    typed-geometry contract and assigned to named units (document and
    selection/crop respectively).
- **Delete superseded surface** once the last C++ caller migrates: the **29**
  `layer_row_*` entry points (20 `PictureView` invokables + 9 free functions) and the
  string-geometry invokables (`display_level_size`, `layer_rect`, `transform_quad`,
  `transform_preview_matrix`, `selection_contour`, `selection_bounds`,
  `composite_bounds`), shrinking the `cxxqt_object.rs` allowlist entry.
- **Paint stroke ordering** owned by `paint_api`: a tool-lifetime stroke object
  (constructed on press, finished/cancelled on release/Esc) plus a mid-stroke present
  path that queues `flush_present` and never calls `image()`; the existing
  `begin_brush` helper layer and `frame.cpp`'s deferred `flush_present` timer are
  migrated into the same unit.
- **Policy**: supersedes Track 4's single-bridge policy; companion docs work is issue
  #281 (`docs/dev/refactor-view-seam.md`). One branch, one PR; migration units land
  independently within it in order: document+history → selection → paint+transform →
  layers+snapshot → type.
- **No algorithm moves.** Adapters only absorb protocol/ordering/batching complexity;
  a 1:1 forwarder of a single invokable is not an adapter, and no document logic
  lives in an adapter. A unit façade that centralizes a unit's call sequences and
  typed conversions counts as absorbing complexity.
- **BREAKING**: none for end users. C++ call sites inside `pictura-app/cpp` change
  includes and call shape; public self-test tokens and exit codes are unchanged.

## Capabilities

### New Capabilities

- `ui/view-seam-adapters`: How the C++ shell consumes the cxx-qt `PictureView` seam —
  the exclusive adapter surface (for covered units), typed geometry and snapshot
  contracts, and paint-stroke ordering enforcement.

### Modified Capabilities

<!-- none: existing panel/bridge requirements keep their observable behavior;
     this change only alters how C++ reaches them -->

## Impact

- **Code**: `crates/pictura-app/cpp/api/` (new), ~142 consumer TUs under
  `crates/pictura-app/cpp/`, `CMakeLists.txt` (explicit adapter entries),
  `crates/pictura-app/src/cxxqt_object*.rs` (typed calls, deletions),
  `scripts/file-size-allowlist.txt` (shrink `cxxqt_object.rs` entry),
  `frame_includes.h` (cxxqt includes owned by units A/D/export).
- **Tests**: new `tst_layers_snapshot` (snapshot ≡ old per-getter output); existing
  string-protocol assertions (36 hits across `cpp/tests/` + `selftest*` for the five
  named protocols, plus `selection_bounds`/`composite_bounds` assertions) migrate to
  typed comparators as their unit lands; self-test exit codes unchanged; no new
  `ST_BEGIN` sites (selftest budget stays ≤ 511).
- **Docs**: #281 records the decision; Track 4 superseded (separate docs commit with
  `TASK-ALLOWS-DOCS`).
- **Dependencies**: none new. cxx-qt 0.10 / Qt 6 as today.
- **Sequencing**: #281 before code; spike on `Vec<LayerRow>` return shape gates unit D.