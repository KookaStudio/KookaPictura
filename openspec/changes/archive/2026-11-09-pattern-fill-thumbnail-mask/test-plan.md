# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A Patt block decodes to pattern pixels | Decode the `pattern_fill.psd` fixture and inspect `decode_patterns` | One pattern with the authored id, 2×2 size, and row-major RGBA | `pictura-codec::rgb_pattern_decodes_id_size_and_rgba` |
| `pat_photoshop_slots` | Decode a synthetic pattern declaring 24 colour slots, three written RGB planes, and a written trailing alpha slot | The pattern decodes; R/G/B come from the first three slots and alpha from the last written slot | `pictura-codec::photoshop_slot_layout_decodes_written_color_planes` |
| `pat_slot_count_cap` | Decode a pattern declaring 25 colour slots | The pattern is skipped without reading its channels | `pictura-codec::a_colour_slot_count_above_photoshop_layout_is_skipped` |
| A grayscale pattern replicates its single channel | Decode a Grayscale pattern with one written colour plane | R, G, and B all equal the plane value | `pictura-codec::grayscale_pattern_replicates_its_channel` |
| A malformed Patt block is skipped | Decode a `Patt` list with a well-formed entry followed by an overrunning length | The earlier pattern survives and nothing panics | `pictura-codec::unsupported_mode_is_skipped_and_truncated_block_keeps_earlier` |
| Non-8-bit pattern planes are skipped | Decode a pattern whose leading colour plane declares 16-bit depth | The pattern is skipped and the following pattern survives | `pictura-codec::non_8_bit_pattern_is_skipped` |
| A crafted oversized pattern rectangle is skipped | Decode a pattern whose channel rectangles disagree with an oversized declared rectangle | The pattern is skipped without a large allocation | `pictura-codec::crafted_oversized_pattern_rect_is_skipped_without_allocating` |
| The trailing alpha slot is the pattern transparency | Decode a pattern with both alpha slots written | The decoded alpha is the last written slot | `pictura-codec::both_alpha_slots_picks_the_last_written` |
| A pattern with a missing colour plane is skipped | Decode an RGB pattern with a hole in its leading colour slots | The pattern is skipped rather than filled with black | `pictura-codec::rgb_pattern_with_a_missing_colour_plane_is_skipped` |
| `uip_layers_mask_adjacency` | Snapshot a layer with a raster and a vector mask | Both mask thumbnails are present in the row | `tst_layers_snapshot::fixtureExpectations` |
| `uip_layers_active_thumb` | Click a mask thumbnail and inspect the row image | Brackets mark the clicked mask card and no group icon is bracketed | `tst_canvas_mask_domain::maskThumbnailUsesOnlyBrackets` |
| `uip_layers_link_glyph` | Render a masked row and measure the chain glyph ink | The glyph is larger than its slot and taller than wide | `tst_layers_panel_fixes::linkGlyphRotatedAndLarger` |
| `lpr_thumb_adjustment_icon` | Snapshot an Invert adjustment row | The row carries a neutral kind-icon card and no raster thumbnail | `tst_layers_snapshot::adjustmentThumbnailKindIcon` |
| `lpr_thumb_fill_preview` | Snapshot solid and gradient fill rows | Each row carries its content preview, not a checkerboard | `tst_layers_snapshot::fillThumbnailPreviews` |
| `lpr_thumb_pattern_fill_loaded` | Open the generated Photoshop-layout pattern-fill fixture with a painted mask and snapshot the Layers rows | The fill row's thumbnail is the tiled pattern (more than one colour, not the flat grey placeholder) and the mask thumbnail is present and opaque | `tst_layers_snapshot::patternFillThumbnailAndMask` |
| `lpr_thumb_adjustment_square` | Lay out adjustment and fill rows in a portrait document | The content cards are 1:1 while the mask cards follow the document aspect | `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspectOnSquareRows` |
| `lpr_thumb_fill_square` | Snapshot a fill row in a non-square document | The card and the preview image are square and the preview fills the card | `tst_layers_snapshot::fillThumbnailIsSquare` |
| `lpr_thumb_mask_square` | Lay out an adjustment or fill row with a mask in a portrait document | The mask card follows the document aspect while the content card stays square | `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspectOnSquareRows` |
