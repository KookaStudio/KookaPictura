# Proposal: line-tool

## Why

The Line tool (issue #47) was catalogued but disabled. photorust's
`core/src/shape.rs` draws a line as the quad a weighted segment sweeps; the
port follows `docs/03-tools/shape-tools.md` and lands on the shape engine,
bridge, handler, and modes of the `rectangle-tool` change.

## What Changes

- `pictura_core::shape::line` (new submodule): the line of Weight (px) as a
  closed outline, Shift snapping its angle to 45°; optional Start / End
  arrowheads with Width (10–1000 %) and Length (10–5000 %) of the weight and
  Concavity (−50–50 %), replacing the line's ends. `ShapeKind::Line`,
  `ShapeOptions::{weight, arrows}`, `Arrowheads`.
- `cxxqt_object/shapes.rs`: `ShapeSpec` gains the weight and arrowheads.
- `options_bar_shape.cpp`: Weight (1 px) and an Arrowheads pop-up.
  `tool_shape.cpp` registers the Line; a click draws nothing (no dialog).
- Qt Test `tst_shape_tools::lineAndCustomShape`. Guard 98 now probes Object
  Rotate (the U group is complete).

## Capabilities

### New Capabilities

- `tools/line-tool`: the Line tool.

## Impact

- `pictura-core` (`shape/line.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `line_points` (<https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/shape.rs>); the arrowheads are CS6's
options (photorust has none) with approximated geometry. Arrowhead defaults
(Width 500 %, Length 1000 %) follow CS6's geometry pop-up;
`docs/03-tools/shape-tools.md` lists 100 % (unsourced). Ceiling
(`ponytail:`): as `rectangle-tool`; overlapping heads on a very short line
are not clamped; the Line is not a live shape.
