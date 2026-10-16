# Proposal: rounded-rectangle-tool

## Why

The Rounded Rectangle tool (issue #44) was catalogued but disabled. photorust's
`core/src/shape.rs` draws its outline; the port follows
`docs/03-tools/shape-tools.md` and lands on the shape engine, bridge, handler,
and modes of the `rectangle-tool` change.

## What Changes

- `pictura_core::shape`: a rectangle whose corners are quarter-circle cubics
  of Radius, clamped to half the shorter side; where the clamp leaves no
  straight edge the arc anchors merge (a stadium has six, a circle four).
- `options_bar_shape.cpp`: Radius (0-1000 px, default 10).
- `tst_shape_tools` draws one in Path mode.

## Capabilities

### New Capabilities

- `tools/rounded-rectangle-tool`: the Rounded Rectangle tool.

## Impact

- `pictura-core` (`shape.rs`), `pictura-app` (C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/shape.rs`
(<https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/shape.rs>). Behavioural parity only: no CS6 oracle exists for
the drawn geometry. Ceiling (`ponytail:`): as `rectangle-tool`. The Radius default is 10 px, as CS6 ships it; `docs/03-tools/shape-tools.md` lists 0 (unsourced).
