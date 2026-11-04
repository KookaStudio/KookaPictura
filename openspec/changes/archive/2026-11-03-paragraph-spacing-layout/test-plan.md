# Test Plan

Covers every scenario in the `tools/paragraph-spacing` delta for
`paragraph-spacing-layout`. All rows are engine tests: the renderer, caret, and
placement share the same layout, so a Rust test exercises the exact path the
canvas preview, commit, warp, area type, and PDF pens use. No manual rows are
expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `indent_box_horizontal` | Lay out a horizontal box with left indent 20, right indent 10, and first-line indent 15; measure each line's first and last glyph x; lay out text sized to fit only when the indent is zero | The first line's first pen is 35 px from the box left, later first pens are 20 px, every line's last pen is at least 10 px from the box right, and the unindented-only text wraps once the indent applies | `pictura-render::paragraph_indents_shape_the_box_lines` |
| `indent_box_vertical` | Lay out a vertical box with top indent 20, bottom indent 10, and first-line indent 15; measure the first column's first glyph y and the second column's first glyph y | The first column starts 35 px below the box top, the second starts 20 px below, and each column holds only the characters that fit the indented height | `pictura-render::vertical_indents_shape_the_columns` |
| `indent_point` | Render point type with left indent 20 and first-line indent 15, then compare its ink bounds with the click and with an unindented render | Every hard line (each is a paragraph) starts 35 px right of the click, and the placement rect covers all of it | `pictura-render::point_type_indents_move_the_glyphs` |
| `space_before_after` | Lay out two horizontal paragraphs with space after 12 and space before 8; measure the baseline gap; repeat in a box tall enough only without spacing | The baselines are 20 px farther apart than the unspaced layout, and the short box reports the second paragraph as overflow | `pictura-render::space_before_and_after_separate_paragraphs` |
| `space_vertical` | Lay out two vertical paragraphs with space before 8 and space after 12; compare the column gap and the point-type placement width | The columns are 20 px farther apart than the unspaced layout, and the point-type rect includes the trailing space | `pictura-render::vertical_space_before_and_after_separate_columns` |
| `caret_indent` | Ask for the caret stops of a wrapping paragraph box with left indent 10 and first-line indent 20; compare the first stop of each visual line with the laid-out glyph pen | The first line's start stop is 30 px right of the box's left edge, the wrapped line's is 10 px right, and each equals its glyph's pen | `pictura-render::caret_follows_indented_lines` |
| `bounds_indent` | Place point type with first-line indent -15 and space after 12; inspect the placement rect against the click and the last baseline | The rect starts at least 15 px left of the click and reaches below the last baseline by the space after | `pictura-render::point_bounds_include_indents_and_spacing` |
| `spacing_ceilings` | Lay out the same text with hanging punctuation, hyphenation, and the every-line composer toggled; compare line texts and glyph pens with the default layout | Every line text and glyph pen is identical to the default layout | `pictura-render::unsupported_paragraph_attributes_do_not_move_glyphs` |
