# Proposal: refine-edge

## Why

Issue #297 (extracted from the #252 UX pass): the Select tools options bar has no
`Select and Mask…` / `Refine Edge` entry, and `Select ▸ Refine Edge…` is a
disabled leaf backed only by the design document `docs/08-selection/refine-edge.md`
("No code exists in this repository"). CS6 Refine Edge is the documented
replacement for the old Extract plug-in and the way soft edges (hair, fur) are
recovered from a selection.

## What Changes

- Add a `refine` module to `pictura-select`: edge-band estimation (plain and
  smart radius), the global refinements (Smooth, Feather, Contrast, Shift Edge),
  and colour decontamination.
- Add the `RefineEdgeSettings` / `OutputTarget` / `ViewMode` models and a
  `refine` entry point that turns a selection mask plus the composite into a
  refined mask and (optionally) a decontaminated pixel buffer.
- Add the Rust bridge (`cxxqt_object/refine.rs`): a preview mask, an apply
  (`Selection`, `Layer Mask`, `New Layer`, `New Layer with Layer Mask`), and the
  decontaminated output.
- Build `RefineEdgeDialog` (CS6 layout: view mode, edge detection, adjust edge,
  output) and wire `Select ▸ Refine Edge…` (`Ctrl+Alt+R`) plus a `Select and
  Mask…` button in the selection tools options bar.
- Cover the engine with property tests and the app with `tst_refine_edge`.

## Capabilities

### New Capabilities

- `tools/refine-edge`: the Refine Edge engine (edge estimation, global
  refinements, decontamination) and its dialog plus command surface.

### Modified Capabilities

- `tools/select-menu`: `Refine Edge` moves from the disabled-row list to an
  implemented command.

## Impact

- `pictura-select` (`refine`), `pictura-app` (`cxxqt_object/refine.rs`,
  `refine_edge_dialog.{h,cpp}`, `frame_menus_select.cpp`, `options_bar*.cpp`,
  `command_ids.h`/`.cpp`). No new dependency.
- **Oracle:** none. Adobe's edge-estimation and matting algorithms are closed;
  the module is a documented behavioral approximation, covered by property
  tests.
- Out of scope: the interactive Refine Radius / Erase Refinements brushes
  (their strokes and local band override), the live-preview view modes beyond
  the mask toggle, refining a layer mask from the Properties panel, and the
  later-CC "Select and Mask" workspace.
