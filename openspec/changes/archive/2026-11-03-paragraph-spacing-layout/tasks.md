# Tasks

## 1. Plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in
  `specs/tools/paragraph-spacing/spec.md`.

## 2. Tests first

- [x] 2.1 Add `pictura-render::paragraph_indents_shape_the_box_lines` in
  `type_paragraph.rs`: a horizontal box with left 20, right 10, first-line 15
  starts the first line at 35, later lines at 20, ends every line at least 10
  from the right edge, and wraps text that fits only without the indent.
- [x] 2.2 Add `pictura-render::vertical_indents_shape_the_columns`: a vertical
  box with top 20, bottom 10, first-line 15 starts the first column at 35 and
  the second at 20, and chunks by the indented height.
- [x] 2.3 Add `pictura-render::point_type_indents_move_the_glyphs`: point type
  with left 20 and first-line 15 puts the first line 35 right of the click and
  later lines 20 right, and the placement rect covers the ink.
- [x] 2.4 Add `pictura-render::space_before_and_after_separate_paragraphs`: two
  horizontal paragraphs with space after 12 and before 8 are 20 px farther
  apart, and a short box hides the second.
- [x] 2.5 Add
  `pictura-render::vertical_space_before_and_after_separate_columns`: two
  vertical paragraphs with space before 8 and after 12 are 20 px farther apart,
  and the point-type rect includes the trailing space.
- [x] 2.6 Add `pictura-render::caret_follows_indented_lines`: the first start
  stop is 30 px right of the click and the second is 10 px right.
- [x] 2.7 Add `pictura-render::point_bounds_include_indents_and_spacing`: a
  negative first-line indent reaches left of the click and space after reaches
  below the last baseline.
- [x] 2.8 Add `pictura-render::unsupported_paragraph_attributes_do_not_move_glyphs`:
  hanging punctuation, hyphenation, and the every-line composer leave every
  line text and glyph pen unchanged.

## 3. Line geometry

- [x] 3.1 Give `VisualLine` its paragraph index and a `first` flag, and add a
  parallel `LineBox` (`start`, `extent`, `block`) to `Lines`.
- [x] 3.2 Compute horizontal line boxes in `visual_lines`: inset each line by
  the left/right indents, add the first-line indent to the first line of each
  paragraph, wrap to that line's width, and apply space before/after to the
  block offset.
- [x] 3.3 Compute vertical line boxes: map left/right to top/bottom, apply the
  first-line indent to the first column, chunk by the indented height, and add
  paragraph spacing to the inter-column block offset.
- [x] 3.4 Count the visible prefix from each line's box (baseline plus
  ascent/descent for horizontal, column edge for vertical) and expose the
  point-type inline bounds and block size.

## 4. Consumers

- [x] 4.1 In `layout_type`, lay out with no shared column, shift each
  horizontal line into its box, apply the alignment factor within the box, and
  add the block offset to its baseline.
- [x] 4.2 In `type_justify`, spread each line to its own column width and
  replace `flush_left` with the per-line placement.
- [x] 4.3 In `type_caret`, read the boxes and visible prefix from
  `visual_lines` for both orientations.
- [x] 4.4 In `text_box_any`, include point-type indent and spacing bounds from
  the line geometry, and keep the point-type origin semantics.
- [x] 4.5 In `type_to_point`, keep the first line where the indented box set it.
- [x] 4.6 Replace the stale `ponytail:` ceiling in `text_render.rs` with the
  remaining hanging/hyphenation/composer ceiling.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `cargo nextest run -p pictura-render`; confirm all pass.
- [x] 5.2 Run `openspec validate --all --strict` and `bash
  scripts/verify-fast.sh`; confirm both pass.
- [x] 5.3 Archive the change with `openspec archive paragraph-spacing-layout`,
  commit `fix(render): apply paragraph indents and spacing in type layout
  (#1144)`, and open the PR titled `fix: apply paragraph spacing in type layout
  (#1144)` with the manual rows from `test-plan.md` (none expected).
