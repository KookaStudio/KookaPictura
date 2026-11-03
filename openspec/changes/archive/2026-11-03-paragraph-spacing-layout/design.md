# Design

## Context

`visual_lines` in `type_paragraph.rs` currently returns each visible line's text
and whether it ends a paragraph. `layout_type` shapes those lines, spreads
justified ones to the box width, and calls `layout_lines` with one alignment
column for the whole block. The caret re-derives the same geometry in
`type_caret.rs`, and `text_box_any` derives point-type bounds from the natural
line advances. None of the three reads `start_indent`, `end_indent`,
`first_line_indent`, `space_before`, or `space_after`.

The layout is shared by horizontal and vertical type and by point and paragraph
type. Area type reuses it; on-path type does not.

## Goals / Non-Goals

**Goals:**

- One per-line geometry model that the renderer, caret, justification, and
  point-type bounds all consume.
- Horizontal: left/right indents narrow each line's column; the first line of a
  paragraph adds the first-line indent relative to the left indent; a negative
  first-line indent extends into the left margin.
- Vertical: left/right indents map to top/bottom; the first column of a
  paragraph adds the first-line indent relative to the top indent.
- Space before/after offset the block axis: baselines for horizontal type,
  inter-column gaps for vertical type.
- Point type follows the same rules and its placement rect covers the result.
- Preserve every zero-indent, zero-spacing result exactly, so existing renders,
  goldens, and tests do not move.

**Non-Goals:**

- Hanging punctuation, hyphenation, and the every-line composer stay
  authored-only; this change does not implement them.
- On-path type has no paragraph flow; its arc-length layout is unchanged.
- No new paragraph panel controls, codec keys, or document-model fields.
- No change to the greedy wrap, bidi shaping, or glyph rasterization.

## Decisions

### Per-line boxes instead of one block column

Chosen: `VisualLine` gains its paragraph index and a `first` flag, and a
parallel `LineBox` carries `start`, `extent`, and `block`:

- horizontal: `start` is the line's left edge, `extent` its width, `block` its
  baseline offset from the first baseline;
- vertical: `start` is the column's top edge, `extent` its height, `block` the
  distance of its centre from the box's right edge.

`visual_lines` computes the boxes while it wraps/chunks, because the indent
changes the available width/height and therefore the line breaks. `layout_type`
then places each line in its own box instead of asking `layout_lines` for one
shared alignment column.

Alternative considered: keep one column and post-shift each line by its indent.
Rejected: the indent changes the wrap width, so post-shifting would produce the
wrong line breaks and could overflow the box.

### Paragraph spacing is block-axis spacing

For horizontal type, `space_before` is added above a paragraph's first baseline
and `space_after` below its last line; the next paragraph's first baseline moves
by both. For vertical type the block axis is horizontal, so the same values
become gaps before the first column and after the last column of a paragraph.
The first paragraph's `space_before` and the last paragraph's `space_after`
count toward point-type bounds; a box's visible prefix counts only lines whose
baseline/column fits, so spacing can push a line into overflow.

Approximation: CS6 does not document whether the first/last paragraph's
spacing is suppressed at a point-type origin. This change applies it to every
paragraph and marks the choice in the engine.

### Indents are visual left/right, not direction-relative

The Paragraph panel labels the fields Left Indent and Right Indent, and the docs
say left/right map to top/bottom for vertical type. `start_indent` is therefore
the visual left (top) margin and `end_indent` the visual right (bottom) margin,
regardless of paragraph direction. `first_line_indent` is relative to that left
(top) margin.

### The caret and bounds consume the same boxes

`caret_lines` already re-derives the line geometry; it now reads the boxes and
the visible prefix from `visual_lines` instead of recomputing alignment from the
box width. `text_box_any` calls `visual_lines` for point type and uses the
returned inline bounds and block size, so the layer rect covers a hanging first
line, an indented line, and trailing paragraph space. Area type and warp inherit
the new layout through `layout_type`; PDF pens and decorations read the glyph
positions, so they follow automatically.

### Point-type origin keeps its meaning

The origin remains the first baseline (horizontal) or the first column centre
(vertical). Space before extends the layer box above/right of the origin instead
of moving the origin; a negative first-line indent extends it left/top. This
keeps reopening and moving a layer stable and preserves the existing zero-indent
bounds.

## Risks / Trade-offs

- [Existing zero-indent renders move] -> The placement math keeps the same
  column when all five fields are zero; the engine tests pin a zero-indent
  render before the change.
- [Vertical paragraph spacing is an interpretation] -> CS6 documents indents as
  top/bottom for vertical type but not space before/after. The block-axis
  reading is the standard writing-mode meaning and is marked as an
  approximation.
- [A negative indent can place ink outside a paragraph box] -> The box clips the
  render as it does today; the point-type rect grows instead, and the test plan
  covers the hanging case.
- [Line boxes add memory per line] -> Two `f32`s plus a flag per line; negligible
  next to shaped glyphs.

## Migration Plan

None. Session state, document format, saved files, and the Paragraph panel are
unchanged.
