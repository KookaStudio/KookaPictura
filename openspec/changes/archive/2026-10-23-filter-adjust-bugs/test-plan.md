# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A large picture previews at pane resolution | Open the gallery on a 4000×3000 document in a narrow pane | Preview is narrower than 1000 device pixels and keeps 4:3 | `tst_filter_gallery::largePicturePreviewsReduced` |
| Previewing leaves the document alone | Preview an effect and cancel | Canvas and history are unchanged | `tst_filter_gallery::stackPreviewsInThePaneOnly` |
| Full-scale copy matches the commit | Preview at scale 1 over the whole document, then commit | Preview equals the committed composite | `pictura-render::full_scale_proxy_renders_like_the_document` |
| `fgp_pixel_scale` | Gaussian Blur radius 8 at a quarter-scale proxy; Mosaic Tiles 12 at half scale in the shared dialog | Proxy uses radius 2 and matches the full render downsampled; Mosaic Tiles uses 6 and Seed is unchanged | `tst_filter_gallery::previewScalesPixelParams`, `tst_filter_menu::scalesUnsuffixedPixelParams` |
| `fgp_thumbnail_scale` | Render the Mosaic Tiles catalog thumbnail for a 3000×2400 document | The effective 80×56 cover scale is max(80/3000, 56/2400) = 0.0267; Tile Size 12 × 0.0267 rounds to 0 then clamps to 2; Grout Width 3 × 0.0267 rounds to 0 then clamps to 1; Seed is unchanged | `tst_filter_gallery::thumbnailScalesPixelParams` |
| A re-preview elsewhere restores the first section | Preview a section, preview another, cancel | Layer equals a fresh preview of the second section; both refreshes reported | `pictura-app::section_previews_restore_only_what_they_wrote` |
| The live planes are written in place | Run and cancel two section previews | Planes keep allocations and are bit-identical to before | `pictura-app::section_previews_write_the_live_planes_in_place` |
| A commit after a section preview is exact | Preview a section, then commit the same filter | Layer equals a fresh commit | `pictura-app::section_previews_restore_only_what_they_wrote` |
| `fdp_refused_restore` | Make Wave Wavelength Max below Min+1, wait, then Cancel | Canvas and composite equal pre-preview pixels; no preview session remains | `tst_filter_menu::refusedPreviewRestoresCanvas` |
| A large zoomed-out document previews on the copy | Preview Add Noise on a 3000×2400 document at 10% zoom | Canvas shows the copy; document is unchanged until OK | `tst_filter_menu::zoomedOutPreviewDrawsAReducedCopy` |
| Pixel parameters scale with the copy | Scale a quarter-size copy's values | Gaussian 5→1.25; Unsharp amount stays 150; Median 2→1 | `tst_filter_menu::pixelParametersScaleWithTheCopy` |
| `fdp_unsuffixed_scale` | Scale Smart Blur, Mosaic, Crystallize, Pointillize, and Color Halftone values by 0.25 | Each value scales by 0.25 and clamps to its range; integer controls round | `tst_filter_menu::scalesUnsuffixedPixelParams` |
| The pane shows its section exactly | Render the pane section while the canvas shows the copy | Pane section equals a section preview; document unchanged | `pictura-app::an_open_section_preview_does_not_leak_into_the_copy` |
| A layer mask target keeps the exact preview | Preview while the active target is a layer mask | No reduced copy is made; mask previews exactly | `pictura-app::a_mask_target_has_no_proxy` |
| `fdp_whole_layer` | Preview Radial Blur, Clouds, Difference Clouds, Fibers with a cropped viewport | Whole layer is filtered; visible region matches the commit | `pictura-app::section_preview_escalates_radial_clouds_fibers` |
| Transparency is bit-identical | Run value kernels on an opaque or partly transparent layer | Alpha plane is bit-identical | `pictura-render::value_kernels_leave_alpha_bit_identical` |
| A moving kernel keeps an opaque layer opaque | Run moving kernels on an opaque layer | Alpha stays 255 everywhere | `pictura-render::moving_kernels_keep_an_opaque_layer_opaque` |
| Offset moves the layer edge | Offset a half-transparent layer right with repeat and wrap off | Transparent half moves; exposed left is opaque | `pictura-render::offset_moves_the_layer_edge_and_fills_what_it_exposes_opaque` |
| Offset can expose transparency | Offset with repeat and wrap off in Set to Transparent mode | Exposed alpha is 0; colour takes the Background colour | `pictura-render::offset_in_transparent_mode_exposes_clear_pixels` |
| Every moving kernel moves a soft layer edge | Run a moving kernel on a soft-edged disc | Alpha moves with the colour | `pictura-render::moving_kernels_move_a_soft_layer_edge` |
| Picture Frame may add opacity | Run Picture Frame over a clear pixel | The border adds opacity there | `pictura-render::picture_frame_adds_opacity_over_a_clear_pixel` |
| Layer metadata is unchanged | Apply a filter successfully | `rect`, `mask`, `opacity`, and `blend` are unchanged | `pictura-render::gaussian_blur_changes_interior_leaves_alpha_and_meta` |
| `fa_offset_repeat_alpha` | Offset in Repeat Edge Pixels mode on a soft-edged alpha | Exposed alpha repeats the nearest edge alpha | `pictura-filters::offset_repeat_edge_pixels_clamps` |
| `op_shine_range` | Open Oil Paint; drag Shine to maximum | Range 0..10, opening 5.5, committed value 10 | `tst_filter_menu::oilPaintShineRange` |
| Wrap re-enters on the opposite side | Offset with wrap true, repeat false, and horizontal equal to width | Output equals input | `pictura-filters::offset_wraps_and_fills_background_with_known_values` |
| Background fills the exposed area | Offset with wrap/repeat false and a distinct background | Exposed colour is the background; exposed alpha 255 | `pictura-filters::offset_wraps_and_fills_background_with_known_values` |
| Transparent fills the exposed area clear | Offset with wrap/repeat false and transparent true | Exposed colour is the background; exposed alpha 0 | `pictura-render::offset_in_transparent_mode_exposes_clear_pixels` |
| Zero offset is a no-op | Offset 0/0 in every edge mode | Buffer is bit-exactly unchanged | `pictura-filters::offset_vertical_shift_and_zero_is_noop` |
| Background fill has no faithful equivalent | Check the Offset oracle mapping row | The row records `-roll` for wrap and its note says background-fill and Repeat Edge Pixels have no ImageMagick operator (tolerance 0) | `pictura-filters::offset_mapping_documents_the_fill_and_repeat_cases` |
| `of_repeat_edge` | Offset with repeat true and a positive shift | Exposed pixels clamp to the nearest edge; interior translates | `pictura-filters::offset_repeat_edge_pixels_clamps` |
| A 1x1 image does not panic | Apply every Other variant to a 1×1 buffer | No variant panics; each returns Ok or a FilterError | `pictura-filters::tiny_images_and_oversized_radii_do_not_panic` |
| Border pixels clamp to the edge | Apply Maximum, Minimum, High Pass, or Custom to a small buffer and inspect a border pixel | The result is consistent with repeating the edge sample | `pictura-filters::maximum_brightens_and_minimum_darkens_known_values`, `pictura-filters::high_pass_flat_image_is_mid_gray`, `pictura-filters::custom_three_by_three_mean_matches_known_values` |
| Auto Tone stretches the range | Auto Tone on a 60..200 ramp | Dark end near 0, light end near 255 | `pictura-adjust::auto_tone_stretches_per_channel` |
| Auto Contrast does not introduce a cast | Auto Contrast on a neutral ramp | R=G=B preserved | `pictura-adjust::auto_contrast_joint_no_cast` |
| Auto Color pulls channels toward neutral | Auto Color on a fixed-channel-offset ramp | Per-channel sums move together | `pictura-adjust::auto_color_neutralizes_midtones` |
| `ia_auto_selection` | Two-ramp image, select the left half, run Auto Tone/Contrast/Color | Selected ramp uses only selected pixels; right ramp unchanged | `pictura-render::auto_tone_uses_selection_histogram` |
| Neutral parameters are identity | Vibrance 0, saturation 0 | Buffer is bit-exactly unchanged | `pictura-adjust::vibrance_identity_and_diminishing_boost` |
| Saturation minus 100 is grey | Saturation -100 | R=G=B | `pictura-adjust::vibrance_saturation_minus_100_is_gray` |
| `ia_vibrance_neutral` | 50% gray, vibrance 100, saturation 0 | Output is the same gray, no hue | `pictura-adjust::vibrance_neutral_pixel_stays_neutral` |
| A 16-bit Levels edit keeps 16-bit precision | 16-bit Levels edit whose 8-bit narrowing is unchanged | At least one low byte differs from its high byte | `pictura-adjust::depth16_native_edit_is_not_the_widened_byte` |
| Native u8 equals the existing apply | Apply each covered adjustment to a fixed u8 image through apply and the native entry | Outputs are byte-identical | `pictura-adjust::native_u8_equals_apply_for_every_covered_adjustment` |
| The ImageMagick oracle is unmoved | Run the adjust oracle tests after the changes | No golden or tolerance change | `pictura-adjust::mapping_marks_no_equivalent_operators` |
| A 32-bit store is adjusted in unit space | Exposure on a depth-32 store | Values above 1.0 stay representable | `pictura-adjust::depth32_native_edit_keeps_values_above_one` |
| `nda_exposure_linear` | Depth-32 linear 0.5, Exposure +1 EV | Output is 1.0 (the input is clamped by `to_unit`; an output above 1.0 is still representable) | `pictura-adjust::native_exposure_linear` |
| `nda_posterize_native` | Depth-16 Posterize 5, then 255 | 0/16384/32768/49151/65535; 255 is identity | `pictura-adjust::native_posterize_16bit` |
| `nda_bw_cm_native` | Depth-16 Black & White with weights that drive the grey past its range, and a non-identity Channel Mixer | The Black & White grey clamps at the store's range before tinting; in either case at least one low byte is not constrained to the 8-bit widen | `pictura-adjust::native_bw_channel_mixer` |
| `fpd_dialog_defaults` | Open each shared-dialog kind | Controls, order, ranges, choices, and opening values match the descriptor | `tst_filter_menu::everyRowOpensOnItsDefaults` |
| `fpd_mapping_defaults` | Map each kind with an empty list and with the descriptor defaults | The two filters are equal | `pictura-app::mapping_defaults_come_from_the_descriptor` |
| `fpd_commit_defaults` | Map and commit Lens Blur from its opening values | Mapped filter opens on Faster; committed slots carry More Accurate | `tst_lens_blur_dialog::fasterStillCommitsMoreAccurate` |
| `fpd_cpp_rows` | Build the filter menu | Rows match the descriptor, including pixel-distance flags | `tst_filter_menu::rowsComeFromTheRustDescriptor` |
| `fpd_whole_layer` | Ask each kind whether it resolves against the whole layer | The answer matches its descriptor | `pictura-app::whole_layer_comes_from_the_descriptor` |
| `fpd_dialog_kind` | Choose each kind from the filter menu | The named dialog (or the shared dialog) opens | `tst_filter_menu::eachKindOpensTheDialogItsDescriptorNames` |
