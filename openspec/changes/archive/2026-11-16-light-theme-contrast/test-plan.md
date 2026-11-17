# Test Plan

One row per scenario in the delta specs. Every row is automated.

## ui/design-system

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ds_light_roles_mirror` | Resolve the light family's tokens at the default level and compare surface lightness with text/icon roles. | Every role resolves; light surfaces are lighter than their text/icon roles; dark tokens are unchanged. | `tst_design_system::lightFamilyTokensAreConsistent` |
| `ds_light_derived_distinct` | Resolve each derived shade token in both families. | Each derived shade differs from its base token. | `tst_design_system::lightFamilyTokensAreConsistent` |
| `ds_light_switch_persists` | Persist the light family, restart the window, and switch the family on an open window. | The family is restored, and cached custom-painted glyphs (brush tips) repaint in the new family. | `tst_smoke::themeFamilyPersists`, `tst_brush_dynamics::tipPresetsFollowLightFamily` |
| `ds_light_row_contrast` | Apply the light family and paint a layer row, a channel row, and a brush tip preset. | Row, gutter, selection, and glyph steps move away from white; selected text and tip glyphs stay legible. | `tst_design_system::contrastShadeFollowsFamily`, `tst_panels_parity::layerRowTextStaysLegibleInLightFamily`, `tst_panels_parity::channelRowsFollowLightFamily` |

## ui/layers-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpr_row_surface` | Show a layer row against the list background in each family. | The row steps one shade toward the family's contrasting direction. | `tst_layers_panel::layerRowSurface` |
| `lpr_eye_no_select` | Click a layer row's visibility-toggle column. | Visibility toggles and the row is not selected or highlighted. | `tst_layers_panel::gutterClickTogglesWithoutSelecting` |
| `lpr_selection_grey` | Select a layer row and sample its surface and gutter. | The selection is a neutral grey stepped further than the row, not the palette blue. | `tst_layers_panel::layerRowSurface` |
| `lpr_selection_focus` | Focus and unfocus the tree with a selected row. | The selected row keeps the same flat grey fill and border in both states. | `tst_panels_parity::selectedRowUsesFlatHighlight` |
| `lpr_content_padding` | Show a layer row and measure the content start and separator. | The content starts 2 px after the gutter; the separator stays 1 px. | `tst_layers_panel::layerRowSurface` |
| `lpr_light_row_contrast` | Apply the light family, select a layer, and paint its row with the live palette. | The selection surface is not white and the white name glyphs contrast with it. | `tst_panels_parity::layerRowTextStaysLegibleInLightFamily` |

## ui/channels-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `uic_channel_hover_cursor` | Hover a channel item and read the cursor. | The cursor is the pointing hand. | `tst_panels_parity::channelHoverAndCtrlOverlay` |
| `uic_channel_ctrl_symbol` | Hold Ctrl over a channel item. | The select-all overlay is drawn. | `tst_panels_parity::channelHoverAndCtrlOverlay` |
| `uic_channel_row_style` | Build an RGB document and read both row surfaces. | Rows use the Layers panel's shading. | `tst_panels_parity::channelRowsUseLayerSurfaces` |
| `cp_light_row_contrast` | Apply the light family and read the selected and unselected channel row surfaces. | The selected row is darker than the unselected rows and matches the family-aware step. | `tst_panels_parity::channelRowsFollowLightFamily` |

## ui/brush-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `bp_light_tip_ink` | Open the Brush panel under the dark family, then apply the light family and sample a tip preset icon. | The dark family paints light ink; the light family repaints the glyph and label in dark ink. | `tst_brush_dynamics::tipPresetsFollowLightFamily` |
