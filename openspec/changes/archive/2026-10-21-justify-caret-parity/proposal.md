# Proposal

## Why

A review of the justified paragraph change (#792) found that the caret's
spread condition was inverted: it spread the paragraph's *last* line and
refused to spread the wrapped lines, so for every justify-last variant the
live Type session drew its caret detached from the letters - up to 13 px
away, a word's width. The renderer was right; the caret, which re-measures
the text itself, was not.

The same review found gaps in the merged corpus: the spread requirement does
not say that it is a horizontal-type rule, its caret scenario claims a
wrapped line's last stop reaches the column's right edge (the caret's own
convention puts a wrap's end boundary at the next line's start), the
paragraph-to-point conversion requirement keeps "keeping the first line where
it was" beside the new sentence about a justified paragraph losing its
spread, and the Qt `Rasterize Type` path says nothing about the justify
variants it cannot render.

## What Changes

- The caret spreads the lines the renderer spreads: a wrapped line of a
  justified paragraph, and the paragraph's last line under Justify All. Its
  shift rule follows the renderer's: a spread line fills the column, a line
  that could not be spread stays flush left, and the paragraph's last line
  takes its variant's alignment.
- A line is spread only when the shaped run carries one space glyph per
  space character. A face whose space glyph another character shares - a
  no-break space mapped to the same glyph, say - would otherwise overshoot
  the column and drift the caret away from the ink.
- The spread orchestration moves out of `text_render.rs` into the
  `type_justify` module, which now owns the whole justification concern.
- The corpus states that the spread is a horizontal-type rule, that a
  vertical paragraph's lines are placed rather than spread, that a wrapped
  line's caret stops stop short of its end, that a justified paragraph's
  conversion to point text loses the spread, and that the Qt `Rasterize
  Type` path renders the justify variants left-aligned.

## Capabilities

### New Capabilities

### Modified Capabilities

- `tools/text-render-seam`: the spread requirement is scoped to horizontal
  type and states the vertical placement, and its caret scenario states the
  wrap's end-boundary convention.
- `tools/paragraph-type`: the conversion requirement says that a justified
  paragraph's lines lose their spread.
- `tools/text-rasterize-bundled`: the Qt `Rasterize Type` requirement says
  which justification bytes it honours.

## Impact

- `crates/pictura-render`: the caret's spread condition and shift rule; the
  space-count guard in `type_justify`; the spread orchestration moved into
  `type_justify`; two new engine tests (the caret against a spread line, and
  the paragraph's last line with no word space).
- No new dependency, no change to the PSD format, and no change to the
  rendering of a document that uses left, right or centre alignment.
