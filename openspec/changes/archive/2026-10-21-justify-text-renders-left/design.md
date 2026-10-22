# Design

## Context

See proposal.md - Why.

The state that shapes the approach:

- `pictura-core::layout_lines` is the layout seam: it positions already-shaped
  glyph runs and shifts a line inside a `wrap_width` by
  `(wrap_width - line_advance) * factor`. It has no notion of a space, a
  paragraph, or a justification.
- `pictura-render::text_render::layout_type` is where a type layer's text
  becomes a layout: it shapes each line with `shape_styled` and calls
  `layout_lines`. It is the only place that knows both the text and the shaped
  glyphs.
- `pictura-render::type_paragraph::Lines` already carries the paragraph
  structure: `VisualLine::soft == false` marks a hard line's end, so the lines
  of one paragraph are the runs between two `soft == false` lines.
  `Lines::rendered()` flattens that structure away.
- `pictura-render::type_caret` measures the text itself, per character
  prefix, rather than reading the layout, so it re-derives every position the
  renderer paints.
- `pictura-render::type_layer::align_factor` is the render crate's copy of
  the justification-to-alignment table. It is the *vertical* placement factor
  in three of its four call sites (`vertical_layout`, the vertical branch of
  `text_box_any`, the vertical branch of the caret, and the vertical branch of
  `type_to_point`).
- The justification byte reaches the renderer as a typed `Justify` in the type
  layer's style and as a `u8` through `TypeSpec::justification()`.

## Goals / Non-Goals

**Goals:**

- A justified paragraph's lines fill the column, and the paragraph's last
  line keeps the alignment its justification variant names.
- One mapping from the justification byte to an alignment, shared by the core
  layout and the render crate, so the two cannot drift.
- The caret stands where the painted glyphs are.
- No new dependency, no change to the public type-layer or PSD formats, and
  no change to the rendering of a document that uses left, right or centre
  alignment.

**Non-Goals:**

