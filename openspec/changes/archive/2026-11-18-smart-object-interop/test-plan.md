# Test Plan

Use fixture files with synthetic artwork. Editable Illustrator clipboard support is a separate fixture-gated follow-up. The test names below include cases to add during apply.

## Vector placement

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `vector_place` | Place a multipage PDF and a PDF-compatible AI fixture into a smaller document, choosing page and crop | Selected artwork, initial fit, and transform flow are correct | `tst_vector_smart_object::placesSelectedPage` |
| `vector_place_refused` | Cancel the page picker and Place transform, discard a placement transform when switching tools, then try a damaged PDF | Nothing remains on the layer stack or in history, and the damaged input is reported | `tst_vector_smart_object::cancelledOrInvalidPlace`, `tst_vector_smart_object::discardPlacementOnToolSwitch` |
| `vector_place_scale` | Enlarge an object containing transparent vector detail, repeat after a rotation or warp, and undo | New pixels retain detail from the vector source at the committed size and undo restores the prior output | `tst_vector_smart_object::scaleRendersFromSource`, `tst_vector_smart_object::scaleAfterRotationRendersFromSource` |
| `vector_place_identity` | Save shared and independent copies with identical names and bytes as PSD and PSB, reopen, and edit one shared source | Shared copies update together and independent copies remain unchanged | `tst_vector_smart_object::sourceIdentityRoundTrips` |
| `vector_place_roundtrip` | Save transformed PDF and PDF-compatible AI placements as PSD and PSB, inspect placement metadata with an independent reader, then reopen and export | Page, crop, geometry, filename, format, and exported bytes survive | `tst_vector_smart_object::sourceRoundTrips`, `pictura-codec::authored_vector_placement_descriptor_oracle` |

## Edit Contents

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| A PSD/PSB source is editable | Query an embedded PSD and PSB | Eligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| A non-parsing source is refused | Query a JPEG or malformed embedded payload | Ineligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| A non-embedded object is refused | Query an external, alias, or unresolved source | Ineligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| A non-smart layer is refused | Query a normal pixel layer | Ineligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| A group or adjustment layer is refused | Query a group and an adjustment layer carrying a payload | Ineligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| An empty payload is refused | Query objects with no payload and an empty payload | Ineligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| An unresolved path is refused | Query a missing row path | Ineligible | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| The predicate mutates nothing | Compare the document before and after eligibility queries | Equal document state | `pictura-app::layer_can_edit_smart_object_contents_eligibility` |
| The command is registered with its stable id | Resolve the existing Edit Contents action through the command registry | Same id and menu registration | `tst_smart_object_actions::editContentsDispatchesBySourceKind` |
| Enabled only for an editable source | Select embedded PSD/PSB, supported PDF/AI, and ineligible layers in turn | Action state follows source eligibility | `tst_smart_object_actions::editContentsDispatchesBySourceKind` |
| One new untitled editor tab opens | Trigger Edit Contents on an embedded PSD from the row menu | One new untitled tab contains the source layers | `tst_smart_object_actions::editContentsDispatchesBySourceKind` |
| Opening records no state on the origin | Compare the origin before and after opening an internal editor | Same document and history | `tst_smart_object_actions::editContentsDispatchesBySourceKind` |
| An ineligible target adds no tab | Dispatch Edit Contents with an ineligible target | No tab, origin edit, or history entry | `tst_smart_object_actions::editContentsDispatchesBySourceKind` |
| `vector_edit_launch` | Intercept the desktop launch for PDF and AI | Temporary file matches the original bytes and suffix, Apply and Discard available, no tab or history | `tst_vector_smart_object::launchesExternalEditor` |
| `vector_edit_handler` | Have the launch test handler refuse the URL | Error surfaced, temp file deleted, history unchanged | `tst_vector_smart_object::launchesExternalEditor` |
| `vector_edit_apply` | Duplicate an object both ways, reorder the layers, save changed artwork to the session file, Apply, undo, and redo | Only shared instances change, placement and masks survive, and bytes and pixels round-trip through history | `tst_vector_smart_object::appliesToSharedInstances` |
| `vector_edit_refused` | Apply unchanged, malformed, and page-missing files, then try after replacing or deleting the origin | No origin change or extra state, with a visible error for invalid or stale sessions | `tst_vector_smart_object::refusesInvalidOrStaleEdits` |
| `vector_edit_discard` | Edit the temporary file, then discard or close the origin before or during Apply | No commit and temporary files removed | `tst_vector_smart_object::discardsExternalSession`, `tst_vector_smart_object::discardDuringApply` |

