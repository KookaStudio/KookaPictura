# Proposal

## Why

`ParagraphAttrs` stores left/right/first-line indents and space before/after,
the Paragraph panel edits them, and the PSD codec round-trips them, but no
layout code reads any of the five values. Changing a first-line indent therefore
has no visual effect on point or paragraph type, and the other four spacing
fields have the same gap. Issue #1144 records the first-line case and asks for
the rest to be checked.

## What Changes

- Horizontal type lays every visual line in a column inset by the left and right
  indents; the first line of each paragraph adds the first-line indent relative
  to the left indent, and a negative value hangs into the left margin.
- Vertical type maps the same fields to top/bottom indents and applies the
  first-line indent to the first column of each paragraph.
- Space before and after offset paragraph baselines (horizontal) or paragraph
  column gaps (vertical), including the first paragraph's space before and the
  last paragraph's space after in point-type bounds.
- Point type honors the same fields; its placement rect grows to include the
  indented and spaced ink instead of ignoring them.
- The caret stops, justification spread, and point-type bounds consume the same
  per-line columns as the renderer, so the caret, preview, commit, warp, area
  type, and PDF pens stay together.
- Hanging punctuation, hyphenation, and the composer remain authored-only
  ceilings; on-path text keeps its arc-length layout.

## Capabilities

### New Capabilities

- `tools/paragraph-spacing`: indents and paragraph spacing in type layout for
  horizontal and vertical, point and paragraph type, with the caret and bounds
  following the laid-out lines.

### Modified Capabilities

None.

## Impact

- `crates/pictura-render/src/type_paragraph.rs`: visual lines carry their
  paragraph and first-line identity plus a per-line box; wrapping uses the
  indented width/height; paragraph spacing produces block offsets and the
  visible prefix.
- `crates/pictura-render/src/text_render.rs`: horizontal placement and
  justification spread use each line's box; the stale "indents are not laid
  out" ceiling is removed.
- `crates/pictura-render/src/type_justify.rs`: the spread takes per-line column
  widths; the post-hoc `flush_left` pass is replaced by per-line placement.
- `crates/pictura-render/src/type_caret.rs`: caret stops use the same boxes.
- `crates/pictura-render/src/type_layer.rs`: point-type bounds include indents
  and paragraph spacing.
- No document model, codec, app bridge, dependency, or golden-baseline change.
