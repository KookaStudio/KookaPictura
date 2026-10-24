# Test plan

Engine tests use the bundled Liberation Sans; the fallback and style tests also
use `assets/fonts/NotoSans-*.ttf`, which covers codepoints Liberation lacks
(for example U+0370) and ships Regular/Bold/Italic faces. Qt suites run
offscreen through CTest. Rows whose scenario title repeats across requirements
appear once per requirement, with the distinguishing case named in Steps.

## codec/psd-type-tool

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A modelled character value survives author and re-read | Author a spec with tracking 120 and horizontal scale 85, then read it back. | The decoded style reports both values. | `pictura-codec::authored_type_tool_reads_back_as_written` |
| A modelled paragraph value survives author and re-read | Author a spec with first-line indent 24 and space after 12, then read it back. | The decoded paragraph reports both values. | `pictura-codec::every_modelled_character_and_paragraph_key_reads_back` |
| The anti-aliasing method is authored to the descriptor and the flag | Author each of the five methods. | Each `AntA` spelling and `/AntiAlias` flag agree. | `pictura-codec::each_anti_alias_method_authors_to_descriptor_and_flag` |
| Tsume, language, and direction survive author and re-read | Author tsume 60, language `French`, direction RTL; decode. | All three read back. | `pictura-codec::tsume_language_and_direction_survive_author_and_read` |
| A re-author keeps an existing direction | Author RTL, re-author an unrelated attribute; decode. | Direction is still RTL. | `pictura-codec::existing_direction_survives_unrelated_reauthor` |

## document/type-style-model

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Defaults for new type | Create a character set for new type. | Scales 100, Metrics kerning, Sharp anti-aliasing, offsets 0, language `English: USA`, tsume 0, toggles off, fractional widths on. | `pictura-core::character_defaults_include_language_and_tsume` |
| Every modelled character field round-trips | Set every character field, including language and tsume, then read each. | Each value equals what was set. | `pictura-core::every_character_field_round_trips` |
| Optical kerning crosses the bridge | Convert a setting with Optical kerning to the model and back. | The model and setting both report Optical. | `pictura-app::optical_kerning_crosses_the_bridge` |
| Defaults for new type | Create a paragraph set for new type. | Left alignment, zero indents/spacing, hanging and hyphenation off, direction LTR, single-line composer, word spacing `[0.8 1.0 1.33]`. | `pictura-core::paragraph_defaults_include_direction` |
| Every modelled paragraph field round-trips | Set every paragraph field, including direction, then read each. | Each value equals what was set. | `pictura-core::every_paragraph_field_round_trips` |

## tools/missing-glyph-protection

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A single uncovered character falls back | Shape `A` + U+0370 in Liberation Sans with Noto Sans registered and protection on. | U+0370 uses Noto Sans and `A` uses Liberation Sans. | `pictura-render::fallback_single_character_uses_covering_face` |
| A mixed line measures with the fallback advance | Wrap a mixed line to a narrow box. | Line width, break point, and final caret stop use the fallback advance. | `pictura-render::mixed_line_wrap_uses_fallback_advance` |
| The preference can be off | Render the same layer with protection off. | No glyph uses a fallback face. | `pictura-render::fallback_off_uses_active_face` |
| The active face wins when it covers the character | Register a second face that also covers `A`. | `A` still uses the active face. | `pictura-render::fallback_prefers_active_face` |
| No covering face keeps the active face | Shape a codepoint no registered face covers. | The active face renders it and rendering succeeds. | `pictura-render::fallback_without_cover_keeps_active_face` |

