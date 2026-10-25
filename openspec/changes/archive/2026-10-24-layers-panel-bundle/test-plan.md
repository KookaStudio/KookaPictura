# Test plan

Every scenario in the delta specs has a row. Rows name the automated test that
covers them; the new suites are written in task 1 before the behavior lands.

## Delta scenarios

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Add a layer | Create a layer with the middle layer active | Inserted directly above, selected, one more row | `pictura-render::add_layer_inserts_above_and_is_transparent` |
| Add a layer inside a selected group [m39_tree] | Select a group and create a layer | Inserted as the group's topmost child and selected | `pictura-render::add_layer_in_inside_group_and_on_top` |
| Add a group | Create a group with no selection | Inserted at the top and selected | `tst_layers_panel_bundle::groupInsertsAtTopWithoutSelection` |
| Duplicate a layer | Duplicate a middle layer | Copy above it, named `<name> copy`, selected | `tst_layers_panel_bundle::duplicateInsertsAboveAndSelects` |
| Delete a layer | Delete a selected layer | Row removed, one state | `tst_layers_panel_bundle::deleteRemovesTheLayer` |
| Move a layer | Move a layer up and down | Order swaps within the container | `tst_layers_panel_fixes::legacyMoveLayerShiftsAndRefuses` |
| Move Down at the Background floor is refused | Move the layer above the Background down | No change, no state | `tst_layers_panel_fixes::moveDownStopsAtBackground` |
| Duplicating the Background makes an ordinary layer | Duplicate the Background | Copy is unlocked, unflagged, with alpha | `tst_layers_panel_bundle::duplicateBackgroundMakesOrdinaryLayer` |
| `lpo_anchor_adjustment` | Select a middle layer in a group, create Invert | Inserted above it inside the group, selected | `tst_layers_panel_bundle::adjustmentInsertsAboveActive` |
| `lpo_anchor_fill` | Select a middle layer, create Solid Color | Inserted above it, selected | `tst_layers_panel_bundle::fillInsertsAboveActive` |
| Defaults on a fresh install [m39_options] | Read the panel options with no saved session | Medium / Entire Document / three toggles on | `tst_layers_panel::panelOptionsFlags` |
| Thumbnail contents changes the thumbnail [m39_options] | Show a small layer with both contents modes | Layer Bounds fills, Entire Document shows the layer small | `tst_layers_snapshot::entireDocumentPlacement` |
| Options persist [m39_options] | Change options and reload the session | Values restored | `tst_layers_panel::panelOptionsFlags` |
| `lpo_copy_name` | Duplicate `Base` with Add "copy" on | Copy is `Base copy` | `tst_layers_panel_fixes::duplicateGroupNamesStayDistinct` |
| `lpo_copy_name_off` | Duplicate `Base` with Add "copy" off | Copy keeps `Base` | `tst_layers_panel_bundle::copyToggleOffKeepsName` |
| `lpo_default_mask` | Use Default Masks on, selection active, create a fill | Fill mask follows the selection | `tst_layers_panel_bundle::fillMaskFromSelection` |
| `lpo_default_mask_white` | Use Default Masks on, no selection, create a fill | White document-sized mask | `tst_layers_panel_bundle::fillMaskWhiteWithoutSelection` |
| `lpo_default_mask_off` | Use Default Masks off, selection active, create a fill | No mask | `tst_layers_panel_bundle::fillMaskOff` |
| The panel menu contains the wired commands [m39_menus] | Open the panel menu | Panel Options and the layer commands present | `tst_command_tree::panelMenusToolsIcons` |
| The row menu sets a color label [m39_menus] | Pick Red from the Color Label submenu | Row label becomes Red | `tst_layers_panel::layerSurfaceCompleteness` |
| Move Up/Down reorders from the menu [m39_menus] | Pick Move Layer Up | Layer swaps in one state | `tst_command_tree::menuMoves` |
| The action strip has no Move buttons [m39_strip] | Inspect the strip | Seven CS6 buttons, no Move | `tst_layers_panel_fixes::actionStripStaysFixedAndCentred` |
| Pixel and smart-object rows offer export | Open the menu for pixel and smart rows | Quick Export then Export As | `tst_layers_row_menu::pixelAndSmartRowsOfferExport` |
| Group and adjustment rows hide export | Open the menu for group and adjustment rows | No export rows | `tst_layers_row_menu::groupAndAdjustmentHideExport` |
| Type rows hide export | Open the menu for a type row | No export rows | `tst_layers_row_menu::typeHidesExport` |
| A shape row offers its shape commands | Open the shape row menu | Copy/Paste Shape Attributes and Rasterize Shape | `tst_command_tree::shapeLayerCommands` |
| A shape row is not offered the adjustment edit | Open the shape row menu | No Edit Adjustment… | `tst_layers_row_menu::shapeOmitsEditAdjustment` |
| `lpr_convert_row` | Open a convertible pixel row menu, pick Convert | Converts and selects the smart object | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| An existing smart object disables Convert to Smart Object | Open a smart-object row menu | Convert disabled | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `lpr_smart_rows` | Open a smart-object row menu | The six smart rows are enabled | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `lpr_smart_linked` | Open a smart-object row menu | Relink and Convert to Linked disabled with tooltip | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| A plain pixel row omits the smart-object commands | Open a plain pixel row menu | No smart-object-only rows | `tst_layers_row_menu::plainPixelOmitsSmartRows` |
| A pixel row offers its full command set | Open a pixel row menu | Common, export, mask, style, clipping, smart, rasterize rows | `tst_layers_row_menu::pixelRowFullSet` |
| `lmk_rowmenu` | Add then delete a mask from the row menu | Both apply in one state each | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `lmk_rowmenu_enable` | Toggle mask enablement from the row menu | Disabled bit flips | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| A group row omits content commands | Open a group row menu | No export/fill; mask rows and Ungroup present | `tst_layers_row_menu::groupRows` |
| A type row offers rasterize | Open a type row menu | Rasterize Type present and enabled | `tst_layers_row_menu::rasterizeRows` |
| An unimplemented applicable command is disabled, not absent | Open a styled row menu | Rasterize Layer Style disabled with tooltip | `tst_layers_row_menu::unimplementedDisabled` |
| `lpr_row_blocks` | Open a pixel row menu | Blocks in the specified order with one separator between | `tst_layers_row_menu::rowBlocksInOrder` |
| `lpr_background_brief` | Open the Background row menu | Only the six brief rows; no Rename/style/label/merge | `tst_layers_row_menu::backgroundBriefMenu` |
| `lpr_group_rows` | Open a group row menu | Mask, link, and merge rows; no export/fill | `tst_layers_row_menu::groupRows` |
| `lpr_pixel_mask_rows` | Open a pixel row menu that can clip | Create Clipping Mask and four vector-mask rows; a clipped row shows Release | `tst_layers_row_menu::pixelMaskAndClippingRows` |
| `lpr_link_rows` | Open an unlinked row menu, link it, reopen | Link then Unlink; Select Linked Layers present | `tst_layers_row_menu::linkRows` |
| `lpr_merge_rows` | Open a single row and a multi-selection menu | Merge Down vs Merge Layers; visible/clipping/flatten | `tst_layers_row_menu::mergeRows` |
| `lpr_rasterize_rows` | Open type, shape, fill, smart, and plain pixel menus | Rasterize Type/Shape/Layer labels; pixel hides; styled has disabled style row | `tst_layers_row_menu::rasterizeRows` |
| `lpr_isolation_row` | Enter isolation and open a row menu | Release from Isolation before the merge block | `tst_layers_row_menu::isolationRow` |
| `lpr_row_action_target` | Right-click an unselected row and run an action | The row is selected first and the action applies to it | manual: the context menu is a modal QMenu; verify in the running app |
| A double-click opens Blending Options | Double-click a pixel row outside the name | Layer Style opens | `tst_layers_panel::layerStylePanelIntegration` |
| A background double-click does not open a style | Double-click the Background outside the name | Conversion path, no style | `tst_fill_layer_editor::backgroundDoubleClickConverts` |
| The style rows report their enablement | Styled row with a copied style | All four style rows enabled | `tst_layers_panel::layerStylePanelIntegration` |
| An unstyled pixel row disables Copy and Clear | Unstyled pixel row | Blending enabled, Copy/Paste/Clear disabled | `tst_layers_panel::layerStylePanelIntegration` |
| An adjustment row omits the style rows | Adjustment row | No style rows | `tst_layers_panel::layerStylePanelIntegration` |
| The row menu clears a row's style | Clear a styled row | Style removed, fx badge gone | `tst_layers_panel::layerStylePanelIntegration` |
| Alt-clicking the fx region toggles all effects | Alt-click a styled row's fx | All effects hide, then show | `tst_layer_style::menuCommands` |
| The Effect dimension matches styled rows | Filter by an effect | Only rows carrying it | `tst_layer_style::menuCommands` |
| The Effect dimension lists the effect keys | Open the Effect choices | The ten effect keys | `tst_layer_style::menuCommands` |
| Mask thumbnail adjacency [uip_layers_mask_adjacency] | Show a masked and vector-masked row | Mask right of image, vector right of mask, chain between | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| Active thumbnail brackets [uip_layers_active_thumb] | Click a thumbnail | Brackets on it; never on a group icon | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| The chain glyph is larger and rotated [uip_layers_link_glyph] | Show linked and unlinked masks | Taller-than-wide rotated chain | `tst_layers_panel_fixes::linkGlyphRotatedAndLarger` |
| `lpr_thumb_adjustment_icon` | Show an Invert and a Levels row | Neutral card with the kind icon, no checkerboard | `tst_layers_snapshot::adjustmentThumbnailKindIcon` |
| `lpr_thumb_fill_preview` | Show Solid, Gradient, Pattern fill rows | Swatch, ramp, and scaled tile previews | `tst_layers_snapshot::fillThumbnailPreviews` |
| Reveal All creates a full mask | Reveal All on a pixel layer | 255 samples and default, composite unchanged | `pictura-render::add_creates_one_sized_to_the_layer_and_refuses_a_second` |
| Hide All creates an empty mask | Hide All on a pixel layer | 0 samples and default, layer hidden | `pictura-render::hide_all_zeroes_the_composite_and_delete_restores_it` |
| `lpr_mask_add_target` | Add a mask to the active layer | It becomes the edit target | `pictura-render::add_creates_one_sized_to_the_layer_and_refuses_a_second` |
| A second mask is refused | Add two masks | Second refused, no state | `pictura-render::add_creates_one_sized_to_the_layer_and_refuses_a_second` |
| An unknown path is refused | Add a mask by a bad path | Refused, document unchanged | `pictura-render::reads_report_absence_without_a_mask_or_path` |
| `lm_group_mask_document` | Reveal All on a group with children and an empty group | Document-sized mask, non-empty thumbnail, editable coverage | `pictura-render::add_layer_mask_group_is_document_sized` |
| Activating and clearing the target | Set and clear a mask target | Set succeeds, clear empties; a path without a mask refuses | `tst_layers_panel::maskEditTargetActivation` |
| A mask round-trips through the adapter | Edit a mask document and write it back | Coverage carries the edit, pixels untouched | `pictura-render::write_back_round_trips_and_limits_to_the_region` |
| A deleted mask empties the target | Delete the targeted mask | Target reads empty | `tst_layers_panel::maskEditTargetActivation` |
| `lm_group_mask_edit` | Target, edit, and write back a group mask | Group coverage and composite follow | `pictura-render::group_mask_round_trips_through_the_adapter` |
| Panel menu still lists the adjustment kinds | Open the Adjustments panel menu | The sixteen kinds, Add Mask by Default, Clip to Layer | `tst_command_tree::panelMenusToolsIcons` |
| `adj_add_mask_default_toggle` | Toggle Add Mask by Default | Check state follows the preference; creation honors it | `tst_layers_panel_bundle::addMaskByDefaultToggles` |
| `adj_mask_from_selection` | Create an adjustment over a selection | Selection mask, active target | `tst_layers_panel_bundle::adjustmentMaskFromSelection` |
| `adj_mask_white` | Create an adjustment with no selection | White document mask, active target, no solo | `tst_layers_panel_bundle::adjustmentMaskWhite` |
| `adj_mask_off` | Create an adjustment with the preference off | No mask; layer thumbnail active | `tst_layers_panel_bundle::adjustmentMaskOff` |
| A transparent document reveals both checker tones | Display a fully transparent document | Checker inside, canvas colour outside | `tst_canvas_backdrop::transparentDocumentShowsCheckerboard` |
| Opaque content covers the checkerboard | Display an opaque region | Document pixels, no checker | `tst_canvas_backdrop::opaqueContentCoversCheckerboard` |
| `app_shell_hidden_background` | Toggle a Background's eye off | Checkerboard replaces the opaque backdrop | `tst_canvas_backdrop::hiddenBackgroundShowsCheckerboard` |

