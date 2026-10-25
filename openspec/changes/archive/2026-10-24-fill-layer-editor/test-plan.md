# Test plan

Every scenario in the delta specs has a row. The new `tst_fill_layer_editor`
suite is written in task 1 before the behavior lands.

## Delta scenarios

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `fle_solid_apply` | Open the Solid Color editor, pick a new colour, accept | Fill carries the colour, composite updates, one state | `tst_fill_layer_editor::solidApply` |
| `fle_solid_cancel` | Open the Solid Color editor and cancel | Fill, composite, and history unchanged | `tst_fill_layer_editor::solidCancel` |
| `fle_gradient_apply` | Change stops, kind, angle, scale, reverse; accept | Every value lands in the `GdFl` block, one state | `tst_fill_layer_editor::gradientApply` |
| `fle_gradient_roundtrip` | Encode a gradient with reverse true and scale 60, then decode | Decoded params match | `pictura-render::encode_gradient_fill_params_round_trips` |
| `fle_gradient_editor` | Activate the sample | Gradient Editor opens on the current gradient; accepted edits update the sample | `tst_fill_layer_editor::gradientEditorOpens` |
| `fle_gradient_cancel` | Change values and cancel | Fill, composite, and history unchanged | `tst_fill_layer_editor::gradientCancel` |
| `fle_pattern_apply` | Pick a document pattern with a scale and link choice; accept | `PtFl` carries the id, scale, and link; one state | `tst_fill_layer_editor::patternApply` |
| `fle_pattern_snap` | Activate Snap To Origin and accept | Origin becomes `(0, 0)` | `tst_fill_layer_editor::patternSnapToOrigin` |
| `fle_pattern_empty` | Open the editor on a document with no patterns | Empty state; Pattern creation and editing unavailable | `tst_fill_layer_editor::patternEmpty` |
| `fle_create_layer_menu` | Layer menu gradient flow; accept both dialogs | One gradient fill above the active layer, selected, one state | `tst_fill_layer_editor::createFromLayerMenu` |
| `fle_create_cancel` | Accept the New Layer dialog, cancel the type editor | No layer, no state | `tst_fill_layer_editor::createCancel` |
| `fle_create_panel` | Click the Solid Color icon | Default fill created and its editor opens | `tst_fill_layer_editor::createFromPanel` |
| `fle_content_options` | Invoke Content Options on a fill layer | The matching fill editor opens | `tst_fill_layer_editor::contentOptionsOpensEditor` |
| `fle_thumb_dblclick` | Double-click a fill content thumbnail | The matching fill editor opens, no Layer Style | `tst_fill_layer_editor::thumbDoubleClickOpensEditor` |
| `fle_adjustment_properties` | Double-click an adjustment content thumbnail | Properties shown, no fill editor | `tst_fill_layer_editor::adjustmentThumbOpensProperties` |
| `fle_mask_thumb_unchanged` | Double-click a mask thumbnail | Existing mask target behavior, no fill editor | `tst_fill_layer_editor::maskThumbKeepsTarget` |
| The menu lists every adjustment kind | Open the New Fill / Adjustment menu | Sixteen adjustment kinds plus the fill types | `tst_command_tree::panelMenusToolsIcons` |
| `lpr_pattern_entry` | Open the menu with and without document patterns | Pattern enabled with patterns, disabled with the reason without | `tst_fill_layer_editor::layerFillMenuPatternGating` |
| `lpr_layer_fill_menu` | Open the Layer > New Fill Layer submenu | Solid Color/Gradient/Pattern with the same gating | `tst_fill_layer_editor::layerFillMenuPatternGating` |
| The command opens the fill editor | Content Options on a fill layer | Matching editor opens | `tst_fill_layer_editor::contentOptionsOpensEditor` |
| The command opens Properties for an adjustment layer | Content Options on an adjustment | Properties shown and refreshed | `tst_command_tree::layerContentOptions` |
| The command is disabled for other layers | Content Options on pixel/type/group/Background | Disabled and inert | `tst_command_tree::layerContentOptions` |
| A double-click opens Blending Options | Double-click a pixel row outside the name | Layer Style opens | `tst_layers_panel::layerStylePanelIntegration` |
| A background double-click does not open a style | Double-click the Background outside the name | Conversion path, no style | `tst_fill_layer_editor::backgroundDoubleClickConverts` |
| The style rows report their enablement | Styled row with a copied style | All four style rows enabled | `tst_layers_panel::layerStylePanelIntegration` |
| An unstyled pixel row disables Copy and Clear | Unstyled pixel row | Blending enabled, Copy/Paste/Clear disabled | `tst_layers_panel::layerStylePanelIntegration` |
| An adjustment row omits the style rows | Adjustment row | No style rows | `tst_layers_panel::layerStylePanelIntegration` |
| The row menu clears a row's style | Clear a styled row | Style removed, fx badge gone | `tst_layers_panel::layerStylePanelIntegration` |
| Alt-clicking the fx region toggles all effects | Alt-click a styled row's fx | All effects hide, then show | `tst_layer_style::menuCommands` |
| The Effect dimension matches styled rows | Filter by an effect | Only rows carrying it | `tst_layer_style::menuCommands` |
| The Effect dimension lists the effect keys | Open the Effect choices | The ten effect keys | `tst_layer_style::menuCommands` |
| `lpr_adjustment_thumb_opens` | Double-click an adjustment content thumbnail | Properties, no Layer Style | `tst_fill_layer_editor::adjustmentThumbOpensProperties` |
| `lpr_fill_thumb_opens` | Double-click a fill content thumbnail | Fill editor, no Layer Style | `tst_fill_layer_editor::thumbDoubleClickOpensEditor` |
| Solid Color icon opens the solid-color flow | Click the Solid Color icon | The command dispatches | `tst_adjustments_panel::solidColorIconDispatchesFillCommand` |
| `adj_pattern_icon` | Show the Pattern icon with and without document patterns | Enabled and dispatches with patterns; disabled with the reason without | `tst_fill_layer_editor::patternIconGating` |
| No document disables the grid | Show the grid with no document | Every entry disabled | `tst_adjustments_panel::noDocumentDisablesGrid` |
| A document enables the grid | Show the grid with a document | All entries enabled except Pattern when there are no patterns | `tst_adjustments_panel::documentEnablesGrid` |
| `alf_gradient_full_roundtrip` | Encode Radial, angle 45, reverse true, scale 60 | Decoded params match | `pictura-render::encode_gradient_fill_params_round_trips` |
| `alf_gradient_defaults` | Encode reverse false, scale 100 | No `Rvrs`/`Scl ` item; decode defaults | `pictura-render::encode_gradient_fill_params_omits_defaults` |
| `alf_pattern_roundtrip` | Encode pattern id, scale, link, origin | Decoded params match | `pictura-render::encode_pattern_fill_round_trips` |
| `alf_pattern_default_origin` | Encode origin `(0, 0)` | No `phase` object; decode origin `(0, 0)` | `pictura-render::encode_pattern_fill_omits_default_origin` |

## Untagged scenario titles

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Pattern icon is inert | Click the Pattern icon with no document patterns | No layer, no command, icon disabled with the no-patterns reason | `tst_fill_layer_editor::patternIconGating` |
| Pattern stays disabled until authoring exists | Open the creation menu with no document patterns | Pattern disabled with the no-patterns reason | `tst_fill_layer_editor::layerFillMenuPatternGating` |

## Regression guard

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Existing fill creation | Create a Solid Color and a Gradient fill without a dialog | The default layer is unchanged and composited correctly | `tst_layers_snapshot::fillThumbnailPreviews` |
| Existing Layer Content Options | Invoke it on an adjustment | Properties still opens | `tst_command_tree::layerContentOptions` |
| Existing mask thumbnails | Click and Shift-click mask and vector thumbnails | Target and enable behavior unchanged | `tst_fill_layer_editor::maskThumbKeepsTarget` |
| Existing gradient map editor | Open Image > Adjustments > Gradient Map | The reused Gradient Editor still drives the map | `tst_image_adjustments::gradientMapEditsItsGradient` |