## tools/paragraph-type

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The first character starts at the right edge | Lay out an RTL paragraph box. | The first logical glyph is at the line's right edge. | `pictura-render::rtl_line_starts_at_right_edge` |
| Wrapping breaks at the same words | Lay out the same text in equal-width LTR and RTL boxes. | Same word breaks and line count. | `pictura-render::rtl_wraps_at_same_words` |
| Alignment keeps its named edge | Center, then right-align an RTL paragraph. | Centered line is centered; right-aligned line touches the right edge. | `pictura-render::rtl_alignment_keeps_named_edge` |
| Overflow is counted in logical lines | Overflow RTL text in a short box. | Extra logical lines are hidden and overflow is reported. | `pictura-render::rtl_overflow_counts_logical_lines` |
| Word spacing absorbs the leftover first | Justify a line with room to grow. | Word spaces grow; letter spacing and glyph scaling stay at Desired. | `pictura-render::word_spacing_absorbs_the_leftover` |
| Excess width spills into letter spacing | Cap word spacing below the leftover. | Letter spacing absorbs the remainder within its bounds. | `pictura-render::letter_spacing_takes_excess` |
| Bounds clamp the spread | Exceed all three maxima. | Line uses the maxima and stays ragged. | `pictura-render::bounds_clamp_and_leave_ragged` |
| Auto leading uses the stored value | Set Auto Leading 130 and fixed-size text. | Baselines are 130% of the size apart. | `pictura-render::auto_leading_uses_paragraph_value` |

## tools/text-shaping

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| An RTL line reverses its visual order | Shape `אבג` with RTL direction. | Visual glyph order is `גבא`. | `pictura-render::rtl_line_reverses_visual_order` |
| Mixed directions keep their runs intact | Shape an RTL line containing an LTR Latin word. | The Latin run stays LTR while the surrounding text stays RTL. | `pictura-render::mixed_direction_keeps_runs` |
| An LTR line is unchanged | Shape the same text with LTR direction. | Glyph order and advances equal the pre-direction shape. | `pictura-render::ltr_line_unchanged_by_direction` |

## tools/type-caret

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Position zero is at the right edge | Open an empty RTL paragraph. | Caret is at the line's right edge. | `pictura-render::rtl_caret_zero_is_at_right_edge` |
| Advancing moves left | Advance one character from position 0 in RTL. | Caret draws at the left side of the first character. | `pictura-render::rtl_caret_advances_left` |
| A click maps to the logical boundary | Click between two RTL characters. | Caret lands on the logical boundary on the clicked side. | `pictura-render::rtl_click_maps_to_logical_boundary` |
| An LTR caret is unchanged | Repeat with LTR text. | Position 0 is left and advancing moves right. | `pictura-render::ltr_caret_unchanged` |

## tools/type-character-rendering

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Support follows the face's feature list | Query Liberation Sans and Noto Sans support masks. | Each bit matches the face's feature tags. | `pictura-render::support_mask_follows_the_feature_list` |
| Style support follows the selected face | Register Noto Sans Regular and Bold with different features. | Bold's mask equals the Bold face's features. | `pictura-render::style_support_follows_selected_face` |
| A bold style renders a different face | Render `A` in Noto Sans Regular and Bold. | The ink differs. | `pictura-render::bold_style_renders_different_face` |
| An unregistered style falls back to Regular | Request `Black` from a family without it. | The Regular face renders and attributes are unchanged. | `pictura-render::unregistered_style_falls_back_to_regular` |
| Style selection is case- and spacing-insensitive | Request `bold italic` for `BoldItalic`. | The Bold Italic face is used. | `pictura-render::style_name_matching_ignores_case_and_space` |

## tools/type-engine-data

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A real text layer exposes its font and size | Read the fixture with size 150 and font index 1. | Style reports the font and size. | `pictura-codec::extraction_resolves_run_against_defaults` |
| A run omitting a character attribute resolves to the sheet default | Omit `Tracking`; set the sheet to 40. | Style reports 40. | `pictura-codec::run_omissions_resolve_to_sheet_and_paragraph_defaults` |
| A run omitting a paragraph attribute resolves to the paragraph default | Omit `SpaceAfter`; set the sheet to 12. | Paragraph reports 12. | `pictura-codec::run_omissions_resolve_to_sheet_and_paragraph_defaults` |
| The anti-aliasing method is reported from the descriptor | Set `AntA` to Crisp. | Style reports Crisp. | `pictura-codec::extraction_resolves_run_against_defaults` |
| The EngineData flag is the anti-aliasing fallback | Omit `AntA`; set `/AntiAlias`. | Style reports Sharp. | `pictura-codec::anti_alias_flag_is_the_fallback` |
| A missing EngineData leaves the style unset | Remove the EngineData value. | Style is `None`; read succeeds. | `pictura-codec::missing_style_data_uses_fallbacks` |
| A malformed EngineData does not fail the document | Give an unparseable EngineData value. | Style is `None`; layer reads with text and raw bytes. | `pictura-codec::missing_style_data_uses_fallbacks` |
| Language resolves to the sheet default | Omit `/Language`; set the sheet to `German`. | Style reports `German`. | `pictura-codec::language_falls_back_to_sheet_default` |
| Tsume, language, and direction decode | Set `/Tsume` 60, `/Language` `French`, `WritingDirection` 1. | Style reports 60, `French`, RTL. | `pictura-codec::tsume_language_and_direction_decode` |
| A missing tsume or direction takes the default | Omit both keys. | Style reports tsume 0 and LTR. | `pictura-codec::missing_tsume_and_direction_take_defaults` |

