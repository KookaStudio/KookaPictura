# Proposal

## Why

Owner review on #541: the paragraph box showed handles, but they looked inert.
They had no resize cursor or size readout, and CS6's rotate band and reference
point were missing. Point type had no box at all. CS6 lets a type session
transform its text in place: the paragraph box resizes (with a W / H readout),
rotates just outside its corners (with an angle readout), and has a draggable
reference point (with an X / Y readout). Holding Ctrl shows the same box on
point type, where a handle scales the text.

## What Changes

- While a Type session is open, paragraph type's box, and point type's text
  bounds while Ctrl is held, show eight handles, a rotate band just outside the
  box, and a reference point at the box centre.
- A handle resizes the paragraph box (the text reflows; overflow stays hidden)
  or, with Ctrl or on point type's box, scales the text about the opposite edge
  or corner. A readout by the pointer shows W / H in the rulers' unit.
- A drag in the rotate band rotates the text and its box about the reference
  point, with an angle readout.
- The reference point drags anywhere, with an X / Y readout. It moves with the
  text when the text is dragged.
- Each zone has its cursor: diagonal or axis resize, the rotate arrow, the
  four-way move on the reference point, the Move tool's outside, the I-beam on
  the text.
- The transform is the type layer's own (`TySh`), so the text stays editable.
  Nothing is recorded until the session commits.
- **BREAKING**: none.

## Capabilities

- Added `tools/type-transform-box`.
