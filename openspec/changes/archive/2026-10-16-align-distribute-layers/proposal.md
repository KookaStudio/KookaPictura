# Proposal: align-distribute-layers

## Why

Issue #64: import align and distribute layers from photorust
(`shell/src/MainWindow.cpp` Move tool Align / Distribute, engine
`core/src/document.rs`). Kooka listed Layer > Align, Align Layers To
Selection, and Distribute as inert menu placeholders, and the Move tool had no
options bar.

## What Changes

- `pictura-render` `layer_ops/align.rs` (new): `AlignEdge` (Top, Vertical
  Center, Bottom, Left, Horizontal Center, Right), `align_layers` (against a
  selection box when given, else the box round every selected layer's
  content), `distribute_layers` (three or more; the outermost stay put), and
  `can_align` / `can_distribute`. A layer lines up by the box round its
  non-transparent pixels; a position-locked layer or the Background holds
  still but counts toward the box. `crop.rs` gains a shared `offset_layer`.
- `cxxqt_object/align.rs` (new bridge): `align_can`, `distribute_can`,
  `align_apply`, `distribute_apply`, each one history state named as CS6
  names it ("Align Top Edges", "Distribute Left Edges", …).
- Layer > Align ▸ (two layers), Align Layers To Selection ▸ (a selection and
  one layer), and Distribute ▸ (three layers) act on the Layers panel's
  selected layers.
- The Move tool gets an options bar (`options_bar_move.cpp`): six Align and six
  Distribute buttons, aligning to the selection when there is one, enabled as
  the menu commands are.
- Twelve Lucide 1.50.0 icons (`layer.align*`, `layer.distribute*`) vendored
  with their map entries.
- Tests: `align` unit tests (ported from photorust's) and the Qt Test
  `tst_align_distribute`.

## Capabilities

### New Capabilities

- `document/align-distribute`: Align / Distribute layers and the Move tool's
  buttons.

## Impact

- `pictura-render`, `pictura-app` (bridge, C++), `assets/icons`. No new
  dependency.

## Provenance

Edge maths and rules from photorust's `Document::align_layers` /
`distribute_layers`; command semantics from `docs/03-tools/move-and-transform.md`.
Ceiling (`ponytail:`): groups, adjustment layers, and fill / shape layers are
skipped rather than aligned by their descendants' content or shape bounds; the
Move bar has no Auto-Select, Show Transform Controls, or Auto-Align Layers.
