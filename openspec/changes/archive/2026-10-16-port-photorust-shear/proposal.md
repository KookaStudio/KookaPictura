# Proposal: port-photorust-shear

## Why

Issue #226 (part of #183, after #221). Kooka's Shear runs its own warp: the
curve indexes the **column** position and shifts columns **vertically**, and
its dialog is six independent "Top Left / Middle Right / …" sliders. CS6's
Shear runs a line from the top of the image to the bottom and dragging it
sideways pushes those **rows** sideways; its dialog is a square curve box with
draggable control points over an `Undefined Areas` radio pair, with the
preview under it. Photorust already models the CS6 behavior (an offset
curve sampled top to bottom, a horizontal shift per row, a wrap flag) and
ships the curve widget. This change reconciles the two: the engine takes
Kooka's `curve` + `fill`, now read as `(row position, horizontal offset)`,
and the dialog becomes the CS6 curve box.

## What Changes

- `Filter::Shear { curve, fill }` keeps its shape, but `curve` is now
  `(position, offset)` points: `position` `-1` is the top row and `1` the
  bottom, and `offset` in `-1..=1` shifts that row horizontally by
  `offset × width/2` pixels. The warp moves to the photorust engine
  (`photorust::distort::shear`, via the shared `remap`), so it premultiplies
  translucent edges like the rest of the ported Distort family. The old
  vertical column shift in `distort/coord.rs` is removed.
- `SHEAR_MAX_POINTS` (8) is exported from `pictura-filters`; the `shear`
  kind grows from 7 to 18 slots (a control-point count, eight
  `(position, offset)` pairs with the unused ones zero, then the fill) in
  `filter_map.rs` and `filter_commands.cpp`. The dialog edits those same
  points, never a sampled lattice.
- The Shear defaults become a straight curve and `Wrap Around`, as CS6's
  dialog opens.
- `Filter ▸ Distort ▸ Shear…` and Last Filter Settings open the CS6 dialog:
  a 4×4 dotted curve box whose line runs top to bottom, click to add a point,
  drag a point out to remove it, `Undefined Areas:` radios (`Wrap Around`
  first, checked), OK / Cancel top right, and a live preview of the picture
  below. Shear previews against the whole layer, never the visible crop,
  because the shift is a fraction of the layer's half-width.

## Capabilities

### Modified Capabilities

- `imaging/distort-filters`: Shear shifts rows horizontally by a
  `(position, offset)` control-point curve of up to `SHEAR_MAX_POINTS` (8)
  points.

### New Capabilities

- `imaging/filter-app-ui` gains *Shear dialog*: the curve box, the
  `Undefined Areas` radios, the sampled slot order, and the whole-layer
  preview.

## Impact

- `pictura-filters` (`photorust/distort`, `photorust/dispatch`, `distort`,
  `filter/apply`, `lib`); `pictura-app` (`filter_map.rs`, `filter_tools.rs`,
  `filter_commands.{h,cpp}`, `filter_param_controls.{h,cpp}`,
  `filter_preview_dialog.cpp`) plus `tst_shear_dialog`.
- **Output changes:** every Shear result changes. No golden baseline covers
  Shear.
- **Oracle:** Shear stays no-equivalent; the recorded ImageMagick delta is
  re-measured against the horizontal `-shear` and the mapping table,
  `tests/README.md`, and `scripts/filter_oracle.py` are updated.
- **Docs:** `docs/06-filters/distort-filters.md` describes the row shift and
  the dialog, in a separate `TASK-ALLOWS-DOCS` commit.
