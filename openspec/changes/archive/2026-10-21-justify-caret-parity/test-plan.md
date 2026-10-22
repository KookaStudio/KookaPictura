# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `justify_spread` | Lay out a two-line paragraph in a 200 px box set to Justify Last Left | The first line's advance is 200 px within a fraction of a pixel, and it starts at the box's left edge | `pictura-render::a_justified_line_fills_the_column` |
| `justify_last_line` | The same paragraph set to Justify Last Right | The first line is spread to the column width; the last line is right-aligned inside it | `pictura-render::the_last_line_keeps_its_variants_alignment` |
| `justify_all` | The same paragraph set to Justify All | Both lines are spread to the column width | `pictura-render::justify_all_spreads_the_last_line` |
| `justify_no_space` | Lay out a Justify Last Right paragraph with a line that has no word space and is not the last | That line is not spread and stays flush left; the paragraph's last line takes the variant's alignment | `pictura-render::a_line_with_no_word_space_stays_flush_left` |
| `justify_no_column` | Lay out a point-type layer with two lines of different lengths set to Justify All | Neither line is spread; each line keeps the variant's alignment | `pictura-render::a_point_type_layer_has_no_column` |
| `justify_caret` | Compute the caret stops of a justified paragraph, and compare them with the painted glyphs of a spread wrapped line | Each stop stands where the spread glyphs are, within a fraction of a pixel | `pictura-render::the_caret_stands_where_the_spread_glyphs_are_on_a_wrapped_line`, `pictura-render::the_caret_stands_where_the_spread_glyphs_are` |
| `para_convert` | Convert a point layer to paragraph text and back through the Type menu | Each command enabled only for its kind; one state each; the box appears and disappears; the same text | `tst_paragraph_type::convertCommands` |
| `para_convert_engine` | Convert two-line point type to paragraph and back; convert wrapped, overflowing paragraph type to point | Identical pixels; the text and origin restored; the wrapped text return-separated without the overflow | `pictura-render::conversion_keeps_glyphs_in_place` |
| `para_convert_justify` | Convert a Justify Last Right paragraph box to point text | The lines set at their natural widths, right-aligned, the first line ending where the paragraph's first line ended | `pictura-render::a_justified_paragraph_loses_its_spread_when_converted` |
| A type layer rasterizes through Qt | Run `Type > Rasterize Type` on a type layer and look at the raster | The layer's channels hold the Qt-rendered text, it is no longer a type layer, and one state is recorded | manual: the Qt rasterize path has no automated case; the app's self-test calls the text helper, not the command |
| An empty Qt render falls back to the bundled face | Rasterize a type layer whose text the Qt render cannot draw | The bundled backend materializes the layer instead | manual: the Qt rasterize path has no automated case |
| A non-type layer is refused | Run `Type > Rasterize Type` on a plain pixel layer | It returns false and records no state | manual: the Qt rasterize path has no automated case |
