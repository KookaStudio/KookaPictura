# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| bif_ranges | Edit source and backdrop on Gray and RGB channels | Preview updates; unrelated channel groups survive | `tst_blend_if::ranges`, `tst_blend_if::preview` |
| bif_modes | Open CMYK and Lab documents with opaque ranges and save | Controls and replacement unavailable; bytes and typed ranges unchanged | `tst_blend_if::unsupportedModes` |
| bif_group | Open a pass-through group carrying opaque ranges and attempt replacement | Editing and replacement unavailable; the exact ranges remain | `tst_blend_if::passThroughGroup` |
| bif_split | Test black and white splits for source and backdrop at 0, 255, both endpoints and midpoint; drag to boundaries and reopen | Ordered linear fades, inclusive equal-endpoint steps, default extremes retained, no crossing and endpoints survive | `pictura-render::split_blend_if`, `tst_blend_if::split` |
| bif_history | Accept, undo and cancel changes | One undo state; cancellation restores typed and raw state | `tst_blend_if::history` |
| bif_save | Save and reopen edited PSD with additional channel groups | All ranges survive | `tst_blend_if::save` |
| bif_cache | Populate a cached prefix, then repeatedly edit a lower layer | Every preview and committed composite reflects the latest range | `pictura-render::blend_if_prefix_invalidation`, `pictura-render::cached_prefix_updates_blend_if_output`, `pictura-render::reduced_preview_updates_target_blend_if` |
| bif_opaque | Open malformed raw block, decline replacement, save; replace then cancel; replace, accept and undo | No implicit loss; exact bytes restored on decline, cancel and undo; accepted replacement starts at defaults | `tst_blend_if::opaqueRanges` |
| lpr_eye | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel::layerRowSurface`, `tst_panels_parity::hiddenLayerShowsFadedEye`, `tst_layers_panel::panelChrome` |
| lpr_label_tint | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::labelGlyphTintFollowsFill` |
| lpr_label_tint_none | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel::layerSurfaceCompleteness` |
| lpr_label_glyph | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::labelGlyphTintFollowsFill` |
| lpr_smart_badge | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::kindBadgesHidpi` |
| lpr_smart_badge_absent | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::kindBadgesHidpi` |
| lpr_chevron | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::emptyGroupShowsChevron` |
| m39_badges | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel::maskRowIndicators`, `tst_panels_parity::emptyGroupShowsChevron` |
| lpc_clip | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel::clippingMasks` |
| lpc_lockbadge | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::lockBadgeShrunk` |
| lpr_lock_size | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::lockBadgeShrunk` |
| lpr_badge_sizes | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::lockBadgeShrunk`, `tst_panels_parity::badgeSizesShrunk` |
| lpc_forced_lock_badge | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_type_tools::typeLayerRowHasNoLockBadge` |
| lpr_thumb_checker | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::thumbnailFillsRow` |
| lpr_thumb_bracket | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_panels_parity::thumbnailAdjacencyAndActiveThumb` |
| lpr_group_no_outline | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::groupIconHasNoOutline` |
| lpr_type_thumb_card | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` |
| lpr_thumb_fill | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::thumbnailFillsRow` |
| lpr_row_fonts | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel::layerSurfaceCompleteness` |
| lpr_row_height | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::thumbnailFillsRow` |
| A styled layer shows the fx badge | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel::layerStylePanelIntegration` |
| lpr_fx_absent_adjustment | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_effects::adjustmentRowsShowNoFxBadge` |
| lpr_blendif_badge | Exercise the badge with non-default, default and malformed raw ranges and inspect row geometry | Icon visibility and name layout follow the retained contract | `tst_blend_if::badge` |
| lpr_blendif_badge_absent | Exercise the badge with non-default, default and malformed raw ranges and inspect row geometry | Icon visibility and name layout follow the retained contract | `tst_blend_if::badge` |
| lpr_blendif_raw | Exercise the badge with non-default, default and malformed raw ranges and inspect row geometry | Icon visibility and name layout follow the retained contract | `tst_blend_if::badge` |
| lpr_thumb_icon_no_outline | Exercise the inherited layer-row scenario | Existing row behavior is retained | `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline`, `tst_layers_panel_fixes::adjustmentThumbnailIsSquareWithoutOutline` |
