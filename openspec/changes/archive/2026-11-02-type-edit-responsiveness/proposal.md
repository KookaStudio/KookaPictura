# Proposal

## Why

Changing a text layer on very large images is slow (#1135). On a 16507×16196 document,
a change of font, size, alignment, colour, or a paragraph option takes 2-3 s to show.
The text raster is only text-sized, but each edit declares no damage, so it composites
the whole document, rebuilds the view pyramid, and diffs the composite for history.

## What Changes

- **Commits.** Re-setting a type layer and committing a type session damage only the
  union of the layer's reach before and after the edit.
- **Previews.** A live type preview uses the same bounds and records no history.
- **Dialogs.** Warp Text and style dialog previews are bounded the same way.
- **Undo.** Crossing a history segment refreshes what differs, not the whole document.
- **Knockouts and huge boxes.** A text edit stays bounded in both.
- **Profiler.** `type_op` lets `scripts/profile-tools.py` time text edits.
- **Hide and show.** Opening a type layer for editing and cancelling that session
  refresh only the layer's reach.

## Capabilities

### New Capabilities

- `tools/type-edit-responsiveness`: text-sized refresh for type layer edits, Warp Text and
  style previews, and undo across a history segment.

## Impact

- `crates/pictura-app`: `type_tools/`, `history/`, the control server's `type_op`.
- `crates/pictura-render`: `damage.rs`.
- `scripts/profile-tools.py`: type scenarios.
- Damage of 75% of the canvas or more stays a whole refresh, as does any display a
  pending preview, a soloed mask, or a live session owns. Hiding a layer for a session in
  a knockout document stays whole, and so does a style edit over a huge box.
