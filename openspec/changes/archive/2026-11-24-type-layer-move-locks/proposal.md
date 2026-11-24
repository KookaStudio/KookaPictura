# Proposal: type-layer-move-locks

## Why

Dragging a type layer can look stuck: the dashed Layer Edges box follows the
pointer while the type content stays, and the release snaps back. Two paths cause
it. With a selection active the Move tool always starts a selection-content move,
but a type layer is pixel-locked, so the engine refuses on release after the
selection outline has already followed the drag — a half-applied move. Without a
selection, a type layer whose box is document-sized previews and damages the whole
box instead of the few words it inks, which stalls or refuses a large-document
move.

## What Changes

- The Move tool asks whether the active target can take a selection-content move
  before starting one. A type or shape layer, a hidden target, or a layer whose
  position, pixel, or cut-transparency lock refuses it moves the whole layer
  instead. A position lock therefore starts no preview at all.
- A type layer's Move preview, backdrop patch, and damage use its ink bounds, not
  its paragraph box, so a document-sized box moves only the words it draws.
- The Move preview cache keeps a type layer's ink offset across a committed move
  instead of resetting it to zero.

## Capabilities

### Modified Capabilities

- `tools/selection-content-move`: a target that cannot move selected pixels moves
  as a whole layer instead, and a position lock refuses the press cleanly.
- `tools/move-drag-feedback`: the marching ants box a type layer's ink, not a
  document-sized paragraph box.

## Impact

- `pictura-app`: the selection-move bridge, the Move tool press, and the Move
  preview/damage bounds. Qt Test and Rust tests. No engine or document-format
  change.

## Provenance

Behavior follows the existing `selection-content-move`, `layer-locks`, and
`canvas-tools` contracts; no external implementation is used.