- Word-spacing min/desired/max bounds, glyph scaling and letter spacing
  (the Justification dialog, #518). The spread divides the leftover width
  evenly.
- Right-to-left text (#518). The shaper is left-to-right only, and so is the
  spread.
- Vertical justification, which in CS6 spreads the columns to fill the
  height. The vertical variants keep their top, bottom and centre placement.
- The Qt `Rasterize Type` path (`crates/pictura-app/cpp/render_text.cpp`). It
  is a separate renderer that already ignores the box, the tracking and the
  leading; Qt can express "Justify Last Left" but not "Justify All".
- A paragraph composer. The greedy wrap stays as it is.

## Decisions

### The spread lives in the render crate, not in the core layout

`pictura-core::layout_lines` sees glyph runs. To justify there, the core
would need to know which glyphs are word spaces and which line ends a
paragraph: a layout flag on `ShapedGlyph` (a shaping record) and a
paragraph-aware layout API.

`pictura-render` owns the text: it shapes the lines and knows the paragraph
structure. The spread is therefore applied there, and the core's contract
stays "position shaped glyph runs".

Rejected: a `ShapedGlyph::space` flag plus a `layout_paragraphs` core API.
It grows a shaping record with layout metadata, rewrites the seam's public
API for one behaviour, and still leaves the render crate to compute the
paragraph structure.

### One alignment serves the whole layout, except for a line that cannot spread

`layout_lines` shifts a line by `(wrap_width - line_advance) * factor`. A
spread line's advance equals the wrap width, so its shift is zero whatever
the factor is. The only line whose factor matters is the paragraph's last
line, and every paragraph in a layer shares one justification. So the layout
keeps a single alignment, and that alignment is the *last line's* alignment:

| byte | justification      | last line's alignment | vertical placement |
|------|--------------------|-----------------------|--------------------|
| 0    | Left               | Left                  | top                |
| 1    | Right              | Right                 | bottom             |
| 2    | Center             | Center                | centre             |
| 3    | Justify Last Left  | Left                  | top                |
| 4    | Justify Last Right | Right                 | bottom             |
| 5    | Justify Last Center| Center                | centre             |
| 6    | Justify All        | Left                  | top                |
| 7+   | unknown            | Left                  | top                |

The vertical column is the same table read as CS6's vertical justification
(Justify Last Top, Bottom, Centered, All), so one mapping serves both
orientations.

One case escapes the single alignment: a line in a justified paragraph that
cannot be spread, because it has no word space or already reaches the column.
Such a line is not the paragraph's last line, so it should stay flush left,
but the layout would shift it by the last line's factor. The render crate
undoes that shift for exactly those lines. The alternative - a per-line
alignment in the core's layout - would put the paragraph's alignment in the
core's API for one case.

This is `pictura-core::TextAlign::from_justification`. The render crate's
`align_factor` matches the same table today; it is deleted in favour of
`TextAlign::from_justification(byte).factor()`, with `TextAlign::factor` made
public.

### The spread widens the word spaces' advances

The line's advance after the spread is `advance + spaces * extra`, so
`extra = (column - advance) / spaces` and the line's advance lands on the
column width. The spaces are the glyphs the face's space glyph maps to, found
by glyph id: `ttf_parser`'s `glyph_index(' ')`.

The widening happens *before* the layout, so `layout_lines` measures the
spread line at the column width, shifts it by zero, and reports the column
width as the line's advance - which is what the underline and strikethrough
span (`type_decor`).

Rejected:

- Re-shaping the line word by word. It loses the shaper's context across the
  line and changes every existing render.
- Using rustybuzz's cluster index to find the spaces. The cluster is a byte
  offset into the piece the caps transform produced, so mapping it back to
  the line's text is fragile.
- Spreading after the layout. The layout has already shifted the line by its
  alignment, which the spread would have to undo, and the core's alignment
  path would stop being the one that positions the text.

The line's advance is measured the way `type_paragraph::line_advance`
measures a line: the shaped advances summed, plus the tracking per glyph.
`layout_lines` measures it the same way (`device_advance` per glyph, and
`units_per_em == font_size` makes `device` the identity), so the two agree.
The arithmetic is `f32`, so the spread line's advance lands on the column
width to within a fraction of a pixel, not exactly; a test pins it with a
tolerance, which fails if the two measurements ever drift apart.

The spread equalises the *advance*, not the ink: `device_advance` adds the
tracking after every glyph, including the last, so with a non-zero tracking
the ink ends one tracking step short of the column.

### The paragraph structure comes from `type_paragraph`

`VisualLine::soft == false` marks a paragraph's last line. `Lines` gains a
method that yields the rendered lines with that flag, so `layout_type` knows
which line of each paragraph is the last.

The structure is read from the whole text, not from the visible lines: a
paragraph whose overflow cuts it short has no visible last line, so its last
*visible* line is spread, because the paragraph continues past the box.

A line with no word space, or one whose advance already reaches the column,
is not spread: there is nowhere to put the leftover width.

### The column is the paragraph box's width

The layout's wrap width is already the box's width for a paragraph-type layer
and the layer rect's width for a point-type layer. The spread is gated on the
layer having a box, so a point-type layer has no column and never spreads,
while the justification variant still selects its lines' alignment (a Justify
Last Right point-type layer is right-aligned). This matches CS6, where
justification fills "the entire width of the text box" and a point-type layer
has no box.

A point-type layer's rect is anchored by that alignment (`text_box_any` puts
the text's edge at the origin), so a point-type layer with Justify Last Right
is anchored by its right edge, as a right-aligned point type always is. A
layer that already exists keeps its rect: `type_layer_spec` re-derives the
origin from the stored rect, so the pixels stay put and only the text's
alignment inside the rect changes.

### The caret measures the same spread

`type_caret` re-derives each pen position from the text, so it must add the
same per-space extra: a stop at character `k` sits at
`shift + advance(k) + spaces_before(k) * extra`. The extra comes from one
shared helper, so the caret and the renderer cannot disagree. A spread line's
shift is zero (it fills the column); the paragraph's last line keeps
`(column - advance) * factor`; a line that cannot be spread keeps the same
shift as the renderer gives it.

The caret measures the *untrimmed* line, while the renderer lays out the
trimmed one (a soft-wrapped line drops its trailing spaces). The stops
inside that tail therefore lie outside the painted line; the spread counts
the trimmed line's spaces, so the tail's stops keep the unspread advance.

### The space count is the text's, and the shaped run's must match

The renderer counts the space *glyphs* in the shaped run; the caret counts the
space *characters* in the text. The two agree when the face has a space glyph
and the shaper emits it for U+0020 - true for the bundled face, and for a
host face with a U+0020. When they disagree - a face without U+0020, or a
face whose space glyph another character shares - the helper reports no extra
and neither side spreads, so the caret cannot drift from the ink.

The shared helper lives in a new `pictura-render` module, `type_justify`,
next to `type_paragraph` and `type_caret`, and holds the whole justification
concern: whether a justification spreads, whether it spreads the last line,
the per-space extra, and the widening of a shaped line's space glyphs. It
depends on `type_paragraph` for the line's advance and on `text_render` for
the face; neither depends on it, so the module graph stays acyclic.

## Risks / Trade-offs

- **The caret drifts from the painted text** if the two measure the spread
  differently -> one shared helper computes the extra, and a test asserts the
  caret's last stop against the spread line's advance.
- **The spread and the layout's advance measurement drift apart** -> the
  test asserts the spread line's advance equals the column width within a
  fraction of a pixel.
- **A line that cannot be spread is shifted by the last line's alignment** ->
  the render crate undoes that shift for those lines, and a test pins the
  flush-left result.
- **A face without a space glyph** would spread nothing while the text counts
  a space -> the helper reports no extra when the face has no space glyph, so
  both sides agree.
- **The Qt `Rasterize Type` command** still renders a justified layer
  unjustified, so the rasterized result and the layer's own pixels disagree
  -> recorded as out of scope; the bundled renderer is the one the type
  layer's own pixels and the live session use.
- **A justified line's underline spans the column**, not the ink -> correct,
  and the same as CS6, but it is a visible change for a layer that already
  carries a justification and an underline.
- **The measurement matches the render requirement** in
  `tools/type-character-rendering` says the paragraph wrap measures the same
  run the renderer paints. The wrap measures the *unspread* run - the breaks
  must be decided before the spread - while the renderer paints the spread
  run; the character attributes are the same in both, which is what that
  requirement is about.