## ui/character-paragraph-panels

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Panel reflects the active type layer | Select a type layer with Show Asian Text Options on. | Character fields show the layer's style, language, tsume, fractional widths, and other values. | `tst_character_paragraph_panels::characterPanelReflectsAndEdits` |
| Editing an attribute updates the layer as one state | Commit a size and undo once. | Text re-renders and one undo restores the size. | `tst_character_paragraph_panels::characterPanelReflectsAndEdits` |
| No type layer shows the defaults | Deselect every type layer and commit a value. | Defaults show and no history state is recorded. | `tst_character_paragraph_panels::characterDefaultsWithoutType` |
| Font style changes the rendered face | Pick Bold in the style field. | The layer re-renders in Bold and one undo restores the style. | `tst_character_paragraph_panels::fontStyleApplies` |
| Language is stored with the layer | Pick `French` and read the layer back. | The layer reports `French`; one undo restores the old value. | `tst_character_paragraph_panels::languageRoundTrips` |
| Tsume applies while the East Asian preference is on | Commit tsume 50 with the preference on. | Character-side spacing changes; one undo restores it. | `tst_character_paragraph_panels::tsumeFollowsAsianPreference` |
| Fractional widths off rounds advances | Uncheck Fractional Widths. | Advances snap to whole pixels; one undo restores fractional spacing. | `tst_character_paragraph_panels::fractionalWidthsMenu` |
| Panel reflects the active type layer | Select a type layer with Middle Eastern on. | Paragraph direction and the other attributes show the layer's values. | `tst_character_paragraph_panels::paragraphPanelReflectsAndEdits` |
| Editing a paragraph attribute updates the layer | Commit a first-line indent and undo once. | The layer reflects the change and one undo restores it. | `tst_character_paragraph_panels::paragraphPanelReflectsAndEdits` |
| Direction is stored with the layer | Pick RTL and read the layer back. | The layer reports RTL; one undo restores LTR. | `tst_character_paragraph_panels::rtlDirectionApplies` |
| A style button applies its toggle | Press Faux Bold. | The layer's Faux Bold is on, one state is recorded, and it re-renders. | `tst_character_paragraph_panels::styleButtonAppliesToggle` |
| Exclusive caps buttons | Turn All Caps on, then press Small Caps. | Small Caps is on and All Caps off. | `tst_character_paragraph_panels::capsButtonsAreExclusive` |
| Exclusive position buttons | Turn Superscript on, then press Subscript. | Subscript is on and Superscript off. | `tst_character_paragraph_panels::positionButtonsAreExclusive` |
| An OpenType button applies its feature | Press an enabled OpenType button. | The attribute flips and one state is recorded. | `tst_character_paragraph_panels::openTypeButtonAppliesFeature` |
| The style field lists the family's faces | Choose a family with four faces. | The style field offers them and the current face. | `tst_character_paragraph_panels::styleFieldListsFamilyFaces` |
| The Fractional Widths item follows the layer | Select a layer with fractional widths off. | The menu item is unchecked. | `tst_character_paragraph_panels::fractionalWidthsMenu` |
| The dialog round-trips its values | Set spacing values and Auto Leading, then accept. | The layer reports them; one undo restores the old values. | `tst_character_paragraph_panels::justificationDialogRoundTrips` |
| Cancelling writes nothing | Change a value and cancel. | Values are unchanged and no state is recorded. | `tst_character_paragraph_panels::justificationCancelWritesNothing` |
| RTL puts the first character at the right edge | Set an LTR paragraph layer to RTL. | The first character renders at the line's right edge; one undo restores LTR. | `tst_character_paragraph_panels::rtlDirectionApplies` |
| The control is absent without the engine | Turn Middle Eastern off with a type layer selected. | No direction control is visible. | `tst_character_paragraph_panels::directionFollowsMiddleEasternPreference` |

