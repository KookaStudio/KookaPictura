# Design

## Context

See proposal.md - Why.

The state that shapes the approach:

- `pictura-render::type_caret` re-derives every pen position from the text,
  character prefix by character prefix. It does not read the layout, so it
  has to decide for itself which lines a justification spreads and how each
  line is shifted.
- `pictura-render::text_render::layout_type` spreads a line by widening its
  space glyphs before the layout, and undoes the layout's alignment shift for
  a line it could not spread.
- The caret's `VisualLine::soft` flag marks a *wrapped* line, so the
  paragraph's last line is the one with `soft == false`. The renderer's
  spread set is `line.soft || spreads_last(justify)`; the caret had
  `!line.soft || spreads_last(justify)`.

## Goals / Non-Goals

**Goals:**

- The caret's stops stand where the renderer paints, for every justification
  and every line of a paragraph.
- A justified line's advance equals the column width, whatever the face's
  space glyph maps to.
- The corpus says what the code does, where the corpus was silent or wrong.

**Non-Goals:**

- The caret's pre-existing imprecision: it re-shapes each prefix, so it
  misses the kerning across a prefix boundary - a fraction of a pixel, not a
  word's width.
- Making the Qt `Rasterize Type` path justify. Qt's text layout cannot
  justify a last line to the right or centre it; the corpus records the
  ceiling instead.

## Decisions

### The caret mirrors the renderer, line for line

The renderer spreads a line iff `line.soft || spreads_last(justify)`: every
wrapped line of a justified paragraph, and the paragraph's last line under
Justify All. The caret now uses the same predicate, and its shift follows the
renderer's three cases: a spread line fills the column (no shift), a line
that could not be spread stays flush left (no shift), and any other line
takes `(column - advance) * factor`.

The shared `type_justify` module is the single place the rule lives. The
caret asks it whether a justification spreads, and the renderer asks it the
same question, so the two cannot drift.

### A line is spread only when its spaces and its space glyphs agree

The renderer counts the word spaces in the line's *text* and widens the space
*glyphs* in the shaped run. A face that maps another character to the same
space glyph - a no-break space, say - would make the two counts differ: the
line's advance would overshoot the column by one extra per space, and the
caret, which divides by the text's count, would drift from the ink.

So the helper compares the two counts and reports no extra when they differ,
and the line is not spread. The renderer has the shaped run; the caret
shapes the trimmed line once to have it too.

### The spread orchestration lives in `type_justify`

`layout_type` grew a loop, an advance estimate and a flush-left correction -
about thirty lines in a file already over the 800-line target. The module
that owns the justification now owns the loop too: `spread_lines` returns
which lines it spread, and `flush_left` shifts the lines it could not spread
back to the margin. `layout_type` keeps the layout.

## Risks / Trade-offs

- **The caret shapes each line twice** (once for the prefix advances, once
  for the space glyphs) -> the caret already re-shapes every prefix, so this
  is the same order of work.
- **A face with a shared space glyph is not justified at all** -> the
  alternative is a line that overshoots the column and a caret off the ink.
- **The corpus's wording changes are requirement-level** -> they are filed as
  a delta on the capabilities they touch, not as an edit to the main specs.
