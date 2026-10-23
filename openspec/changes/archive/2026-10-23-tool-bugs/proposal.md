# Proposal: Paint, selection and type tool bugs

## Why

The audit found a Pencil option leaking into the Brush, healing writing the wrong target, phantom history and stale RGB in the paint engine, missing Mixer modes, edge-clamped selection morphology, a per-move Quick Selection commit, wrong Refine Edge units, and a dropped Baseline Shift on path type.

Two findings shape the plan. `#867` is a contract change, not a code catch-up: the published selection model currently says the canvas edge is exempt from erosion, and this change reverses that. `#864` has two acceptable outcomes in its issue; this plan chooses refusal because a layer mask is a single-channel grayscale document and the healing engine requires RGB planes, so mask healing would be a new engine rather than a wiring fix.

## What changes

- #863 — Auto Erase is read only by the Pencil stroke path; the Brush ignores the shared flag.
- #864 — healing refuses while a layer mask is the edit target, with no history state and no pixel or mask change.
- #865 — stroke dirtiness is derived from stored-byte changes, not generated coverage; Clear writes RGB 0 at full coverage.
- #866 — Mixer Brush accepts Mode and Opacity, composites through the shared paint-mode path, and exposes both controls.
- #867 — Contract/Border treat out-of-canvas samples as zero; the edge-exempt rule is replaced.
- #868 — Quick Selection accumulates one selection per stroke, records one history state on release, restores the pre-stroke selection on cancel, and switches New to Add after the first selection.
- #869 — Refine Edge distances are document pixels and the preview scales Radius/Smooth/Feather by the proxy factor.
- #870 — type on a path applies the run's Baseline Shift along the path normal, including the caret and bounds.

## Capabilities

- Modified: `tools/brush-tools`
- Modified: `compositing/layer-masks`
- Modified: `tools/paint-engine`
- Modified: `tools/mixer-brush-tool`
- Modified: `tools/selection-model`
- Modified: `tools/refine-edge`
- Modified: `tools/type-on-path`
- Added: `tools/quick-selection`

## Impact

`crates/pictura-paint/src/{stroke,stroke/blend,stroke/raster,mixer,stamp}.rs`, `crates/pictura-app/src/cxxqt_object/{paint_tools,healing,impl_selection}.rs`, `crates/pictura-app/src/cxxqt_object/impl_selection/refine.rs`, `crates/pictura-app/cpp/{tool_brush,tool_wand,tool_healing,tool_mixerbrush,options_bar_paint}.cpp`, `crates/pictura-select/src/{lib,refine}.rs`, `crates/pictura-render/src/{type_path,gpu/shader}.rs`.

Tests: `crates/pictura-paint/src/{stroke/tests,mixer,stamp}.rs`, `crates/pictura-select/src/{lib,refine}.rs`, `crates/pictura-select/tests/oracle.rs`, `crates/pictura-render/src/type_path.rs`, `crates/pictura-app/cpp/tests/{tst_options_bars_parity,tst_mask_edit,tst_tool_commit,tst_refine_edge}.cpp`, and the preview-scale helper in `crates/pictura-app/src/cxxqt_object/impl_selection/refine.rs`. No new Qt test file, so no CMake test registration.

## Issues

Closes #863, #864, #865, #866, #867, #868, #869, #870.
