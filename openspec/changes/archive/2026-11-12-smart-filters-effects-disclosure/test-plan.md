# Test plan — smart-filters-effects-disclosure

One row per delta scenario. Coverage names the automated test (`tst_<suite>::<case>` or `<crate>::<fn>`); no manual-only rows.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpr_one_list_toggle` | Show a layer with both effects and smart filters; open and close the toggle. | Exactly one right-edge toggle; opening shows both groups; closing hides both. | `tst_layers_smart_filters::oneToggleControlsBothLists` |
| `lpr_fx_list_toggle` | Show a styled layer; inspect the layer row and the Effects header; close and reopen the list. | The single right-edge toggle sits beside fx, its glyph flips between down (closed) and up (open), and the Effects header has no disclosure. | `tst_layers_effects::effectsListToggleSitsOnTheLayerRow` |
| `lpr_sf_list_toggle` | Show a smart object with a smart filter; inspect the row and the Smart Filters header; close and reopen the list. | The single right-edge toggle sits on the layer row, its glyph flips between down (closed) and up (open), and the header has no disclosure. | `tst_layers_smart_filters::smartFiltersListToggleSitsOnTheLayerRow` |
| `lpr_list_closed` | Close the effects and smart-filters list toggles. | The group header and every list row disappear; the layer row remains. | `tst_layers_effects::closingTheListHidesTheGroup`, `tst_layers_smart_filters::closingTheListHidesTheGroup` |
| `lpr_nested_cell` | Open an effects or smart-filters list; inspect the block borders, rules, and row height. | The left cell border and right cell run through the block; no rule between list rows or between the Effects and Smart Filters groups; each list row keeps its minimum vertical padding; the block bottom rule closes it. | `tst_layers_effects::nestedRowsFormOneCell`, `tst_layers_smart_filters::nestedRowsFormOneCell`, `tst_layers_smart_filters::noRuleBetweenGroups` |
| `lpr_synthetic_no_thumb` | Inspect a synthetic Effects/Smart Filters/effect/filter row. | No fx thumbnail card and no thumbnail outline; the expected thumbnail slot is plain row surface. | `tst_layers_effects::syntheticRowsHaveNoThumbnail`, `tst_layers_smart_filters::syntheticRowsHaveNoThumbnail` |
| `lpr_nested_eye` | Open a synthetic list on a masked layer; compare the header and child eye x and the name gap. | The header eye starts at the owning layer's image-thumbnail right edge (not after the mask); each child row sits one indent step deeper; the name gap is half the previous spacing. | `tst_layers_effects::nestedEyeAlignsWithImageThumb`, `tst_layers_smart_filters::nestedEyeAlignsWithImageThumb` |
| `lpr_nested_hidden` | Hide an effect/filter row and click its eye slot twice. | The eye-off glyph is painted at 50% opacity; the second click toggles the row back on. | `tst_layers_effects::hiddenEffectShowsFadedEye`, `tst_layers_smart_filters::hiddenFilterShowsFadedEye` |
| `lpr_synthetic_click` | Select another layer, then click an Effects/Smart Filters header and a child row outside the eye. | The owning layer becomes current and selected and the document target moves to it; the synthetic row is not highlighted. | `tst_layers_effects::clickingEffectRowSelectsLayer`, `tst_layers_smart_filters::clickingFilterRowSelectsLayer` |
| `lpr_group_chevron_kept` | Show a group row with a child and a styled layer row. | The group row keeps the left nested-layers chevron; the styled row draws the right-edge toggle. | `tst_layers_effects::groupKeepsNestedLayersChevron` |
| `lpr_eye_gutter` | Show a layer row; inspect the eye rect padding. | The eye glyph is centred in the fixed gutter with equal left and right padding. | `tst_layers_panel::eyeGutterIsCentredAndSeparated` |
| `lpr_eye_gutter_flat` | Show a layer row; sample several points in the visibility cell. | The cell is a flat fill one shade lighter than the row surface, with no gradient. | `tst_panels_parity::eyeGutterIsFlatAndLighter` |
| `lpr_eye_sep` | Show a layer row; inspect the gutter right edge. | A 1 px darker separator sits at the gutter edge. | `tst_layers_panel::eyeGutterIsCentredAndSeparated` |
| `lpr_selection_focus` | Focus and unfocus the tree with a selected row; sample the row interior and border. | The selected row keeps the same flat grey fill and border in both states, with no focus tint or frame. | `tst_panels_parity::selectedRowUsesFlatHighlight` |
| `lpr_badge_sizes` | Show a layer with a lock, fx badge, and clipping glyph. | The lock badge is 80% of its old two-thirds-thumbnail side; the fx badge and clipping glyph are 20% smaller, with a 4 px clip gap. | `tst_panels_parity::lockBadgeShrunk`, `tst_panels_parity::badgeSizesShrunk` |
| `lpr_shape_effects_toggle` | Give a shape layer an effect. | The fx badge stays hidden but the right-edge list toggle still opens and closes the Effects list. | `tst_layers_effects::shapeLayerWithEffectsShowsTheListToggle` |
| `lpr_synthetic_not_selectable` | Inspect the flags of an Effects header/row. | The synthetic rows are not selectable, so keyboard selection resolves to the owning layer. | `tst_layers_effects::syntheticRowsAreNotSelectable` |
| `lpr_synthetic_drag_refused` | Start a drag on an Effects header/row. | No drag starts; the owning layer is not moved. | `tst_layers_effects::syntheticRowsDoNotDrag` |
| `lp_sf_drag_preserved` | Start a drag on a filter child and on the Smart Filters header. | The child still carries its filter path; the header carries no drag. | `tst_layers_smart_filters::filterRowStillDrags` |
| `lpr_thumb_gap` | Show a regular layer and a group at the same depth. | The regular thumbnail starts after the gutter; the group reserves the chevron slot. | `tst_layers_panel::layerRowSurface` |
| `The parent and one child per filter appear` | Show a smart object with filters. | A Smart Filters parent and one child per filter appear. | `tst_layers_smart_filters::treeShowsGroupAndToggles` |
| `lp_sf_order` | Apply Gaussian Blur then Emboss. | The Emboss row is above the Gaussian Blur row. | `tst_smart_filter_apply::filterRowsShowNewestFirst` |
| `Toggling a filter eye updates the composite` | Flip a filter child eye. | The filter flag flips, the composite updates, and one history state is recorded. | `pictura-app::app_setter_changes_the_composite_and_records_one_state` |
| `Toggling the group eye bypasses the chain` | Flip the group eye off. | The group flag clears and the composite falls back to the unfiltered source. | `pictura-app::disabled_group_bypasses_the_chain` |
| `A toggle survives a write and re-read` | Toggle a filter/group flag, write, and read back. | The re-read smart object reports the toggled flag. | `pictura-app::toggles_persist_on_a_preserved_smart_object` |
| `Synthetic rows are not real layers` | Double-click a smart-filter row. | No rename or layer operation; the filter dialog opens. | `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` |
| `lp_sf_drag` | Drag a filter row above another; undo; drop on a layer row. | The order moves and applies last; undo restores it; the layer drop is refused. | `tst_smart_filter_apply::dragReordersSmartFilters` |
| `lpr_row_surface` | Show a layer row against the list background. | The row surface is one shade lighter than the list. | `tst_layers_panel::layerRowSurface` |
| `lpr_eye_no_select` | Click the visibility column of a layer row. | Visibility toggles; the row is neither selected nor highlighted. | `tst_layers_panel::gutterClickTogglesWithoutSelecting` |
| `lpr_selection_grey` | Select one or more layer rows. | The selection is a neutral grey, not the palette blue. | `tst_layers_panel::layerRowSurface` |
| `lpr_content_padding` | Show a layer row. | The content column starts 2 px after the gutter; the separator stays 1 px. | `tst_layers_panel::layerRowSurface` |
| `lpr_eye` | Show a visible row, a hidden row, and a regular nested layer. | The visible row uses the eye asset; a hidden row paints the eye-off glyph at 50% opacity; a regular nested layer keeps the fixed eye x. | `tst_layers_panel::layerRowSurface`, `tst_panels_parity::hiddenLayerShowsFadedEye`, `tst_layers_panel::panelChrome` |
| `lpr_label_tint` | Show a layer with a color label. | The whole visibility column is filled with the label color behind the eye. | `tst_layers_panel_fixes::labelGlyphTintFollowsFill` |
| `lpr_label_tint_none` | Show a layer with label None. | The normal gutter surface is used with no tint. | `tst_layers_panel::layerSurfaceCompleteness` |
| `lpr_label_glyph` | Show rows with each color label. | The eye tint stays legible on every fill. | `tst_layers_panel_fixes::labelGlyphTintFollowsFill` |
| `lpr_smart_badge` | Show an embedded/placed smart object. | The smart-object badge is drawn on the thumbnail lower-right corner. | `tst_panels_parity::kindBadgesHidpi` |
| `lpr_smart_badge_absent` | Show an ordinary pixel layer. | No smart-object badge is drawn. | `tst_panels_parity::kindBadgesHidpi` |
| `lpr_chevron` | Collapse and expand a group with children. | The right/down chevron flips and clicking it toggles the group. | `tst_panels_parity::emptyGroupShowsChevron` |
| `m39_badges` | Show a masked, clipped, locked, expandable, or missing-badge row. | The matching badge/control is drawn, and a missing asset omits only that badge. | `tst_layers_panel::maskRowIndicators`, `tst_panels_parity::emptyGroupShowsChevron` |
| `lpc_clip` | Show a clipped layer. | The clipping indicator, indentation, and base underline are drawn. | `tst_layers_panel::clippingMasks` |
| `lpc_lockbadge` | Show a locked layer and an unlocked layer. | The locked row draws the right-side lock badge; the unlocked row draws none. | `tst_panels_parity::lockBadgeShrunk` |
| `lpr_lock_size` | Show a locked layer. | The lock badge side is 80% of the old two-thirds-thumbnail side and smaller than the thumbnail. | `tst_panels_parity::lockBadgeShrunk` |
| `lpc_forced_lock_badge` | Commit a type layer and lock position. | Forced Transparency/Image locks draw no badge; a Position lock shows it. | `tst_type_tools::typeLayerRowHasNoLockBadge` |
| `lpr_thumb_checker` | Show a regular translucent layer. | The thumbnail draws over the two-tone checkerboard. | `tst_layers_panel_fixes::thumbnailFillsRow` |
| `lpr_thumb_bracket` | Show the singular active layer. | White corner brackets sit one pixel outside the thumbnail outline. | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| `lpr_group_no_outline` | Show a group row. | The folder glyph carries no black outline. | `tst_layers_panel_fixes::groupIconHasNoOutline` |
| `lpr_type_thumb_card` | Show a type layer in a non-square document. | A square white T card with no outline; the hit rect is the card. | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` |
| `lpr_thumb_fill` | Show a portrait document layer. | The letterboxed thumbnail is taller than wide and fills the row height. | `tst_layers_panel_fixes::thumbnailFillsRow` |
| `lpr_row_fonts` | Inspect the Background, a linked layer, and a placed smart object. | Background italic; linked/placed underlined; ordinary names normal. | `tst_layers_panel::layerSurfaceCompleteness` |
| `lpr_row_height` | Read a row size hint. | The height is at least 28 px from the named constant. | `tst_layers_panel_fixes::thumbnailFillsRow` |
| `A styled layer shows the fx badge` | Show a styled and an unstyled layer. | The styled row draws fx at the right edge; the unstyled row draws none. | `tst_layers_panel::layerStylePanelIntegration` |
| `lpr_fx_absent_adjustment` | Show an adjustment/fill layer without effects, then with one. | No fx badge or reserved space until an effect exists. | `tst_layers_effects::adjustmentRowsShowNoFxBadge` |
| `lpr_blendif_badge` | Show a layer with customised Blend If. | A Blend If chip is drawn left of the fx/lock run and the name elides before it. | `tst_layers_panel::blendIfBadge` |
| `lpr_blendif_badge_absent` | Show a layer with default Blend If. | No chip is drawn; the name keeps the full right-edge run. | `tst_layers_panel::blendIfBadge` |
| `lpr_blendif_raw` | Show a layer with an unparsed blending_ranges block. | The Blend If chip is still drawn. | `tst_layers_panel::blendIfBadge` |
| `lpr_thumb_icon_no_outline` | Show an adjustment or type row. | The icon card carries no black outline; a raster row keeps its outline. | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline`, `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline` |
| `lpr_fx_rows` | Show a pixel layer with a drop shadow. | The Effects header and Drop Shadow row follow the layer. | `tst_layers_effects::nestedEffectRows` |
| `lpr_fx_group_eye` | Click the Effects header eye. | fx.visible flips, the composite unstyles, and the row reads hidden. | `tst_layers_effects::effectsEyeTogglesMaster` |
| `lpr_fx_effect_eye` | Click a Drop Shadow row eye. | dropShadow.on flips and the composite changes; the row stays listed. | `tst_layers_effects::effectEyeTogglesOne` |
| `lpr_fx_expand_new` | Add a style with Expand New Effects on. | The Effects group and its rows start expanded. | `tst_layers_effects::newStyleStartsExpanded` |
| `lpr_fx_no_group` | Show an unstyled layer. | No Effects row follows it. | `tst_layers_effects::unstyledLayerHasNoGroup` |
