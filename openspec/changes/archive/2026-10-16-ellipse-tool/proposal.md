# Proposal: ellipse-tool

## Why

The Ellipse tool (issue #45) was catalogued but disabled. photorust's
`core/src/shape.rs` draws its outline; the port follows
`docs/03-tools/shape-tools.md` and lands on the shape engine, bridge, handler,
and modes of the `rectangle-tool` change.

## What Changes

- `pictura_core::shape`: the ellipse inscribed in the dragged box as four
  smooth anchors (quarter-ellipse cubics); Shift draws a circle, Alt grows it
  from the press point.
- `tst_shape_tools` draws one in Path mode.

## Capabilities

### New Capabilities

- `tools/ellipse-tool`: the Ellipse tool.

## Impact

- `pictura-core` (`shape.rs`), `pictura-app` (C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/shape.rs`
(<https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/shape.rs>). Behavioural parity only: no CS6 oracle exists for
the drawn geometry. Ceiling (`ponytail:`): as `rectangle-tool`.
