# Proposal

## Why

A text layer whose paragraph justification is anything but left, right or
centre renders left-aligned: the renderer collapses the seven justification
values the engine models to three alignments, and nothing spreads a line to the
column width. The value itself survives the PSD round trip - the codec reads
and writes all seven - but every re-render of the layer, in the live type
session or after any edit, lays the text out as if the paragraph were left
aligned. The Paragraph panel and the Paragraph Style dialog already offer the
seven values, so the panel's own choices do nothing.

## What Changes

- The renderer keeps the paragraph's justification kind instead of collapsing
  it. The bytes 0, 1 and 2 keep their left, right and centre alignment; 3
  (Justify Last Left), 4 (Justify Last Right) and 5 (Justify Last Center) keep
  the paragraph's last line aligned left, right and centre; 6 (Justify All)
  aligns every line left, and every line is spread, so the alignment never
  shows.
- A justified line spreads its word spaces evenly so that the line's advance
  equals the column width. The column is the paragraph box's width.
- The paragraph's last line is the one line a justification does not spread:
  Justify Last Left, Right and Center align it, Justify All spreads it too.
- A line with no word space to spread stays flush left, unless it is the
  paragraph's last line under a justify-last variant.
- A point-type layer has no box and so no column: its lines are not spread,
  and the justification variant still selects the line's alignment, so a
  Justify Last Right point-type layer is right-aligned. Its layer rect is
  anchored by that alignment, as a right-aligned point type always is.
- Vertical type keeps one placement per justification: 3 places the column at
  the top, 4 at the bottom and 5 at the centre, which are CS6's vertical
  Justify Last Top, Bottom and Centered. The vertical spread - Justify All
  filling the box with the columns - is not implemented.
- The caret measures the same spread, so it stands between the painted
  glyphs.
- Converting a justified paragraph to point text drops the spread: the lines
  set at their natural widths, anchored as the paragraph's last line was.
- The justification-to-alignment mapping in `pictura-core` becomes the single
  source of truth; the duplicate match in `pictura-render` is deleted.

Semantics follow the `Justify` names, the Paragraph panel's labels, and the
CS6 text: Justify All justifies every line including the last; Justify Last
Left, Right and Center justify every line but the last, which is aligned
left, right and center. Issue #792's expected-behaviour paragraph states the
opposite; this change follows the names and CS6.

Out of scope, still tracked elsewhere: the Justification dialog's word-spacing
min/desired/max bounds and right-to-left text (#518), vertical justification
(spreading the columns to fill the height), and the Qt `Rasterize Type` path.
That path is a separate renderer which already ignores the box, the tracking
and the leading, and it collapses the justify variants to `Qt::AlignLeft`, so
rasterizing a justified layer through the command yields a different raster
from the layer's own justified pixels.

## Capabilities

### New Capabilities

### Modified Capabilities

- `tools/text-render-seam`: the alignment requirement widens from the three
  alignments to the seven justification values, and a new requirement states
  how a justified paragraph spreads to the column width.
- `tools/paragraph-type`: converting a justified paragraph to point text
  drops the spread and anchors the lines as the paragraph's last line was.

## Impact

- `crates/pictura-core`: `TextAlign::from_justification` maps the justify
  variants; `TextAlign::factor` becomes public so the render crate shares the
  mapping; the "unknown justification falls back to left" test now covers 7
  and above.
- `crates/pictura-render`: `type_paragraph` exposes the rendered lines with
  their paragraph structure; `layout_type` spreads a justified line's word
  spaces and keeps an unspreadable line flush left; `BundledText` gains a
  space-glyph lookup; `align_factor` delegates to the core mapping;
  `type_caret` measures the same spread; a new `type_justify` module holds
  the justification concern.
- `crates/pictura-app`: unchanged - the Paragraph panel, the Paragraph Style
  dialog and the bridge already carry the seven values.
- No new dependencies.
