# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tout_path_bounds` | Create a work path from an engine-authored, single-run point-type layer; compare its outline with the rendered glyph ink | Ink bounds agree within one pixel; the path is the active Paths row; the type layer is unchanged | `tst_type_menu::createWorkPath`, `pictura-render::type_outline_paths_match_rendered_ink` |
| `tout_path_replaces` | Create two work paths in a row, then undo once | The second replaces the first; undo restores the first | `tst_type_menu::createWorkPathReplaces` |
| `tout_path_no_outlines` | Try Create Work Path on an outline-less face | Disabled; no history state | `tst_type_menu::outlineRefusals`, `pictura-render::type_outline_refuses_outline_less` |
| `tout_shape_replaces` | Convert a type layer to a shape | Shape kind, ink-matching outline, an existing vector mask replaced by the glyph outline, type fill colour, TySh gone, one state | `tst_type_menu::convertToShape`, `pictura-render::convert_type_to_shape_replaces_content` |
| `tout_shape_keeps` | Convert a styled, raster-masked type layer | Name, stack, blend, opacity, raster mask, and effects retained | `tst_type_menu::convertToShapeKeepsAppearance`, `pictura-render::convert_type_to_shape_keeps_effects` |
| `tout_shape_undo` | Convert then undo once | The type layer and its text return | `tst_type_menu::convertToShapeUndo` |
| `tout_shape_round_trip` | Convert, save, and reopen the PSD | It reopens as a shape with the same outline, fill, and effects | `tst_type_menu::convertToShapeRoundTrip`, `pictura-render::convert_type_to_shape_round_trip` |
| `tout_shape_no_outlines` | Try Convert to Shape on an outline-less face | Disabled; no history state | `tst_type_menu::outlineRefusals` |
| `tout_warped` | Convert a warped type layer to a work path | The path follows the warped glyphs | `tst_type_menu::warpedOutlines`, `pictura-render::type_outline_paths_warped` |
| `tout_onpath` | Convert type on a path to a work path | Glyphs sit on the path, rotated with the tangent | `tst_type_menu::onPathOutlines`, `pictura-render::type_outline_paths_on_path` |
| `tout_area` | Convert area type to a work path | The outline covers the laid-out glyphs without the area clip | `tst_type_menu::areaOutlines`, `pictura-render::type_outline_paths_area` |
| `tout_session_gate` | Open a Type session and inspect both outline commands | Both are disabled and record no state | `tst_type_menu::sessionGate` |
| `tout_layer_rows` | Open the Layers row menu and Type menu on the same type layer and a non-type row | Both enable for an outline face and dispatch the stable ids; the Type menu disables for non-type; the rows are absent for a non-type row | `tst_layers_row_menu::typeRows`, `tst_layers_row_menu::rowActionsCarryCommandIds`, `tst_type_menu::layersRowParity` |
| `tm_aa_check` | Open the Type menu on a Smooth, vertical type layer | Smooth and Vertical are checked | `tst_type_menu::antiAlias` |
| `tm_aa_apply` | Trigger Crisp, then undo | Anti-aliasing changes and restores with one state | `tst_type_menu::antiAlias` |
| `tm_orientation` | Trigger Vertical, then undo | Orientation changes and restores with one state | `tst_type_menu::orientation` |
| `tm_row_disabled` | Open the Type menu on a pixel layer | AA and orientation rows are disabled | `tst_type_menu::rowsDisabled` |
| `tm_rasterize` | Rasterize a type layer | Pixel layer, one Rasterize Type state, command then disabled | `tst_type_menu::rasterizeTypeLayer` |
| `tm_update_all` | Update all text layers in editable and non-editable documents | Reproducible layers re-render under one state; non-editable/no-type disables | `tst_type_menu::updateAllTextLayers` |
| `tm_update_all_runs` | Update uniform type and imported mixed-run type together | Uniform type re-renders with its descriptor unchanged; mixed-run type keeps its stored pixels and descriptor | `pictura-render::refreshing_type_preserves_descriptors_and_imported_mixed_runs` |
| `tm_language` | Open Language Options with East Asian on | East Asian checked, Default and Middle Eastern unchecked; the panel follows | `tst_type_menu::languageOptions` |
| `tm_language_default` | Turn both features on, trigger Default, relaunch | Both off, Default checked, no document state, persisted | `tst_type_menu::languageOptionsPersist` |
| `tm_font_preview` | Choose Huge, open a family picker | Sample is larger than at Small; the row is checked and persists | `tst_type_menu::fontPreviewSize` |
| `tm_font_preview_none` | Choose None, open a family picker | Family name only, no sample | `tst_type_menu::fontPreviewNone` |
| `tm_lorem_session` | Trigger Paste Lorem Ipsum during a type session | The block replaces the selection; no state until commit | `tst_type_menu::pasteLoremSession` |
| `tm_lorem_layer` | Trigger it on a selected type layer with existing text | The block is appended after the text; one Paste Lorem Ipsum state | `tst_type_menu::pasteLoremLayer` |
| `tm_lorem_disabled` | Trigger it with no session and no type layer | Disabled | `tst_type_menu::pasteLoremDisabled` |
| `tm_glyphs` | Toggle Type > Panels > Glyphs twice, with and without a type layer | Panel shows then hides; the check follows | `tst_type_menu::glyphsRow` |
| `tm_opentype` | Toggle Fractions, then undo | Check follows the layer; one state; undo restores | `tst_type_menu::openTypeRow` |
| `tm_opentype_cross_surface` | Toggle Fractions from the Type menu, then inspect the Character panel menu | Both surfaces reflect the same flag | `tst_type_menu::openTypeCrossSurface` |
| `tm_default_rows` | Open the menu before and after saving defaults, including a non-editable document | Save enabled on type; Load disabled until saved or when non-editable | `tst_type_menu::defaultStyleRows` |
| `tm_session_gate` | Open the Type menu during a live Type session | AA, orientation, OpenType, Rasterize, Update All, and Load Defaults are disabled; Paste Lorem stays enabled | `tst_type_menu::sessionGate` |
| `tm_removed_rows` | Inspect the catalogue and Type menu | No Type Check Spelling, Find And Replace, or Extrude to 3D | `tst_type_menu::removedRows` |
| `fr_dialog` | Open Find And Replace on a type document | Fields, options, Replace All, Close; disabled with no type | `tst_find_replace::dialogFields` |
| `fr_not_editable` | Open it on a non-editable document or a non-type current layer with scope off | Replace All is disabled and records no state | `tst_find_replace::nonEditableDisabled` |
| `fr_session_gate` | Open a Type session and trigger Find And Replace | Disabled and records no state | `tst_find_replace::sessionGate` |
| `fr_modal_metadata` | Dispatch `edit.find.and.replace.text` through the control server | Refused as modal; the handler does not run | `tst_command_tree::modalMetadata` |
| `fr_current_layer` | Replace All with Search All Layers off | Only the current layer changes; one state | `tst_find_replace::currentLayerScope` |
| `fr_all_layers` | Replace All with Search All Layers on | Every not-fully-locked type layer, including groups, changes under one state | `tst_find_replace::allLayersScope` |
| `fr_no_match` | Replace All with no match, then Cancel | Document and history unchanged | `tst_find_replace::noMatchRecordsNothing`, `pictura-core::replace_matches_no_match` |
| `fr_locked` | Include a fully locked type layer | It is skipped; others change | `tst_find_replace::lockedLayersSkipped` |
| `fr_empty_find` | Leave Find What empty | Replace All is disabled and records no state | `tst_find_replace::emptyFindDisabled`, `pictura-core::replace_matches_empty_find` |
| `fr_blank_result` | Replace all of a layer's text with an empty string | The layer stays an empty type layer; one state | `tst_find_replace::blankResultKeepsType` |
| `fr_case` | Replace `cat` with and without Case Sensitive | Only `cat` or both `cat` and `Cat` change | `tst_find_replace::caseSensitive`, `pictura-core::replace_matches_case` |
| `fr_whole_word` | Replace `cat` with Whole Word Only | `catalog` is left alone | `tst_find_replace::wholeWordOnly`, `pictura-core::replace_matches_whole_word` |
| `fr_forward` | Replace `aa` with `X` in `aaa`, Forward on and off | Forward on gives `Xa`, off gives `aX`; no rescan | `tst_find_replace::forwardOverlap`, `pictura-core::replace_matches_forward` |
| `fr_round_trip` | Replace, save, reopen | Still a type layer with the new text and font | `tst_find_replace::roundTripsAsType` |
| `tds_save` | Save defaults from a styled layer | No document state; the layer is unchanged | `tst_type_menu::saveLoadDefaults` |
| `tds_save_disabled` | Open the menu on a pixel layer | Save Default Type Styles is disabled | `tst_type_menu::defaultsDisabled` |
| `tds_load` | Load defaults onto another type layer | Attributes change; text and placement stay; one state | `tst_type_menu::saveLoadDefaults` |
| `tds_load_undo` | Load then undo | Attributes restore; text unchanged | `tst_type_menu::saveLoadDefaults` |
| `tds_no_defaults` | Open the menu before saving defaults | Load is disabled | `tst_type_menu::defaultsDisabled` |
| `tds_not_editable` | Open the menu in a non-editable document or during a Type session | Load is disabled and records no state | `tst_type_menu::defaultsDisabled` |
| `tds_persist` | Save defaults, relaunch, load | Same font and attributes as before the restart | `tst_type_menu::defaultsPersist`, `tst_preferences_store::mixedLegacyProfileSplits` |
| `cr_type_rasterize` | Open the Layer > Rasterize submenu on a type layer | Type is enabled and rasterizes | `tst_layer_content::rasterizeKeepsAppearanceAndHistory` |
| A registered management command dispatches | Trigger a layer-management command | Its handler runs | `tst_command_tree::layerMenuGroupsAndDelete` |
| Inapplicable commands are disabled on open | Disable a management command's precondition and reopen the menu | The row is greyed, not hidden | `tst_command_tree::layerMaskMenu` |
| A kind-less rasterize variant is present but disabled | Build the Rasterize submenu | Shape, Vector Mask, Smart Object, Video, and 3D are disabled | `tst_layer_content::rasterizeKeepsAppearanceAndHistory` |
| A type row offers its CS6 text controls | Open the Layers row menu for a type layer with and without outline data | The text controls and outline rows enable/disable as specified; Windows AA stays disabled | `tst_layers_row_menu::typeRows`, `tst_type_menu::layersRowParity` |
| A fill row offers its mask and merge rows | Open the row menu for a fill layer | The mask, merge, export, and rasterize rows are present; Convert to Smart Object and Copy CSS are disabled | `tst_layers_row_menu::fillRows` |
| A group row offers export and Blending Options | Open the row menu for a group | Export, vector mask, merge, and Blending Options are present; Convert to Smart Object is disabled | `tst_layers_row_menu::groupRows` |
| Per-kind Duplicate and Delete labels | Open the row menu for a pixel row, a group, and a multi-selection | The labels read Duplicate Layer/Delete Layer, Duplicate Group/Delete Group, and Duplicate Layers/Delete Layers | `tst_layers_row_menu::rowBlocksInOrder` |
| `ffp_ui_font_names` | Open a family popup on the lightest installed face | Its name reaches full contrast | manual: visual contrast of a light face in the open popup |
| `ffp_preview_size` | Set Preview Size to Huge, open a family popup | The sample grows and the row grows with it | `tst_type_menu::fontPreviewSize` |
| `ffp_preview_none` | Set Preview Size to None, open a family popup | Only the family name is drawn | `tst_type_menu::fontPreviewNone` |