## Paste As

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `vector_paste_pdf` | Trigger actual Edit > Paste with PDF-only MIME data and empty internal clipboard and system image | Action and Smart Object and Pixels enabled; other paste command gates unchanged | `tst_vector_paste::offersSupportedDestinations` |
| `vector_paste_aicb` | Paste PDF MIME with unverified Illustrator data | Smart Object and Pixels enabled; Paths and Shape Layer disabled | `tst_vector_paste::offersSupportedDestinations` |
| `vector_paste_raster` | Paste a foreign raster image, then an internal positioned clip | Ordinary paste creates layers with the existing position rules and skips the dialog | `tst_vector_paste::regularPasteIsUnchanged` |
| `vector_paste_layers` | Paste PDF as Smart Object and as Pixels in separate runs | Expected layer type, original embedded bytes or transparent pixels, one undo step | `tst_vector_paste::createsLayerDestinations` |
| `vector_paste_refused` | Seed an internal clip, then cancel vector paste and force a conversion error | No document or history change, no stale paste, and a visible error on conversion failure | `tst_vector_paste::cancelledOrInvalidPaste` |

## Imported link repair

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `external_link_open` | Open a linked fixture with the original asset absent | Stored preview renders and repair is enabled | `tst_smart_object_relink::opensWithStoredPreview` |
| `external_link_apply` | Relink shared sources after a local transform and with an enabled masked Smart Filter, then undo and redo | One history state updates matching previews and the record while preserving placement geometry and visible filter output | `tst_smart_object_relink::repairsSharedSource`, `tst_smart_object_relink::repairPreservesLocalTransform`, `tst_smart_object_relink::repairPreservesSmartFilters` |
| `external_link_refused` | Cancel, choose an invalid file, and change the target before a staged repair returns | Document content, source path, and history remain as before the repair | `tst_smart_object_relink::refusesInvalidOrStaleRepair` |
| `external_link_incompatible` | Select different format-family and dimensions, and invalid page/crop replacements | Error with record, previews, and history unchanged | `tst_smart_object_relink::refusesIncompatibleRepair` |
| `external_link_reopen` | Save repaired PSD and PSB documents, remove the selected fixture copy, then reopen | Updated preview and external record survive and repair remains available | `tst_smart_object_relink::repairedLinkSurvivesReopen` |
| `external_record_write` | Rewrite one of multiple records in PSD and PSB, reopen, and inspect the records and referencing descriptors | Selected path, filename, file type, size, and present timestamps update; understood cached data is refreshed, UUID, placed descriptors, and other data survive; an independent reader confirms the record | `pictura-codec::repaired_external_record_roundtrips_selectively` |
| `external_record_opaque` | Load variants with unrecognized metadata or cached data and with missing or inconsistent placed dimensions, then save without repair | Ineligible and byte-identical record | `pictura-codec::unsupported_external_record_stays_opaque` |

