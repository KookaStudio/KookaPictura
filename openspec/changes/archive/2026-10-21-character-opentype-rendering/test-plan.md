# Test plan

Engine tests use the bundled Liberation Sans, which has `dlig`, `sups`, `subs`,
and `kern` but no `smcp`. Where a feature is needed, they load the repository's
`assets/fonts/NotoSans-Regular.ttf`, which has `liga`, `smcp`, `frac`, `ordn`,
`salt`, and `onum`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Turning standard ligatures off breaks the ligature | Shape `fi` in Noto Sans with Standard Ligatures on, then off. | The on run has one glyph and the off run has two. | `pictura-render::standard_ligatures_off_breaks_the_ligature` |
| A discretionary feature changes the glyphs only when on | Shape `1/2` in Noto Sans with Fractions off, then on. | The glyph ids differ. | `pictura-render::fractions_change_the_glyphs_only_when_on` |
| A feature the face lacks changes nothing | Shape text in Liberation Sans (no `swsh`) with Swash off, then on. | The glyphs are identical. | `pictura-render::a_feature_the_face_lacks_changes_nothing` |
| Support follows the face's feature list | Query the support mask for Liberation Sans and Noto Sans. | Liberation sets only Discretionary Ligatures. Noto sets Standard Ligatures, Stylistic Alternates, Ordinals, and Fractions. | `pictura-render::support_mask_follows_the_feature_list` |
| All Caps sets upper case | Shape `type` with All Caps on, and `TYPE` plain. | The glyph ids are equal. | `pictura-render::all_caps_shapes_upper_case` |
| Synthesized small caps are smaller capitals | Shape `ab` in Liberation Sans with Small Caps. | The glyphs are `A` and `B` at scale 0.7, each advance 0.7 × the capital's. | `pictura-render::synthesized_small_caps_are_scaled_capitals` |
| Superscript is smaller and raised | Render `x` plain and with Superscript and compare the ink boxes. | The superscript ink is shorter and its bottom is above the plain baseline. | `pictura-render::superscript_is_smaller_and_raised` |
| Subscript is smaller and lowered | Render `x` plain and with Subscript. | The subscript ink bottom is below the plain ink bottom. | `pictura-render::subscript_is_smaller_and_lowered` |
| Faux Bold adds ink | Render `l` plain and with Faux Bold, and measure the line advance. | The bold render covers more pixels and its advance is wider. | `pictura-render::faux_bold_adds_ink_and_advance` |
| Faux Italic slants right | Render `l` plain and with Faux Italic, and compare the top row's centre with the bottom row's centre. | The italic offset (top minus bottom) is larger. | `pictura-render::faux_italic_slants_right` |
| Underline paints below the baseline | Render `xx` with Underline and without. | A row below the baseline is painted across the run only in the underlined render. | `pictura-render::underline_paints_below_the_baseline` |
| Strikethrough paints through the letters | Render `xx` with Strikethrough and without. | A row between the baseline and the x-height is fully covered across the gap between the letters. | `pictura-render::strikethrough_crosses_the_letters` |
| Manual zero turns metrics kerning off | Measure `AV` with Metrics, with Manual 0, and as `A` + `V` alone. | Manual 0 equals the unkerned sum and exceeds Metrics. | `pictura-render::manual_zero_turns_metrics_kerning_off` |
| Manual kerning widens the line | Measure `type` with Manual 0 and Manual 100. | The difference is 0.1 em × 4 glyphs. | `pictura-render::manual_kerning_widens_the_line` |
| Optical tightens an open pair | Measure `AV` with Optical and with Manual 0. | Optical is smaller. | `pictura-render::optical_tightens_an_open_pair` |
| All Caps widens the measured line | Measure `type` with All Caps off and on, and `TYPE` plain. | The All Caps advance equals `TYPE` and exceeds `type`. | `pictura-render::all_caps_widens_the_measured_line` |
| The caret's last stop meets the styled advance | Compute the caret stops for point type with All Caps and Faux Bold. | The last stop minus the first equals the styled line advance. | `pictura-render::caret_last_stop_meets_the_styled_advance` |
| Defaults for new type | Build `CharacterAttrs::default()`. | The CS6 defaults hold, kerning Metrics. | `pictura-core::new_type_defaults_are_cs6` |
| Every modelled character field round-trips | Set each bridge field non-default, convert to the model and back. | Every field reads back equal. | `pictura-app::character_setting_round_trips_every_field` |
| Optical kerning crosses the bridge | Convert kerning mode 1 to the model and back. | The model holds `Optical` and the mode reads back as 1. | `pictura-app::optical_kerning_crosses_the_bridge` |
| Panel reflects the active type layer | Select a type layer and refresh the panel. | The size, tracking, and toggles match the layer. | `tst_character_paragraph_panels::characterPanelReflectsAndEdits` |
| Editing an attribute updates the layer as one state | Commit a size and undo once. | One state is recorded and the undo restores the size. | `tst_character_paragraph_panels::characterPanelReflectsAndEdits` |
| No type layer shows the defaults | Refresh with no type layer and commit. | The defaults are shown and no state is recorded. | `tst_character_paragraph_panels::characterDefaultsWithoutType` |
| A style button applies its toggle | Press Faux Bold. | The layer's `faux_bold` is on with one Edit Type Layer state. | `tst_character_paragraph_panels::styleButtonAppliesToggle` |
| Exclusive caps buttons | Press All Caps, then Small Caps. | The layer has Small Caps on and All Caps off. | `tst_character_paragraph_panels::capsButtonsAreExclusive` |
| Exclusive position buttons | Press Superscript, then Subscript. | The layer has Subscript on and Superscript off. | `tst_character_paragraph_panels::positionButtonsAreExclusive` |
| An OpenType button applies its feature | Press the first enabled OpenType button, or force Discretionary Ligatures on if none is enabled. | The matching layer attribute flips with one state. | `tst_character_paragraph_panels::openTypeButtonAppliesFeature` |
| Buttons grey out for features the font lacks | Select a type layer and compare each button's enabled state with `type_api::openTypeFeatures` for its font. | Each button is enabled exactly when its bit is set. | `tst_character_paragraph_panels::openTypeButtonsFollowTheFont` |
| No type layer disables every OpenType button | Refresh with no type layer. | All eight OpenType buttons are disabled. | `tst_character_paragraph_panels::characterDefaultsWithoutType` |
| Picking Optical sets optical kerning | Activate the `Optical` item. | The layer's kerning mode is Optical (1). | `tst_character_paragraph_panels::kerningPicksOptical` |
| Typing a number sets manual kerning | Type `50` into the kerning combo and press Enter. | The kerning is Manual (2) with value 50. | `tst_character_paragraph_panels::kerningTypedNumberIsManual` |
| Picking Auto leading | Set a fixed leading, then activate `(Auto)`. | The leading mode is Auto (0). | `tst_character_paragraph_panels::leadingPicksAuto` |
| Dragging an icon right raises the value | Drag the horizontal-scale icon 20 px right. | The field and the layer read 120 with one state. | `tst_character_paragraph_panels::scrubIconRaisesValue` |
| Scrubbing kerning leaves Metrics | Drag the kerning icon 10 px right from Metrics. | Manual 10 with one state. | `tst_character_paragraph_panels::scrubKerningLeavesMetrics` |
| Panel visual match to CS6 | Open the Character panel beside the CS6 reference capture in #780. | Same grid order, icon rows, and greyed OpenType icons. | manual: visual likeness to a reference screenshot is a judgement call |
| JetBrains Mono greying | Set type in JetBrains Mono and open the Character panel. | Only Contextual Alternates, Ordinals, and Fractions are enabled. | manual: depends on a user-installed font that CI does not have |
