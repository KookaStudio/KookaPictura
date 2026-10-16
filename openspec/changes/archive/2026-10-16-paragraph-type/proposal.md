# Proposal

## Why

Issue #682 (split out of #541): the Type tools only make point type. CS6's
other core mode, paragraph type, sets text in a box it wraps to
(`docs/03-tools/type-tools.md`, *Type creation modes*). Without it, a long
caption has to be broken by hand. The Type menu's Convert To Point Text and
Convert To Paragraph Text are stubs.

## What Changes

- **Drag a box:** dragging with a Type tool (or a Type Mask tool) opens a
  paragraph-type session in that box. Lines wrap to the box width, at spaces,
  and a word wider than the box breaks between characters. Vertical type wraps
  its columns to the box height.
- **Paragraph Text Size dialog:** Alt-click with Horizontal or Vertical Type
  asks for the box Width and Height, then opens the box at the click.
- **Box while editing:** the session draws the box with eight handles. Dragging
  a handle resizes the box and reflows the text. When text overflows the box,
  the bottom-right handle shows a `+` and the overflow lines are not rendered.
- **Paragraph layers:** the committed layer keeps its box. Reopening it edits in
  the same box, and the PSD carries it in the EngineData shape
  (`ShapeType 1` + `BoxBounds`), which is read back on open.
- **Convert:** `Type > Convert To Paragraph Text` boxes a point layer at its
  text bounds without moving any glyph. `Type > Convert To Point Text` ends each
  wrapped line with a return and drops the overflow, as CS6 does. Each convert
  is one undo state.
- **BREAKING**: none. Point type is unchanged.

## Capabilities

- Added `tools/paragraph-type`.