## Layers panel

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `external_link_row` | Open the menu on an eligible external row while another row is active, then dispatch Relink | The command is enabled and targets the clicked record | `tst_smart_object_relink::rowActionTargetsClickedSource` |
| `m39_menus` | Open the panel menu | The wired commands are present and unimplemented ones are disabled with the tooltip | `tst_command_tree::menusPanel`, `tst_layers_panel_fixes::legacyMoveLayerShiftsAndRefuses` |
| `m39_menus` | Pick Red from the row color-label submenu | The layer's color label becomes Red | `tst_command_tree::menusPanel` |
| `m39_menus` | Pick Move Layer Up for a non-top row | The layer swaps in one undo step | `tst_layers_panel_fixes::legacyMoveLayerShiftsAndRefuses` |
| `m39_strip` | Show the Layers panel | Exactly seven strip buttons, no Move buttons | `tst_layers_panel_fixes::actionStripStaysFixedAndCentred` |
| `Pixel and smart-object rows offer export` | Open the row menu for a pixel and a smart-object row | Export As… and Quick Export, Quick Export first | `tst_layers_row_menu::pixelAndSmartRowsOfferExport` |
| `Group and adjustment rows offer export` | Open the row menu for a group and an adjustment row | Quick Export and Export As… | `tst_layers_row_menu::groupAndAdjustmentOfferExport` |
| `Type rows offer export` | Open the row menu for a type row | Quick Export and Export As… | `tst_layers_row_menu::typeRowsOfferExport` |
| `A shape row offers its shape commands` | Open the row menu for a shape row | Copy/Paste Shape Attributes and Rasterize Shape enabled | `tst_layers_panel::shapeRowActions` |
| `A shape row is not offered the adjustment edit` | Open the row menu for a shape row | No Edit Adjustment… row | `tst_layers_row_menu::shapeOmitsEditAdjustment` |
| `lpr_convert_row` | Pick Convert to Smart Object on a pixel row | The layer converts and the smart object is selected | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `An existing smart object disables Convert to Smart Object` | Open the row menu on a smart object | Convert to Smart Object is disabled | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `lpr_smart_rows` | Open the row menu on an embedded PSD/PSB Smart Object | Its six Smart Object commands are enabled | `tst_smart_object_actions::rowMenuOffersSmartRows`, `tst_smart_object_actions::rowMenuOffersSmartObjectContents` |
| `lpr_smart_linked` | Open the row menu on an embedded Smart Object | Relink and Convert to Linked are disabled, with the placeholder tooltip for Convert to Linked | `tst_smart_object_actions::rowMenuOffersSmartObjectContents` |
| `A plain pixel row omits the smart-object commands` | Open the row menu on a plain pixel row | No smart-object rows | `tst_layers_row_menu::plainPixelOmitsSmartRows` |
| `A pixel row offers its full command set` | Open the row menu on a pixel row | Common, export, mask, style, clipping, smart-object, rasterize rows | `tst_layers_row_menu::pixelRowFullSet` |
| `lmk_rowmenu` | Pick Add Layer Mask > Reveal All, then Delete Layer Mask | A mask is added then removed, one state each | `tst_layers_mask_commands::addMaskSubmenu` |
| `lpr_add_mask_submenu` | Pick Reveal All, then Reveal Selection with a selection | Both masks added, one state each; Reveal Selection disabled without a selection | `tst_layers_mask_commands::addMaskSubmenu` |
| `lpr_add_vector_mask_submenu` | Pick Reveal All in the Add Vector Mask submenu | A reveal-all vector mask is added | `tst_layers_mask_commands::addVectorMaskSubmenu` |
| `lmk_rowmenu_enable` | Pick Enable/Disable Layer Mask on a masked row | The disabled bit flips in one state | `tst_layers_mask_commands::maskMenuActions` |
| A group row omits content commands | Open the row menu on a group | Ungroup, mask, and export rows present | `tst_layers_row_menu::groupRows` |
| `A type row offers rasterize` | Open the row menu on a type row | Rasterize Type present | `tst_layers_row_menu::rasterizeRows` |
| `An unimplemented applicable command is disabled, not absent` | Open the row menu for a kind with an unimplemented applicable row | The row is present, disabled, with the tooltip | `tst_layers_row_menu::unimplementedDisabled` |
| `lpr_row_blocks` | Open the row menu on a styled, masked, mergeable pixel row | Blocks appear in CS6 order with one separator between non-empty blocks | `tst_layers_row_menu::rowBlocksInOrder` |
| `lpr_background_brief` | Open the row menu on the Background | Only the six brief rows | `tst_layers_row_menu::backgroundBriefMenu` |
| `lpr_group_rows` | Open the row menu on a group | Mask, export, link, merge, and flatten rows | `tst_layers_row_menu::groupRows` |
| `lpr_pixel_mask_rows` | Open the row menu on a pixel row that can clip | Create Clipping Mask and the vector-mask rows | `tst_layers_row_menu::pixelMaskAndClippingRows` |
| `lpr_link_rows` | Open the row menu on an unlinked pixel row, then link it | Link/Select Linked, then Unlink | `tst_layers_row_menu::linkRows` |
| `lpr_merge_rows` | Open the row menu for one row above another, then a multi-selection | Merge Down then Merge Layers | `tst_layers_row_menu::mergeRows` |
| `lpr_rasterize_rows` | Open type, shape, fill, smart and plain pixel menus | Kind-specific rasterize labels; plain pixel hides them; styled pixel enables Rasterize Layer Style; styled fill disables it | `tst_layers_row_menu::rasterizeRows`, `tst_layers_row_model::theRowTableCarriesStaticCommandIds` |
| `lpr_isolation_row` | Open the row menu with isolation active | Release from Isolation between the style and merge blocks | `tst_layers_row_menu::isolationRow` |
| `lpr_row_action_target` | Right-click an unselected row and run an action | The row is selected first and the action applies to it | `tst_layers_panel::maskRowActionsTargetTheClickedRow` |
| `lpr_row_action_anchor` | Select two layers, right-click the second, run a layer-specific command | Both stay selected; the clicked row is the anchor | `tst_layers_row_menu::rightClickAnchorsInsideSelection` |
| `lpr_row_action_registry` | Open the row menu on a pixel row | Every implemented document-command row carries its command id and registry state | `tst_layers_row_menu::rowActionsCarryCommandIds` |
