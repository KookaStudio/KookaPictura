# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Sources must match the target's dimensions | Open 20×20 twice and a 30×30, open the dialog | Active first, 30×30 absent | `tst_apply_image::sourcesMatchDimensions` |
| Composite is hidden across colour modes | Open a Grayscale source beside an RGB target | No composite in Channel | `tst_apply_image::compositeHiddenAcrossModes` |
| Scale and Offset follow the mode | Switch Blending | Shown only for Add and Subtract | `tst_apply_image::scaleOffsetFollowMode` |
| The mode list | Read Blending | No Dissolve, Multiply current, one Subtract | `tst_apply_image::modeList` |
| Add with Scale 2 averages | Add at Scale 2 | Average within 1/255 | `pictura-render::add_scale_two_averages` |
| Subtract honours Scale and Offset | Subtract at varied Scale and Offset | Scale lowers, Offset raises | `pictura-render::subtract_honours_scale_and_offset` |
| Layer modes use the shared blend | Each listed mode, opaque | Equals an independently composited layer | `pictura-render::layer_modes_match_the_compositor` |
| A single channel feeds every target channel | Source channel Red | Gray triplet of red | `pictura-render::single_channel_feeds_every_target_channel` |
| Opacity and Invert | Opacity 50 %, then Invert | Halfway, complement | `pictura-render::opacity_and_invert` |
| Preserve Transparency | Clear pixels, preserve on | Clear pixels and alpha unchanged | `pictura-render::preserve_transparency_keeps_alpha` |
| Transparent target takes the source | Clear pixel, preserve off | Opaque source colour | `pictura-render::clear_target_takes_the_source` |
| Selection as mask | Mask channel Selection | Selection coverage scales the blend | `tst_apply_image::selectionAsMask` |
| Inverted channel mask | Inverted mask plane | Applies where dark | `pictura-render::inverted_mask_applies_where_dark` |
| Active selection confines the result | Selection, no Mask | Only selected pixels change | `tst_apply_image::selectionConfines`, `pictura-render::selection_confines_the_result` |
| Cancel restores | Preview, Cancel | Unchanged, no state | `tst_apply_image::cancelRestores` |
| OK commits one state | Preview, OK, undo | One state, undo restores | `tst_apply_image::okCommitsOneState` |
| Merged from the active document does not compound | Change a control twice | Equals one application | `tst_apply_image::mergedDoesNotCompound` |
| Section matches a full apply | Run a section | Equal inside, unchanged outside | `pictura-render::section_matches_full_apply` |
| Mask target | Target a mask, OK | Mask changes, pixels do not | `tst_apply_image::maskTarget` |
| 16-bit keeps precision | Blend a 16-bit layer | Store finer than 8-bit, channels narrow it | `pictura-render::sixteen_bit_keeps_precision` |
| 32-bit does not clamp | Add over 1.0 | Store above 1.0 | `pictura-render::thirty_two_bit_does_not_clamp` |
| Cancel restores a native store | Preview on a 16-bit document, Cancel | Bit-identical | `tst_apply_image::cancelRestoresNativeStore` |
