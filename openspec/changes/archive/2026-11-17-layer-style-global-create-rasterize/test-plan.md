# Test plan

Every scenario in the delta specs has a row. New coverage named here is written
by the task groups of `tasks.md`; rows for existing behaviour point at the
current suite or a dedicated regression test and are expected to keep passing
after the copy/rename changes.

## Delta scenarios

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pgl_read` | Read a document whose resources 1037/1049 hold 45 and 60 | Global light reads 45/60 | `pictura-codec::global_light_reads_angle_and_altitude` |
| `pgl_default` | Read a document with no resources, and one with a two-byte 1037 | Missing values read 30; open does not fail or panic | `pictura-codec::global_light_defaults_for_missing_or_malformed` |
| `pgl_write` | Set 90/45 and save a document that carried an XMP block | 1037 = 90, 1049 = 45, XMP bytes unchanged | `pictura-codec::global_light_writes_and_preserves_other_resources` |
| `pgl_roundtrip` | Save a document with no light resources and no light edit | No 1037/1049 written; section bytes match the input | `pictura-codec::global_light_unchanged_section_round_trips` |
| `pgl_roundtrip_matching` | Save a document already holding matching 1037 = 45 and 1049 = 60 without editing the light | Both resources keep their values; section is byte-identical | `pictura-codec::global_light_matching_section_round_trips` |
| `gle_angle` | Composite the same shadow at global angle 0 and 90 | The shadow paints on opposite sides | `pictura-render::global_light_angle_orients_a_shadow` |
| `gle_local` | Change the global angle with `uglg` false | Composite is unchanged | `pictura-render::global_light_ignored_when_local` |
| `gle_cache` | Composite, change the document light, composite again | The second composite shows the new light | `pictura-render::global_light_change_rebuilds_passes` |
| `gle_altitude` | Render an enabled bevel at global altitude 30 and 60 | Highlight/shadow coverage changes | `pictura-render::global_light_altitude_lights_a_bevel` |
| `gle_inner_shadow` | Composite an inner shadow at global angle 0 and 90 | The band appears on opposite edges | `pictura-render::global_light_inner_shadow_angle` |
| `gle_bevel_angle` | Change the global angle with a bevel using it | The highlighted side swaps | `pictura-render::global_light_bevel_angle` |
| `lsa_global_set` | Set 370/95 through the setter; accept the dialog with 90/45 | Document reads 10/90; one "Global Light" state | `pictura-render::global_light_setter_normalises_and_clamps`, `tst_layer_style::globalLightDialogWritesAndUndoes` |
| `lsa_global_noop` | Set the values already stored | Setter returns false; no state | `pictura-render::global_light_setter_ignores_unchanged` |
| `lsa_create_layers` | Explode a style with a shadow, bevel, Color Overlay, Inner Glow and Stroke and recomposite | Composite matches within tolerance; base unstyled; new sibling layers carry pixels/modes | `pictura-render::create_style_layers_recomposes_identically`, `pictura-render::create_style_layers_names_and_modes`, `tst_layer_style::createLayersCommand` |
| `lsa_create_inherit` | Explode a base at opacity 128 with a raster mask | Produced layers: opacity 128, fill 255, no mask, unique names; base keeps its mask/fill | `pictura-render::create_style_layers_inherits_base_state` |
| `lsa_create_order` | Explode Pattern + Color Overlay + Inner Glow + Stroke | Above layers read Pattern, Color, Inner Glow, Stroke bottom-to-top | `pictura-render::create_style_layers_above_order` |
| `lsa_create_global` | Explode a globally-lit shadow with document angle 90 | Recomposes at 90, not the stored local angle | `pictura-render::create_style_layers_global_light` |
| `lsa_create_bevel` | Explode an Inner bevel and a Pillow bevel | Inner yields two layers, Pillow four, above the base | `pictura-render::create_style_layers_pillow_four_passes` |
| `lsa_create_refuse` | Ask for an unstyled layer, group, Background, only-disabled effects, clipped base, or base owning a clipped run | No layers, no style change, no state | `pictura-render::create_style_layers_refuses` |
| `lsa_rasterize` | Rasterize a shadowed pixel layer | Shadow in pixels; `has_layer_style` false; composite matches within tolerance | `pictura-render::rasterize_layer_style_bakes_and_clears` |
| `lsa_rasterize_fill` | Rasterize a styled layer at Fill 50 % | Composite unchanged; fill reads 255 | `pictura-render::rasterize_layer_style_fill_cases` |
| `lsa_rasterize_global` | Rasterize a globally-lit shadow at document angle 90 | Baked pixels match the pre-command 90° composite | `pictura-render::rasterize_layer_style_global_light` |
| `lsa_rasterize_pattern` | Rasterize a Pattern Overlay | Baked pixels show the document pattern | `pictura-render::rasterize_layer_style_pattern_overlay` |
| `lsa_rasterize_refuse` | Rasterize an unstyled layer, fill, type, group, Background, only-disabled effects, customized Blend If/knockout, or base owning a clipped run | Nothing changes; no state | `pictura-render::rasterize_layer_style_refuses` |
| `lsa_rasterize_undo` | Undo a baked layer | Style returns and pixels restore in one step | `tst_layer_style::rasterizeLayerStyleCommand` |
| `Copy, paste, clear, hide and show` | Copy, paste, clear, hide and show | Each command acts and records its named state | `tst_layer_style::menuCommands` |
| `lsd_menu_global_create` | Choose Global Light… then Create Layers on a styled layer | Dialog opens; Create Layers records one state and clears the base style | `tst_layer_style::globalLightDialogWritesAndUndoes`, `tst_layer_style::createLayersCommand` |
| `lsd_menu_rasterize` | Enablement with a renderable style, then only disabled effects | Enabled then disabled | `tst_layer_style::rasterizeLayerStyleCommand` |
| `lsd_global_dialog` | Accept the dialog with 90/45 | Document 90/45; globally-lit effects follow; one state | `tst_layer_style::globalLightDialogWritesAndUndoes` |
| `lsd_global_cancel` | Change the angle and cancel | Previous light kept; no state | `tst_layer_style::globalLightDialogCancelKeepsTheLight` |
| `lsd_global_page` | Check Use Global Light with global angle 90 | Angle field reads 90 and is disabled; unchecking restores the local value | `tst_layer_style::globalAngleFieldFollowsGlobalLight` |
| `lsd_global_altitude` | Check Use Global Light on the bevel with global altitude 60 | Altitude field reads 60 and is disabled; unchecking restores the local value | `tst_layer_style::globalAltitudeFieldFollowsGlobalLight` |
| `m39_menus` | Open the panel menu | Panel Options and wired layer commands present | `tst_command_tree::panelMenusToolsIcons` |
| `m39_menus` | Pick Red from the Color Label submenu | Row label becomes Red | `tst_layers_panel::layerSurfaceCompleteness` |
| `m39_menus` | Pick Move Layer Up | Layer swaps in one state | `tst_command_tree::menuMoves` |
| `m39_strip` | Inspect the action strip | Seven CS6 buttons, no Move | `tst_layers_panel_fixes::actionStripStaysFixedAndCentred` |
| `Pixel and smart-object rows offer export` | Open pixel and smart-object row menus | Quick Export then Export As | `tst_layers_row_menu::pixelAndSmartRowsOfferExport` |
| `Group and adjustment rows offer export` | Open group and adjustment row menus | Export rows present per the current requirement | `tst_layers_row_menu::groupAndAdjustmentOfferExport` |
| `Type rows offer export` | Open a type row menu | Export rows present per the current requirement | `tst_layers_row_menu::typeRowsOfferExport` |
| `A shape row offers its shape commands` | Open the shape row menu | Copy/Paste Shape Attributes and Rasterize Shape | `tst_command_tree::shapeLayerCommands` |
| `A shape row is not offered the adjustment edit` | Open the shape row menu | No Edit Adjustment… | `tst_layers_row_menu::shapeOmitsEditAdjustment` |
| `lpr_convert_row` | Pick Convert to Smart Object on a convertible pixel row | Converts and selects the smart object | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `An existing smart object disables Convert to Smart Object` | Open a smart-object row menu | Convert disabled | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `lpr_smart_rows` | Open a smart-object row menu | The six smart rows are enabled | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `lpr_smart_linked` | Open a smart-object row menu | Relink and Convert to Linked disabled with tooltip | `tst_smart_object_actions::rowMenuOffersSmartRows` |
| `A plain pixel row omits the smart-object commands` | Open a plain pixel row menu | No smart-object-only rows | `tst_layers_row_menu::plainPixelOmitsSmartRows` |
| `A pixel row offers its full command set` | Open a pixel row menu | Common, export, mask, style, clipping, smart and rasterize rows | `tst_layers_row_menu::pixelRowFullSet` |
| `lmk_rowmenu` | Add then delete a mask from the row menu | Both apply in one state each | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `lpr_add_mask_submenu` | Create a mask from the Add Layer Mask submenu | Mask follows the chosen entry | `tst_layers_mask_commands::addMaskSubmenu` |
| `lpr_add_vector_mask_submenu` | Create a vector mask from the submenu | Vector mask follows the chosen entry | `tst_layers_mask_commands::addVectorMaskSubmenu` |
| `lmk_rowmenu_enable` | Toggle mask enablement from the row menu | Disabled bit flips | `tst_layers_mask_commands::rowMenuFollowsTheMaskState` |
| `A group row omits content commands` | Open a group row menu | No export/fill; mask rows and Ungroup present | `tst_layers_row_menu::groupRows` |
| `A type row offers rasterize` | Open a type row menu | Rasterize Type present and enabled | `tst_layers_row_menu::rasterizeRows` |
| `An unimplemented applicable command is disabled, not absent` | Open a menu row for an unimplemented applicable command | Present, disabled, tooltip shown | `tst_layers_row_menu::unimplementedDisabled` |
| `lpr_row_blocks` | Open a pixel row menu | Blocks in order with one separator between | `tst_layers_row_menu::rowBlocksInOrder` |
| `lpr_background_brief` | Open the Background row menu | Only the six brief rows | `tst_layers_row_menu::backgroundBriefMenu` |
| `lpr_group_rows` | Open a group row menu | Mask, link and merge rows; no export/fill | `tst_layers_row_menu::groupRows` |
| `lpr_pixel_mask_rows` | Open a pixel row menu that can clip | Create Clipping Mask and vector-mask rows; clipped row shows Release | `tst_layers_row_menu::pixelMaskAndClippingRows` |
| `lpr_link_rows` | Open an unlinked row menu, link it, reopen | Link then Unlink; Select Linked Layers present | `tst_layers_row_menu::linkRows` |
| `lpr_merge_rows` | Open a single-row and a multi-selection menu | Merge Down vs Merge Layers; visible/clipping/flatten | `tst_layers_row_menu::mergeRows` |
| `lpr_rasterize_rows` | Open type, shape, fill, smart and plain pixel menus | Kind-specific rasterize labels; plain pixel hides them; styled pixel enables Rasterize Layer Style; styled fill disables it | `tst_layers_row_menu::rasterizeRows`, `tst_layers_row_model::theRowTableCarriesStaticCommandIds` |
| `lpr_isolation_row` | Enter isolation and open a row menu | Release from Isolation before the merge block | `tst_layers_row_menu::isolationRow` |
| `lpr_row_action_target` | Right-click an unselected row and run an action | The row is selected first and the action applies to it | manual: the context menu is a modal QMenu; verify in the running app |
| `lpr_row_action_anchor` | Right-click inside a selection | The clicked row becomes the anchor | `tst_layers_row_menu::rightClickAnchorsInsideSelection` |
| `lpr_row_action_registry` | Open a pixel row menu | Actions carry registry ids and matching state | `tst_layers_row_menu::rowActionsCarryCommandIds` |
| `lpr_fx_dblclick_effect` | Double-click an effect row | Layer Style opens on that page | `tst_layers_effects::doubleClickEffectOpensItsPage` |
| `lpr_fx_dblclick_badge` | Double-click the fx badge | Layer Style opens on Blending Options | `tst_layers_effects::doubleClickBadgeOpensBlendingOptions` |
| `lpr_fx_menu` | Right-click the fx badge / effects row | Menu offers Global Light… and Create Layers enabled, registry-dispatched | `tst_layers_effects::rowMenuListsLayerStyleCommands` |
| `lpr_fx_disable` | Choose Disable Layer Effects | `fx.visible` false, unstyled composite, one state | `tst_layers_effects::disableLayerEffectsIsOneState` |

## Regression guard

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Existing style commands | Copy, paste, clear, scale, show/hide on a styled layer | They are unchanged; Global Light is now enabled and opens its dialog | `tst_layer_style::menuCommands` |
| Existing effect rendering | Composite each effect family with `uglg` false | Pixel results unchanged | `pictura-render::global_light_ignored_when_local` |
| Existing row menus | Open pixel, group, fill, type and smart rows | Rows unchanged except Global Light…/Create Layers enabled in the style submenu and Rasterize Layer Style enabled on a styled raster row | `tst_layers_row_menu::kindTextLists`, `tst_layers_effects::rowMenuListsLayerStyleCommands`, `tst_layers_row_menu::rasterizeRows` |
| A bevel and emboss descriptor decodes to typed parameters | Decode an `ebbl` with every field set | The typed values match the descriptor | `pictura-render::bevel_decodes_typed_parameters` |
| Missing bevel and emboss keys take Photoshop defaults | Decode an `ebbl` with only `enab`/`present` | Photoshop defaults, `use_global_angle` true | `pictura-render::bevel_missing_keys_take_defaults` |
| The style, technique and direction enums decode every value | Decode every enum value | Each maps to its typed variant | `pictura-render::bevel_enums_decode_every_value` |
| Out-of-range numeric values are clamped or rejected | Decode huge and overflowing values | Clamped within range; overflow is `None` | `pictura-render::bevel_huge_numeric_parameters_are_bounded_or_rejected` |
| A disabled bevel and emboss decodes but is inert | Decode and composite a disabled bevel | Decodes with `enabled` false; composite is byte-identical | `pictura-render::bevel_disabled_decodes_but_is_inert` |
| Malformed or absent bevel and emboss effects are a no-op | Decode absent/malformed `ebbl` cases | `None`, no panic | `pictura-render::bevel_malformed_or_absent_is_none` |