## ui/character-styles-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Panel lists the document styles | Show a document with `Heading` and `Caption`. | The panel lists them in order. | `tst_character_styles_panel::listsAndApplies` |
| Clicking a style applies it | Click a row with a type layer active, then undo once. | Attributes change and one undo restores them. | `tst_character_styles_panel::listsAndApplies` |
| No type layer records nothing | Click a row with no active type layer. | No history state is recorded. | `tst_character_styles_panel::noTypeLayerRecordsNothing` |
| An empty list is valid | Show a document with no character styles. | The list is empty and delete is disabled. | `tst_character_styles_panel::emptyListIsValid` |
| Creating a style adds it to the document | Create `Quote` with Faux Italic. | The list contains `Quote` and the style reports Faux Italic. | `tst_character_styles_panel::createsAndDeletes` |
| Deleting a style removes it | Delete a selected style. | It disappears and one undo restores it. | `tst_character_styles_panel::createsAndDeletes` |
| Editing a style updates every applying layer | Edit an applied style's size and accept. | The layer re-renders and one undo restores the old size. | `tst_character_styles_panel::editsAndReapplies` |
| Renaming an applied style keeps it applied | Rename an applied style and accept. | The layer still applies it under the new name and one undo restores both. | `tst_character_styles_panel::renameKeepsApplied` |
| The dialog offers the character pages | Open an existing style for editing. | Three character pages and Preview checked. | `tst_character_styles_panel::dialogPages` |
| Accepted options round-trip into the style | Check Faux Bold and Fractions, then accept. | The style reports both. | `tst_character_styles_panel::dialogRoundTrips` |
| Preview changes the style without history and Cancel restores it | Change a size with Preview, then cancel. | Preview changes the style, no state is recorded, and cancel restores the opening size. | `tst_character_styles_panel::dialogPreviewRestoresOnCancel` |
| Both menu paths toggle the panel | Check and uncheck each menu entry. | The panel shows/hides and each item follows visibility. | `tst_character_styles_panel::menusToggle` |
| character_styles_icon | Request the icon at 16 px and run the provenance guard. | The icon is non-null and its asset has provenance. | `tst_icon_assets::characterStylesGlyphResolves` |

## ui/type-preferences

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| type_pref_defaults | Open the Type page with no prior change. | Asian off, Middle Eastern off, glyph protection on. | `tst_type_preferences::defaults` |
| A change applies to open documents | Turn Middle Eastern on with a type layer open. | The direction control appears and no history state is recorded. | `tst_type_preferences::engineTogglesPanelVisibility` |
| The settings survive a restart | Change all three, save, then load the session. | Each reads back and the renderer reports the saved glyph-protection value. | `tst_type_preferences::persists` |
| Tsume appears with Show Asian Text Options | Turn Asian on with a type layer active. | Set Tsume shows the stored value. | `tst_type_preferences::tsumeAppearsWithAsianOptions` |
| Direction appears with Middle Eastern | Turn Middle Eastern on. | The LTR/RTL control shows the stored value. | `tst_type_preferences::directionAppearsWithMiddleEastern` |
| Hiding keeps the stored value | Set tsume 50, turn Asian off and on. | The control returns showing 50. | `tst_type_preferences::hidingKeepsStoredValue` |
| Turning protection off removes the substitution | Turn glyph protection off on a layer with an uncovered character. | The layer re-renders without fallback and no state is recorded. | `tst_type_preferences::glyphProtectionTogglesFallback` |
