# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tg_curve` | Evaluate the curve at γ 1.45 over 0…1, and at γ 2.2 | 0→0, 1→1, 0.5→(0.36, 0.38), never above the input; identity at 2.2 | `pictura-render::curve_maps_coverage` |
| `tg_table` | Read the tables for γ 1.00, 1.45, 1.80, 2.20 across 0…1 | Within half a level of the curve (0.01 at γ 1.00 next to full coverage) | `pictura-render::the_table_matches_the_curve` |
| `tg_default_on` | Composite a black type layer with stored anti-aliased pixels over white, default setting | Partial pixels are no darker than the plain blend, one is strictly lighter; solid and empty pixels equal | `pictura-render::type_edges_thin_by_default` |
| `tg_opacity` | Composite a solid black type pixel at 50 % opacity over white | The result is white blended with `1 − 0.5^(1.45/2.2)` of black | `pictura-render::opacity_passes_through_the_curve` |
| `tg_pixel_layer` | Composite a pixel layer with alpha 128 over white | It equals the plain blend | `pictura-render::pixel_layers_skip_the_curve` |
| `tg_off` | Set the gamma off and composite the type layer | It equals the plain blend of its coverage | `pictura-render::off_reproduces_the_plain_blend` |
| `tg_rasterize` | Materialize an alpha ramp at 100 %; rasterize live type at 100 % and 50 % opacity and composite both | Stored alpha is the curve within one level; composites agree within two levels | `pictura-render::materialize_bakes_the_curve`, `pictura-render::rasterized_type_composites_as_live_type` |
| `tg_gpu` | Ask the GPU compositor for a document whose type layer has pixels | `UnsupportedText` | `pictura-render::gpu_defers_type_while_gamma_is_on` |
| `cs_text_gamma_default` | Open Color Settings with a fresh state home and press More Options | Checkbox on, value 1.45, controls visible | `tst_text_gamma_fonts::colorSettingsDefaults` |
| `cs_text_gamma_apply` | Commit black type, open Color Settings, clear the checkbox, OK; reopen and cancel | The composite under the type changes, the history count does not; the reopened dialog shows it off | `tst_text_gamma_fonts::colorSettingsAppliesGamma` |
| `cs_text_gamma_value` | Commit type, set the value to 1.80 in Color Settings, OK; reopen | Darker edges than at 1.45, no state, 1.80 stored and shown | `tst_text_gamma_fonts::colorSettingsAppliesValue` |
| `cs_text_gamma_persist` | Save a session with gamma off at 1.80 and load it | Off at 1.80 | `tst_text_gamma_fonts::gammaSessionRoundTrip` |
| `mf_detect` | Check `Liberation Sans`, an installed family, its PostScript stem, and an unknown name | Only the unknown name is missing | `tst_text_gamma_fonts::detectsMissingFonts` |
| `mf_alert` | Save a PSD with two type layers in an unknown font and open it | One `missingFontsAlert` box naming the font once | `tst_text_gamma_fonts::openAlertsMissingFonts` |
| `mf_no_alert` | Save a PSD with a Liberation Sans type layer and open it | No `missingFontsAlert` box | `tst_text_gamma_fonts::openWithoutMissingFontsIsQuiet` |
| `mf_badge` | Commit one type layer in an unknown font and one in Liberation Sans | Only the first row reports a missing font | `tst_text_gamma_fonts::layersRowMarksMissingFont` |
| `mf_replace_engine` | `replace_type_fonts` over a grouped layer in the missing font and a layer in another font | Returns 1; the grouped layer is Liberation Sans with the same text; the other is unchanged | `pictura-render::replace_type_fonts_replaces_named_fonts` |
| `mf_style_override` | Give a missing-font layer a Bold style override and replace | Style and override are Regular; no family override | `pictura-render::a_style_override_follows_the_replacement` |
| `mf_replace_command` | Dispatch Replace All Missing Fonts over two layers in an unknown font, then undo | Both Liberation Sans, both rows unmarked, one named state; undo restores the font on both | `tst_text_gamma_fonts::replaceAllMissingFonts` |
| `mf_replace_not_rgb8` | Convert a document with missing-font type to Grayscale | Layer stays type; command disabled; a replacement changes nothing | `tst_text_gamma_fonts::replaceDisabledOutsideRgb8`, `pictura-render::a_document_type_cannot_be_set_in_is_unchanged` |
| `mf_replace_disabled` | Refresh the registry with only Liberation Sans type | The command is disabled | `tst_text_gamma_fonts::replaceDisabledWithoutMissingFonts` |
| Visual match to CS6 | Render the same type in CS6 and Pictura at γ 1.45 and compare edges | Edge weight and translucent-type opacity look alike | manual: no CS6 reference renders exist; the curve is an approximation |
| Missing-font badge artwork | Open a document with a missing font and look at the Layers panel | The warning triangle on the T card reads in light and dark themes | manual: the offscreen suite checks the row's role, not how the badge looks |
