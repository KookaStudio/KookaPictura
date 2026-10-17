# Design

## The box in the model

`TypeSpec::box_bounds` and `TypeTool::box_bounds` are `Option<[f64; 4]>`: the
box as left, top, right, bottom relative to the origin, before the matrix.
`None` is point type. A dragged box puts the origin at its top-left with bounds
`[0, 0, w, h]`. Convert To Paragraph Text keeps the point origin and uses the
text's own bounds (`[-w·factor, -ascent, …]`), so no glyph moves. That
generality is also what CS6 files carry. `BoxBounds` need not start at zero.

## Visual lines (`pictura_render::type_paragraph`)

`visual_lines` turns the text into the lines the layout sets:

- **Point type:** the hard lines, as before.
- **Horizontal box:** each hard line wraps greedily. A line breaks after a run
  of spaces when the next word would cross the box width (measured with the
  layout's own advance: shaped advance × horizontal scale + tracking). A first
  word wider than the box breaks at the last character that fits (at least
  one). A soft-wrapped line keeps its trailing spaces for the caret, and the
  render trims them for alignment, so centred and right-aligned lines align on
  their ink.
- **Vertical box:** each hard line breaks into columns of `⌊height / cell⌋`
  characters.

`visible` counts the lines that fit: horizontal lines whose baseline + descent
fits the height (first baseline at top + ascent), vertical columns whose
`leading` slot fits the width. Lines past it are *overflow*. They are not
rendered, and `type_overflows` reports them for the `+`. The caret still steps
through them, laid out past the box, as CS6 lets the caret walk into hidden
text.

`text_box_any`, `layout_type`, `type_caret_stops`, and `type_baselines` all
read `visual_lines`, so the render, the caret, and the hit test agree on every
wrap. Caret stops stay one per UTF-16 boundary. A soft-wrapped line contributes
its characters' stops but not its end boundary, which is the next line's start.

## Placement

A box's layer rect is the box itself (`flat_rect`, or its mapped corners under
a matrix), not the ink, as CS6's paragraph layers are box-sized. Alignment
within a box uses the box width.

## PSD

The authored EngineData gains `/Rendered` with one shape child:
`/ShapeType 1` and `/BoxBounds [l t r b]` under `/Cookie /Photoshop` for a
box, and `/ShapeType 0` with `/PointBase [0 0]` for point type. A merge over
an existing tree replaces `/Rendered` whole, since Photoshop rebuilds its line
cache. The descriptor's `bounds` and `boundingBox` are the box for paragraph
type. Decode reads `ShapeType` / `BoxBounds` back into `TypeTool::box_bounds`.

## Converting

- `type_to_paragraph(spec)`: the box is `type_placement(spec).bounds`. Wrap and
  visible-line tests carry a 0.01 px tolerance, so the measured width always
  fits its own box.
- `type_to_point(spec)`: the visible lines (soft breaks become `\r`, trailing
  spaces trimmed). The origin moves to where point type puts the same first
  line: `l + w·factor`, `t + ascent` (horizontal) or `r − leading/2`,
  `t + h·factor` (vertical), mapped through the matrix.

The bridge `type_convert(path, paragraph)` re-sets the layer through the same
path as an edit and records "Convert To Paragraph Text" / "Convert To Point
Text".

## Tool handler

- A press that misses an existing layer opens a session as before. A drag past
  4 screen pixels turns it into a box drag, and the release fixes the box
  (minimum 1 px a side). A plain click is still point type.
- During a box session, a press within 5 screen pixels of a handle resizes the
  box: points map back through the inverse matrix, the box stays normalized,
  and each move reflows. A press inside the box places the caret, even off the
  text.
- Alt-click on Horizontal/Vertical Type opens `execParagraphTextSizeDialog`
  (the Type Mask tools keep Alt for selection subtract, as their options bar
  defines). It remembers the last size for the session.

## Ceilings (`ponytail:`)

- The wrap is greedy, not Adobe's every-line composer. There is no
  hyphenation, and indents and space before/after stay unrendered (as for
  point type today).
- The box handles resize only. Rotate and skew of the box come from Free
  Transform on the committed layer.
- Vertical columns break by character count; there are no kinsoku rules.
