# Proposal: polygon-tool

## Why

The Polygon tool (issue #46) was catalogued but disabled. photorust's
`core/src/shape.rs` draws its outline; the port follows
`docs/03-tools/shape-tools.md` and lands on the shape engine, bridge, handler,
and modes of the `rectangle-tool` change.

## What Changes

- `pictura_core::shape`: a regular polygon of Sides (3-100) centred on the
  press, the drag its radius, the first corner under the pointer; Shift snaps
  that rotation to 15°.
- `options_bar_shape.cpp`: Sides (default 5).
- `tst_shape_tools` draws a hexagon in Pixels mode.

## Capabilities

### New Capabilities

- `tools/polygon-tool`: the Polygon tool.

## Impact

- `pictura-core` (`shape.rs`), `pictura-app` (C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/shape.rs`
(<https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/shape.rs>). Behavioural parity only: no CS6 oracle exists for
the drawn geometry. Ceiling (`ponytail:`): as `rectangle-tool`, plus no star or smoothing on a drag (the Create Polygon dialog of the `live-shapes` change has them) and no fixed Radius. The polygon turns to follow the pointer, as photorust's does (CS6 behaviour unsourced); `docs/03-tools/shape-tools.md`'s "upright" start angle holds for a drag straight up.
