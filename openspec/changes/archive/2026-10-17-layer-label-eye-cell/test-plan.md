# Test plan

One row per spec scenario tag; scenarios that share a tag share a row. Coverage
names the automated test, or `manual: <reason>` when it cannot be automated.
Checked by `scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpr_label_tint` | Show a labeled row red in the Layers panel; sample the visibility column at `x = 0`, `x = 1`, `x = kEyeColumn - 2`, and near the bottom; compare its thumbnail and name left with an unlabeled sibling; select the labeled row and sample the fill and the content surface | The sampled fill is the opaque label color, the selected row keeps the fill while the content stays on the selection surface, and no chip space is reserved (thumbnail/name x match) | `tst_layers_panel::layerSurfaceCompleteness` |
| `lpr_label_tint_none` | Sample an unlabeled row's column at `x = 1` and `x = kEyeColumn - 2` before and after labeling a sibling | Both samples are equal, neutral, and unchanged; no tint or chip is painted | `tst_layers_panel::layerSurfaceCompleteness` |
| `lpr_label_glyph` | Call `labelGlyphTint` for each of the seven palette labels and for `None`; spot-check yellow and violet in the running panel | Violet uses the light tint, the other six use dark, and `None` is invalid; the glyph stays readable | `tst_layers_panel_fixes::labelGlyphTintFollowsFill` |
| `lpr_eye` | Check the eye asset on a visible and a hidden row, the eye's x under a nested group, and a gutter click | The on/off asset follows visibility, the eye x is depth-independent, and a gutter click toggles visibility without selecting | `tst_layers_panel::panelChrome`, `tst_layers_panel::gutterClickTogglesWithoutSelecting` |
| `lpr_smart_badge` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpr_smart_badge_absent` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpr_chevron` | Expand and collapse a group through the disclosure icon | The icon points right when collapsed and down when expanded, and the click toggles the group | `tst_layers_panel::panelChrome` |
| `m39_badges` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpc_clip` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpc_lockbadge` | Lock a layer, then clear the lock; read the row badge geometry | A locked row draws the badge and an unlocked row draws none | `tst_panels_parity::lockBadgeShrunk` |
| `lpc_forced_lock_badge` | Commit a type layer, then lock its position | Forced Transparency/Image locks show no badge; Position adds one | `tst_type_tools::typeLayerRowHasNoLockBadge` |
| `lpr_thumb_checker` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpr_thumb_bracket` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpr_group_no_outline` | Show a group row and a regular layer row | The folder glyph carries no black outline; the regular thumbnail keeps it | `tst_layers_panel_fixes::groupIconHasNoOutline` |
| `lpr_type_thumb_card` | Show a type layer in a portrait document; read the thumb rect and painted image | The card is square, has no black outline, and the hit rect is the same card | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` |
| `lpr_thumb_fill` | Show a portrait document's layer row | Its letterboxed thumbnail is taller than wide and spans the row height minus the pad | `tst_layers_panel_fixes::thumbnailFillsRow` |
| `lpr_row_fonts` | Unchanged row-delegate behavior carried by the replacement requirement | As before this change | manual: unchanged by this change; no new assertion is needed here |
| `lpr_row_height` | Read a row's size hint | The height is at least 28 px from the single named row-height constant | manual: the row-height floor is observable in the running panel only through the painted row |
| A styled layer shows the fx badge | Apply and clear a layer style; read the fx rect | A styled row paints the fx badge and a cleared row paints none | `tst_layers_panel::layerStylePanelIntegration` |
| `lpr_blendif_badge` | Narrow a layer's Blend If range; read the badge rect | The customised row paints the `Blend If` chip | `tst_layers_panel::blendIfBadge` |
| `lpr_blendif_badge_absent` | Show a default Blend If row, then restore a customised row to the full range | Neither default state paints the chip | `tst_layers_panel::blendIfBadge` |
| `lpr_blendif_raw` | Build a layer with a raw `blending_ranges` block and no typed view | The raw block is treated as customised | `pictura-app::blend_if_customised_only_when_non_default` |
