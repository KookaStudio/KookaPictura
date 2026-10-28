# Test plan

Every scenario in the delta spec has a row. CI runs the Qt suites; no local build
is required for this change. Scenarios carried over unchanged from the modified
requirements keep their existing coverage and are listed as `manual:` with the
reason, matching the `text-layer-thumbnail-card` precedent.

## Delta scenarios

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpr_thumb_aspect` | Show a regular layer of a non-square document | The drawn thumbnail matches the document aspect, the unused slot left empty | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` |
| `lpr_thumb_source_aspect` | Show a full-width layer of a tall document with Entire Document contents | The thumbnail image has the document aspect and no transparent padding column | `tst_layers_panel_fixes::tallThumbnailKeepsDocumentAspect` |
| `lpr_thumb_square_card` | Show an adjustment, fill, type, and Smart Filters row in a portrait document | Each card is 1:1; a pixel row stays letterboxed | `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline`, `tst_layers_panel_fixes::fillThumbnailIsSquareWithOutline` |
| `uip_layers_mask_adjacency` | Show a masked and vector-masked row | Mask right of image, vector right of mask, chain between | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| `uip_layers_active_thumb` | Click a thumbnail | Brackets on it; never on a group icon | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| `uip_layers_link_glyph` | Show linked and unlinked masks | Taller-than-wide rotated chain | `tst_layers_panel_fixes::linkGlyphRotatedAndLarger` |
| `lpr_thumb_adjustment_icon` | Show an Invert and a Levels row | Neutral card with the kind icon, no checkerboard | `tst_layers_snapshot::adjustmentThumbnailKindIcon` |
| `lpr_thumb_fill_preview` | Show Solid, Gradient, Pattern fill rows | Swatch, ramp, and scaled tile previews | `tst_layers_snapshot::fillThumbnailPreviews` |
| `lpr_thumb_adjustment_square` | Show an Invert row in a portrait document; read the thumb rect and painted image | The card is 1:1, the icon is not stretched, and there is no black outline | `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline` |
| `lpr_thumb_fill_square` | Show a Solid Color fill row in a portrait document; read the snapshot preview and the painted card | The preview image is square and the card is 1:1 with the preview filling it and keeping its outline | `tst_layers_panel_fixes::fillThumbnailIsSquareWithOutline`, `tst_layers_snapshot::fillThumbnailIsSquare` |
| `lpr_thumb_mask_square` | Show an adjustment and a fill row carrying a mask and a vector mask | Both mask cards use the same 1:1 rect as the image card | `tst_layers_panel_fixes::maskThumbnailMatchesSquareCard` |
| `lpr_eye` | Check the eye asset on a visible and a hidden row, and the eye x under a nested group | The on/off asset follows visibility and the eye x is depth-independent | `tst_layers_panel::panelChrome`, `tst_layers_panel::gutterClickTogglesWithoutSelecting` |
| `lpr_label_tint` | Show a layer with a non-`None` color label | The whole visibility column is filled behind the eye, no chip | manual: unchanged by this change; covered by the existing label/eye-cell suite |
| `lpr_label_tint_none` | Show a layer with label `None` | The normal gutter surface, no tint or chip | manual: unchanged by this change; covered by the existing label/eye-cell suite |
| `lpr_label_glyph` | Show a row for each of the seven color labels | The eye glyph is tinted for contrast on every fill | `tst_layers_panel_fixes::labelGlyphTintFollowsFill` |
| `lpr_smart_badge` | Show an embedded or placed smart-object layer | The smart-object badge on the thumbnail's lower-right corner | manual: unchanged by this change; covered by the existing smart-object row suite |
| `lpr_smart_badge_absent` | Show an ordinary pixel layer | No smart-object badge | manual: unchanged by this change; covered by the existing smart-object row suite |
| `lpr_chevron` | Show a group collapsed and expanded, then click the chevron | Right/down icon and a toggled group | manual: unchanged by this change; covered by the existing group/chevron checks |
| `m39_badges` | Show a masked layer, a group with children, and a row whose badge asset is missing | Mask thumbnail present, group expandable, missing badge omitted | `tst_layers_panel::maskRowIndicators` |
| `lpc_clip` | Show a clipped layer | The clipping indicator plus the indentation and base underline | `tst_layers_panel::clippingMasks` |
| `lpc_lockbadge` | Show a locked and an unlocked layer | A right-side lock badge only on the locked row | manual: unchanged by this change; covered by the existing lock-badge checks |
| `lpc_forced_lock_badge` | Commit a type layer, then lock its position | Forced Transparency/Image locks show no badge; Position adds one | `tst_type_tools::typeLayerRowHasNoLockBadge` |
| `lpr_thumb_checker` | Show a regular layer with translucent pixels and a group row | Checkerboard under the regular thumbnail; folder glyph and no checkerboard for the group | manual: unchanged by this change; covered by the existing row-surface suite |
| `lpr_thumb_bracket` | Show a regular layer row, single- and multi-selected | White corner brackets only on the singular active thumbnail | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| `lpr_group_no_outline` | Show a group row and a regular layer row | The folder glyph has no black outline; the regular thumbnail keeps it | `tst_layers_panel_fixes::groupIconHasNoOutline` |
| `lpr_type_thumb_card` | Open a portrait document; commit a type layer; read the thumb rect and painted image | The card is square, has no black outline, and the hit rect is that card | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` |
| `lpr_thumb_fill` | Show a portrait document's layer row | Its letterboxed thumbnail is taller than wide and spans the row height minus the pad | `tst_layers_panel_fixes::thumbnailFillsRow` |
| `lpr_row_fonts` | Inspect the Background, a linked layer, and a placed smart object | Background italic, others normal, linked/placed underlined | manual: unchanged by this change; the name-font roles are asserted by the panel suite |
| `lpr_row_height` | Read a row's size hint | At least 28 px from the single named row-height constant | `tst_layers_panel_fixes::thumbnailFillsRow` |
| A styled layer shows the fx badge | Show a styled and an unstyled row | The styled row draws the fx badge, the unstyled one does not | `tst_layers_panel::layerStylePanelIntegration` |
| `lpr_fx_absent_adjustment` | Show an adjustment and a fill row with no effects, then with an effect | No fx badge or reserved space until the layer carries an effect | `tst_layers_effects::adjustmentRowsShowNoFxBadge` |
| `lpr_blendif_badge` | Narrow a layer's Blend If range; read the badge rect | The customised row paints the `Blend If` chip | `tst_layers_panel::blendIfBadge` |
| `lpr_blendif_badge_absent` | Show a layer with no Blend If customisation | No chip and the full right-edge run for the name | `tst_layers_panel::blendIfBadge` |
| `lpr_blendif_raw` | Show a layer with an unparsable `blending_ranges` block | The row still draws the `Blend If` chip | `tst_layers_panel::blendIfBadge` |
| `lpr_thumb_icon_no_outline` | Show an adjustment, a Smart Filters, and a layer-effects row with no raster image; and a pixel and a fill row | The icon-only cards have no black outline; the raster cards keep theirs | `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline`, `tst_layers_effects::effectsRowHasNoOutline`, `tst_layers_smart_filters::smartFiltersRowHasNoOutline` |

## Regression guard

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Pixel and shape rows keep the letterboxed, outlined card | Show a pixel layer and a shape layer in a portrait document | Both cards are document-aspect and carry the black outline | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` |
| Pixel-row mask and vector-mask layout is unchanged | Show a masked and vector-masked pixel layer | The mask cards keep the document aspect and adjacency | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
