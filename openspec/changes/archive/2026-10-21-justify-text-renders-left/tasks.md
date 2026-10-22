# Tasks

## 1. The tests that pin the new behaviour

- [x] 1.1 Add `pictura-core::a_justify_variant_maps_to_its_last_line_alignment` to `crates/pictura-core/src/text_render.rs`: the justification bytes 3, 4 and 5 map to `Left`, `Right` and `Center`. It fails today, where 4 and 5 map to `Left`.
- [x] 1.2 Extend `pictura-core::justification_maps_and_unknown_falls_back_to_left` so the byte 6 maps to `Left` and only 7 and above fall back. It fails today for 4, 5 and 6.
- [x] 1.3 Create `crates/pictura-render/src/type_justify.rs` with its test block, declare it in `crates/pictura-render/src/lib.rs`, and add `pictura-render::a_justified_line_fills_the_column`: a two-line paragraph in a 200 px box set to Justify Last Left has a first line whose advance is 200 px within a fraction of a pixel and which starts at the box's left edge. It fails today.
- [x] 1.4 Add `pictura-render::the_last_line_keeps_its_variants_alignment`: the same paragraph set to Justify Last Right spreads the first line and right-aligns the last one inside the column. It fails today.
- [x] 1.5 Add `pictura-render::justify_all_spreads_the_last_line`: the same paragraph set to Justify All spreads both lines. It fails today.
- [x] 1.6 Add `pictura-render::a_line_with_no_word_space_stays_flush_left`: a Justify Last Right paragraph with a line that has no word space and is not the last keeps that line flush left. It guards the new rule: the old mapping put every line flush left, so the assertion holds there too.
- [x] 1.7 Add `pictura-render::a_point_type_layer_has_no_column`: a point-type layer with two lines of different lengths set to Justify All spreads neither line and keeps each line's alignment. It guards the rule that a layer without a box has no column.
- [x] 1.8 Add `pictura-render::a_justify_variant_places_a_vertical_column`: a vertical type layer set to Justify Last Right sits at the bottom of its box. It fails today.
- [x] 1.9 Add `pictura-render::the_caret_stands_where_the_spread_glyphs_are` to `crates/pictura-render/src/type_caret.rs`: the last caret stop of a spread line stands at the column's right edge. It fails today.
- [x] 1.10 Add `pictura-render::a_justified_paragraph_loses_its_spread_when_converted` to `crates/pictura-render/src/type_paragraph.rs`: a Justify Last Right paragraph box converted to point text sets its lines at their natural widths, right-aligned, the first line ending where the paragraph's first line ended. It fails today.

## 2. The core mapping

- [x] 2.1 Map the justify variants in `pictura-core`'s `TextAlign::from_justification` (3 to `Left`, 4 to `Right`, 5 to `Center`, 6 to `Left`) and make `TextAlign::factor` public. Verify `cargo test -p pictura-core` passes.
- [x] 2.2 Replace `pictura-render`'s `align_factor` match with `TextAlign::from_justification(byte).factor()`, keeping the `u8` signature its call sites use, and check the vertical placement still reads as top, bottom and centre. Verify `cargo clippy --workspace --all-targets` is clean.

## 3. The spread

- [x] 3.1 Fill `crates/pictura-render/src/type_justify.rs` with the justification concern: whether a justification spreads, whether it spreads the paragraph's last line, the extra width each word space takes when a line is spread to the column, and the widening of a shaped line's space glyphs. Keep the spread logic out of `crates/pictura-render/src/text_render.rs`, which is already over the 800-line target. Verify the module's own unit tests pass.
- [x] 3.2 Add `BundledText::space_glyph`, the face's space glyph id, and use it to find the word spaces of a shaped line. Report no extra when the face has no space glyph, so the renderer and the caret agree. Verify the helper finds one space per space character of a shaped run.
- [x] 3.3 Expose the rendered lines with their paragraph-last flag from `pictura-render`'s `type_paragraph::Lines`, reading the structure from the whole text so a paragraph cut short by the overflow still spreads its last visible line. Verify `cargo test -p pictura-render` passes.
- [x] 3.4 Spread a justified line's word spaces in `layout_type` before the layout, so the line's advance lands on the column width, and undo the last line's alignment for a line that cannot be spread so it stays flush left. Verify the tests of 1.3, 1.4, 1.5 and 1.6 pass.

## 4. The caret

- [x] 4.1 Add the per-space extra to the caret's pen positions, zero the shift of a spread line and keep the shift of a line that cannot be spread, through the shared helper. Verify the test of 1.9 and the existing caret tests pass.

## 5. The record and the delivery

- [x] 5.1 Drop the clause that names the justification variants as not laid out from the `ponytail:` comment in `crates/pictura-render/src/text_render.rs`, keeping the items that are still not laid out.
- [x] 5.2 Record the change in `docs/dev/STATE.md` as a new entry, leaving the text render seam's historical entry as it was. Commit the change with `TASK-ALLOWS-DOCS`.
- [x] 5.3 Run `openspec validate --all --strict`, then `openspec archive` the change and open the pull request closing #792. The build and test gates (`bash scripts/verify-fast.sh`) were not run in this session and must pass before the pull request merges.