## Tag keys

The checker keys a tagged scenario by its tag; these rows supply those keys.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `m39_tree` | Create a layer with a group selected | Inserted as the group's topmost child and selected | `pictura-render::add_layer_in_inside_group_and_on_top` |
| `m39_options` | Read and change the panel options | Defaults and persistence hold | `tst_layers_panel::panelOptionsFlags` |
| `m39_menus` | Open the panel and row menus | Wired commands and the color label work | `tst_command_tree::panelMenusToolsIcons` |
| `m39_strip` | Inspect the action strip | Seven CS6 buttons, no Move | `tst_layers_panel_fixes::actionStripStaysFixedAndCentred` |
| `uip_layers_mask_adjacency` | Show masked and vector-masked rows | Thumbnails and chain glyphs in order | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| `uip_layers_active_thumb` | Click a thumbnail | Brackets on the active thumbnail only | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| `uip_layers_link_glyph` | Show linked and unlinked masks | Rotated, larger chain glyph | `tst_layers_panel_fixes::linkGlyphRotatedAndLarger` |

## Regression guard

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Existing pixel/group creation | Create layers and groups with and without a selection | Insertion and selection match the pre-change behavior | `pictura-render::add_layer_in_inside_group_and_on_top` |
| Existing mask commands | Add, delete, enable, and link a pixel mask | Behavior and history unchanged | `tst_layers_mask_commands::maskMenuActions` |
| Existing thumbnail rendering | Render pixel, type, shape, smart, and group rows | Checkerboard, T card, shape, page badge, and folder glyph unchanged | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| Existing row menu commands | Run duplicate, delete, group, ungroup, and move from the row menu | Each still acts on the row and records one state | `tst_layers_panel::layerSurfaceCompleteness` |
