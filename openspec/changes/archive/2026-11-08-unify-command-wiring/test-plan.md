# Test plan

New test cases named here are written in the apply phase. Existing cases are
listed so the parity work extends them rather than duplicating them.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `Menu bar is generated from the table` | Construct the frame and walk the menu bar | Every top-level menu and leaf resolves to a catalogue row | `tst_command_tree::cr_menu_catalogue` |
| `Command identifiers are stable` | Change a label and shortcut in a fixture, then dispatch by id | The id resolves and the handler runs | `tst_command_tree::cr_identifiers_stable` |
| `cr_rust_only` | Inspect the repository for a second command table | Only the Rust catalogue and the generated header carry entries | `tst_command_tree::cr_rust_only` |
| `cr_id_drift` | Run `scripts/gen-command-ids.py --check` | Every catalogue name and value matches `command_ids.h`; a mismatch fails | `gen-command-ids::main` |
| `cr_list_commands` | Call `list_commands` before and after the catalogue move | Existing ids, paths, labels, implemented, enabled, and checked are unchanged; panel-only rows are additive | `tst_command_tree::cr_list_commands` |
| `cr_panel_command_no_menu` | Construct the frame with a panel-only command | It is absent from the menu bar, keeps a window action for its shortcut, and is present in `describe()` and the shortcut editor | `tst_command_tree::panelCommandAbsentFromMenuBar` |
| `cr_id_derived_validated` | Derive an id from a path the catalogue does not contain | The bridge returns an empty identifier and warns with the path | `pictura-script::unknown_path_is_rejected` |
| `Undo label tracks history` | Open Edit with no undoable step | The history item reflects no undo | `tst_command_tree::undoLabelTracksHistory` |
| `Label updates after an edit` | Commit an edit and reopen Edit | The label names the new state | `tst_command_tree::undoLabelTracksHistory` |
| `cr_dynamic_label_surfaces` | Open the Layer menu, row menu, and flyout for a multi-selection | Duplicate and Delete read the plural labels on all three | `tst_layers_row_menu::kindTextLists`, `tst_merge_commands::layersPanelContextSensitiveLabel` |
| `cr_modal_metadata` | Ask the registry about known modal and non-modal ids, including the panel dialogs | The modal ids are refused from metadata; the non-modal ids are not | `tst_command_tree::modalMetadata`, `tst_control_modal_guard::panelModalsAreRefused` |
| `cr_modal_not_flagged` | Dispatch every implemented command | Each gets a response and no dialog stays open | `tst_control_modal_guard::everyCommandAnswers` |
| `cr_panel_row_action` | Open a row menu and inspect an action; open the flyout after a document | It carries the command id and matches the registry state; the flyout re-reads it on open | `tst_layers_row_menu::rowActionsCarryCommandIds`, `tst_layers_panel_bundle::flyoutStateFollowsDocument` |
| `cr_panel_shortcut` | Open the shortcuts editor on Application Menus and assign a binding to a panel action | The action is listed under its panel with its label, and the binding fires | `tst_command_tree::panelCommandsInShortcuts`, `tst_keyboard_shortcuts::listsPanelCommands`, `tst_command_tree::panelCommandShortcutWired` |
| `cr_panel_preference` | Inspect the command list for Add Mask by Default | The preference is absent from the list | `tst_command_tree::panelPreferencesNotCommands` |
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
| `lpr_smart_rows` | Open the row menu on a smart object | The six smart-object rows are enabled | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `lpr_smart_linked` | Open the row menu on a smart object | Relink and Convert to Linked are disabled with the tooltip | `tst_smart_object_actions::rowMenuOffersSmartObjectContents` |
| `A plain pixel row omits the smart-object commands` | Open the row menu on a plain pixel row | No smart-object rows | `tst_layers_row_menu::plainPixelOmitsSmartRows` |
| `A pixel row offers its full command set` | Open the row menu on a pixel row | Common, export, mask, style, clipping, smart-object, rasterize rows | `tst_layers_row_menu::pixelRowFullSet` |
| `lmk_rowmenu` | Pick Add Layer Mask > Reveal All, then Delete Layer Mask | A mask is added then removed, one state each | `tst_layers_mask_commands::addMaskSubmenu` |
| `lpr_add_mask_submenu` | Pick Reveal All, then Reveal Selection with a selection | Both masks added, one state each; Reveal Selection disabled without a selection | `tst_layers_mask_commands::addMaskSubmenu` |
| `lpr_add_vector_mask_submenu` | Pick Reveal All in the Add Vector Mask submenu | A reveal-all vector mask is added | `tst_layers_mask_commands::addVectorMaskSubmenu` |
| `lmk_rowmenu_enable` | Pick Enable/Disable Layer Mask on a masked row | The disabled bit flips in one state | `tst_layers_mask_commands::maskMenuActions` |
| `A group row omits content commands` | Open the row menu on a group | No export or fill rows; Ungroup and mask rows present | `tst_layers_row_menu::groupRows` |
| `A type row offers rasterize` | Open the row menu on a type row | Rasterize Type present | `tst_layers_row_menu::rasterizeRows` |
| `An unimplemented applicable command is disabled, not absent` | Open the row menu for a kind with an unimplemented applicable row | The row is present, disabled, with the tooltip | `tst_layers_row_menu::unimplementedDisabled` |
| `lpr_row_blocks` | Open the row menu on a styled, masked, mergeable pixel row | Blocks appear in CS6 order with one separator between non-empty blocks | `tst_layers_row_menu::rowBlocksInOrder` |
| `lpr_background_brief` | Open the row menu on the Background | Only the six brief rows | `tst_layers_row_menu::backgroundBriefMenu` |
| `lpr_group_rows` | Open the row menu on a group | Mask, export, link, merge, and flatten rows | `tst_layers_row_menu::groupRows` |
| `lpr_pixel_mask_rows` | Open the row menu on a pixel row that can clip | Create Clipping Mask and the vector-mask rows | `tst_layers_row_menu::pixelMaskAndClippingRows` |
| `lpr_link_rows` | Open the row menu on an unlinked pixel row, then link it | Link/Select Linked, then Unlink | `tst_layers_row_menu::linkRows` |
| `lpr_merge_rows` | Open the row menu for one row above another, then a multi-selection | Merge Down then Merge Layers | `tst_layers_row_menu::mergeRows` |
| `lpr_rasterize_rows` | Open the row menu for type, shape, fill, smart object, and plain pixel rows | Rasterize Type/Shape/Layer/Layer; plain pixel hides them; styled row shows the disabled style row | `tst_layers_row_menu::rasterizeRows` |
| `lpr_isolation_row` | Open the row menu with isolation active | Release from Isolation between the style and merge blocks | `tst_layers_row_menu::isolationRow` |
| `lpr_row_action_target` | Right-click an unselected row and run an action | The row is selected first and the action applies to it | `tst_layers_panel::maskRowActionsTargetTheClickedRow` |
| `lpr_row_action_anchor` | Select two layers, right-click the second, run a layer-specific command | Both stay selected; the clicked row is the anchor | `tst_layers_row_menu::rightClickAnchorsInsideSelection` |
| `lpr_row_action_registry` | Open the row menu on a pixel row | Every implemented document-command row carries its command id and registry state | `tst_layers_row_menu::rowActionsCarryCommandIds` |
| `lmc_row_menu_enabled_mask` | Open the row menu on an enabled mask | Delete and Disable enabled; Add submenu and Enable greyed | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `lmc_row_menu_states` | Open the row menu without a mask, then with a disabled one | Add submenu enabled, then Delete and Enable | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `lmc_row_menu_submenu` | Open the Add Layer Mask submenu with and without a selection | Selection rows enabled only with a selection | `tst_layers_mask_commands::addMaskSubmenu` |
| `lmc_layer_menu_apply_smart` | Open Layer > Layer Mask on a smart object | Apply is disabled | `tst_layers_mask_commands::layerMenuApplyAvailability` |
| `lmc_mask_entry_agreement` | Compare the Layer menu, mask-thumbnail menu, and Properties | Apply is disabled on all three for a group/adjustment | `tst_layers_mask_commands::layerMenuApplyAvailability` |
| `ks_lists_menus` | Open the dialog on Application Menus | `File > New…` is listed under File with `Ctrl+N` | `tst_keyboard_shortcuts::listsMenusAndTools` |
| `ks_lists_tools` | Switch to Tools | Move is listed with `V` | `tst_keyboard_shortcuts::listsMenusAndTools` |
| `ks_lists_panel_commands` | Open the dialog on Application Menus | A panel action appears under its panel | `tst_keyboard_shortcuts::listsPanelCommands` |
| `ll_last_layer_disabled` | Reduce a document to one layer | Trash, row menu, flyout, Properties, and Layer > Delete are greyed | `tst_last_layer::theLastLayerCannotBeDeleted` (flyout checked in the same case) |
| `ll_last_layer_refused` | Delete the last top-level layer from every entry point | The node stays and no state is recorded | `tst_last_layer::theLastLayerCannotBeDeleted` |
| `ll_remove_layer_guarded` | Call `remove_layer` on the last layer | Refused; the layer stays; no state | `tst_last_layer::removeLayerKeepsTheLastLayer` |
| `ll_can_delete_agrees` | Ask availability for each selection shape | It matches what the delete would do | `tst_last_layer::canDeleteAgrees` |
| `ll_properties_footer_guarded` | Show an undeletable layer in Properties and invoke the footer command | The footer button is disabled and the direct command is refused; the layer stays | `tst_properties_panel::footerDeleteUsesTheGate` (button state included) |
