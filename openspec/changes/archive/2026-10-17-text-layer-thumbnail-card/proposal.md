# Proposal

## Why

The Layers panel draws a text layer's thumbnail as a white card letterboxed to
the document's aspect ratio and rings it with the same 1 px black outline every
other non-group thumbnail gets. On a wide or tall canvas the "T" card is
stretched and boxed, where CS6 shows a square type card with no outline.

## What Changes

- A type layer's thumbnail is a **square (1:1) card** for any document aspect
  ratio, not letterboxed to the document.
- The type card draws **no 1 px black outline**; a regular pixel layer keeps
  its outline and a group keeps its outline-free folder glyph.
- The thumbnail's hit rectangle follows the drawn square card, so clicking the
  type card still selects the layer and Ctrl-click still loads its pixels.
- The white active-target corner brackets are unchanged (they already sit one
  pixel outside the card).
- **BREAKING**: none.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/layers-panel`: the "Layer row badges and delegate" requirement — a type
  layer's thumbnail is square and carries no black outline.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h` — the type branch and
  the outline block in `LayerRowDelegate::paint`, and the image-thumbnail card
  rect shared by `paint` and `thumbRect`.
- `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` — a new Qt Test case
  next to `groupIconHasNoOutline`.
- No Rust change; no new dependencies.
