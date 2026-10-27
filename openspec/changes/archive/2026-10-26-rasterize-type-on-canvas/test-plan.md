# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rtoc_transformed` | Set a type layer, Free Transform it 3×, move it, rasterize it | Rect and colour planes are the on-canvas render; alpha is the baked gamma; composite unchanged within 2 | `pictura-render::rasterize_keeps_a_transformed_type_layer_as_shown` |
| `rtoc_app` | Commit a type layer, stretch it with Free Transform handles, run Rasterize Type | One `Rasterize Type` state, not type, same rect, sampled composite unchanged within 2 | `tst_type_tools::rasterizeKeepsTransformedType` |
| `rtoc_unlocked` | Rasterize a type layer carrying its type locks and an applied paragraph style | No locks, no applied style | `pictura-render::rasterize_unlocks_type_and_drops_its_style_state` |
| `rtoc_color_font` | Set text in Bitcount Prop Single Ink at 24 px, Rasterize Type | Same red dot glyphs as on canvas; no rainbow tiles or checkerboards | manual: needs the Bitcount COLR font installed; the bundled fonts carry no colour face, and the keep-the-planes rule it relies on is covered by `pictura-render::rasterize_keeps_a_transformed_type_layer_as_shown` |
| `ffp_color_fast` | Open the family popup with a colour face current (Latin colour face preferred), grab it | Rendered within 150 ms | `tst_paragraph_styles_panel::dialogFontFamilyPopup` |
| `ffp_dropdown_items` | Open the options bar family dropdown at every page of the list and grab it | Each popup paints within 250 ms; with Bitcount installed, opening it on Bitcount Prop Single Ink shows rows | `tst_paragraph_styles_panel::fontPopupPaintsPromptlyAcrossTheList` |
| `ffp_ui_font_names` | Draw the family-picker row of the lightest installed face | In the open popup the name reaches full contrast (> 120 gray levels from the highlight) | `tst_paragraph_styles_panel::fontRowsNameFamiliesInTheUiFont` |
