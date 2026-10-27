# Tasks

## 1. Rasterize keeps the on-canvas render

- [x] 1.1 `render_text_layer` keeps a type layer's rendered planes and bakes the text gamma into its alpha; a layer without planes keeps the bundled render; type locks, applied styles and overrides are dropped
- [x] 1.2 `rasterize_type` goes through `render_text_layer`; delete `render_text_rgba`, `render_text.{cpp,h}` and their build entries
- [x] 1.3 Retire self-test check `lpr_qt_text` (525) and lower `scripts/selftest-budget.txt` to 507
- [x] 1.4 Tests: `pictura-render::rasterize_keeps_a_transformed_type_layer_as_shown`, `pictura-render::rasterize_unlocks_type_and_drops_its_style_state`, `tst_type_tools::rasterizeKeepsTransformedType`

## 2. Font picker

- [x] 2.1 `isColorFont` tests for `COLR`/`CBDT`/`sbix`/`SVG `; the preview delegate skips colour faces
- [x] 2.2 `tst_paragraph_styles_panel::dialogFontFamilyPopup` picks a colour face (Latin preferred) for its timing check
- [x] 2.3 Rows draw the family name in the UI font and a face sample after it; `tst_paragraph_styles_panel::fontRowsNameFamiliesInTheUiFont`
- [x] 2.4 Samples need the face to carry their glyphs; `tst_paragraph_styles_panel::fontPopupPaintsPromptlyAcrossTheList` times the popup at every page
