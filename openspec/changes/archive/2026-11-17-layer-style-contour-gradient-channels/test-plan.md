# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A bevel and emboss descriptor decodes to typed parameters | Decode the stored bevel fields | The typed fields, highlight and shadow match | `pictura-render::bevel_decodes_typed_parameters` |
| Missing bevel and emboss keys take Photoshop defaults | Decode an `ebbl` with only `enab`/`present` | Photoshop defaults for every field | `pictura-render::bevel_missing_keys_take_defaults` |
| The style, technique and direction enums decode every value | Decode both enum sets | Each maps to its typed value | `pictura-render::bevel_enums_decode_every_value` |
| Out-of-range numeric values are clamped or rejected | Decode huge finite values and an overflowing value | Clamped bounds, then `None` | `pictura-render::bevel_huge_numeric_parameters_are_bounded_or_rejected` |
| A disabled bevel and emboss decodes but is inert | Decode `enab` false and composite | Decodes, composites byte-identically | `pictura-render::bevel_disabled_decodes_but_is_inert` |
| Malformed or absent bevel and emboss effects are a no-op | Decode absent/wrong-typed/wrong-enum effects | `None`, no panic | `pictura-render::bevel_malformed_or_absent_is_none` |
| The bevel is interior and leaves exterior pixels unchanged | Composite an Inner bevel against no effect | Outside unchanged, interior differs | `pictura-render::bevel_is_interior_and_exterior_is_unchanged` |
| Angle and altitude shape the highlight and shadow | Composite with two angles, then two altitudes | Composites differ | `pictura-render::bevel_angle_and_altitude_shape_the_shading` |
| Size, soften and depth shape the shading | Composite with small/large size, soften and depth | Larger spans more, depth 0 is a no-op | `pictura-render::bevel_size_soften_and_depth_shape_the_shading` |
| Direction flips the bevel | Composite `Up` then `Down` | Highlight and shadow exchange | `pictura-render::bevel_direction_flips_the_highlight_and_shadow` |
| Highlight and shadow colour, opacity and mode shape the bevel | Composite with colours, opacities and modes | Opacity approaches plain content; modes differ | `pictura-render::bevel_highlight_and_shadow_colour_opacity_mode_shape_it` |
| The layer mask shapes the matte | Mask half the canvas | Bevel absent where the mask is zero | `pictura-render::bevel_layer_mask_shapes_the_matte` |
| The bevel is bounded to the content rect plus reach | Composite on a larger canvas | Outside the padded rect unchanged | `pictura-render::bevel_is_bounded_to_the_content_rect` |
| Each style lights its side of the edge | Composite all four styles | Inner/Outer/Emboss/Pillow change the right regions | `pictura-render::each_bevel_style_lights_its_side_of_the_edge` |
| A deferred style or technique is a no-op | Composite the Stroke style | Byte-identical to no effect | `pictura-render::absent_disabled_or_stroke_emboss_bevel_is_a_no_op` |
| Each technique rounds the chamfer differently | Composite Smooth, ChiselHard and ChiselSoft | The three differ | `pictura-render::each_technique_rounds_the_chamfer_differently` |
| A disabled or absent bevel is a no-op | Composite absent, disabled and depth 0 | Byte-identical to no effect | `pictura-render::absent_disabled_or_stroke_emboss_bevel_is_a_no_op`, `pictura-render::bevel_size_soften_and_depth_shape_the_shading` |
| Opening on an effect and cancelling | Open on Color Overlay, set Opacity 0, cancel | Row checked, red until 0, style and history unchanged | `tst_layer_style::dialogOpensOnItsEffectAndCancelDropsTheEdits` |
| Pattern Overlay picks a built-in pattern | Set the second pattern | Checkerboard shows; pattern reads 1 | `tst_layer_style::patternOverlayPicksABuiltInPattern` |
| OK records one state | Move Size twice, OK, then accept an idle dialog | One state; 2 px stroke; idle adds none | `tst_layer_style::okRecordsOneState` |
| `bevel-contour-decode` | Decode a bevel whose `TrnS`/`MpgS` carry custom points and `useShape` true | Both point lists, `edge_contour_on`, `Inpr` and `AntA` decode | `pictura-render::bevel_contours_decode` |
| `bevel-contour-defaults` | Decode a bevel with no contour keys | Both contours linear, edge contour off, range 50, anti-alias false | `pictura-render::bevel_contours_default_to_linear` |
| `bevel-contour-malformed` | Decode empty, single-point, non-finite and wrong-typed contours | Each falls back to linear; the bevel still decodes; no panic | `pictura-render::bevel_contour_malformed_is_linear` |
| `bevel-gloss-contour` | Composite a bevel with a linear gloss contour, then a bent one | Composites differ inside the band; outside is byte-identical | `pictura-render::gloss_contour_remaps_the_shading` |
| `bevel-edge-contour` | Composite a bevel with a linear edge contour, then a bent one | Composites differ inside the band | `pictura-render::edge_contour_remaps_the_profile` |
| `bevel-edge-contour-off` | Composite the same bent contour with `useShape` false and true | False matches the linear composite; true differs | `pictura-render::edge_contour_off_is_a_no_op` |
| `bevel-contour-range` | Composite a bent contour at range 100 and a smaller range | Composites differ and stay bounded to content plus reach | `pictura-render::contour_range_scales_the_profile` |
| `bevel-contour-linear-noop` | Compare a linear-contour bevel with the edge contour on and the default range against edge-off | Byte-identical composites | `pictura-render::linear_contours_are_byte_identical` |
| `contour-author-round-trip` | Set `bevel.glossContour` to `0,0 64,128 255,255`, read and decode | The list round-trips and decodes with the same points | `pictura-render::contour_text_round_trips` |
| `contour-author-default` | Read `bevel.edgeContour` on a plain layer | `0,0 255,255` | `pictura-render::absent_contour_reads_linear` |
| `contour-author-refused` | Write one point, duplicate inputs, 256, a non-number, a group and the Background | Each write is refused and the contour is unchanged | `pictura-render::bad_contour_text_is_refused` |
| `contour-preserves-point-items` | Rewrite a point carrying an extra `CrPt` item | The extra item survives; only `Hrzn`/`Vrtc` change | `pictura-render::contour_edit_preserves_point_items` |
| `contour-sub-panel-fields` | Set/read `bevel.edgeContourOn`, `edgeRange`, `edgeAntiAlias`; decode | 1/30/1 round-trip; `useShape`/`Inpr`/`AntA` match | `pictura-render::bevel_sub_panel_fields_round_trip` |
| `contour-antialias-default` | Create a bevel through a contour write and read `bevel.antiAlias` | Reads 1; `antialiasGloss` is true | `pictura-render::bevel_gloss_antialias_defaults_on` |
| `gradient-author-round-trip` | Set a three-stop `gradientOverlay.gradient`, read and decode | The text and the decoded stops match | `pictura-render::gradient_text_round_trips` |
| `gradient-author-preserves-opacity` | Rewrite colour stops on a gradient with non-linear `Trns` and a non-default `Mdpn` | `Trns` and `Mdpn` unchanged; composited alpha unchanged; RGB changes | `pictura-render::gradient_edit_preserves_opacity_stops` |
| `gradient-author-refused` | Write one stop, a location above 4096, a flat location, bad hex, eight-digit hex, a group or the Background | Each write is refused and the gradient is unchanged | `pictura-render::bad_gradient_text_is_refused` |
| `authored-style-survives-save` | Author a contour and a three-stop gradient; save and reopen | The reopened points/stops and composite match | `pictura-render::authored_contour_and_gradient_survive_save` |
| `channel-include-toggle` | Set `blending.include1` 0 then 1 | The restriction is added then removed, each reporting a change | `pictura-render::channel_include_toggles_the_restriction` |
| `channel-include-default` | Read `blending.include0..2` on a plain layer | Each reads 1 | `pictura-render::channels_are_included_by_default` |
| `channel-include-group` | Set `blending.include2` 0 on a group, then an effect key | Restriction contains 2; the effect key stays refused | `pictura-render::group_accepts_channel_include` |
| `channel-include-copy-paste` | Copy a restricted style and paste it onto another layer | The target's restrictions match the source's and the composite reflects them | `pictura-render::channel_exclude_survives_copy_and_paste` |
| `brst-decode` | Read a layer with a two-index `brst` | Restrictions are the indices; no `brst` in `extra_blocks` | `pictura-codec::brst_decodes_channel_restrictions` |
| `brst-default` | Read a layer without `brst` | Restrictions are empty | `pictura-codec::absent_brst_includes_every_channel` |
| `malformed-brst-dropped` | Read a three-byte `brst`, then save | Read succeeds, restrictions empty, no `brst` in the saved file | `pictura-codec::malformed_brst_is_dropped` |
| `brst-round-trip` | Write restrictions 1 and 2, reopen | `brst` lists both indices; the reopened layer reads them | `pictura-codec::channel_restrictions_round_trip` |
| `brst-blend-excludes` | Composite red over blue excluding red | Red channel equals the backdrop; green and blue take the layer | `pictura-render::excluded_channel_keeps_the_backdrop` |
| `brst-blend-default` | Composite a stack with no restrictions | Byte-identical to the same stack before the feature | `pictura-render::empty_channel_restrictions_are_a_no_op` |
| `brst-blend-effects` | Composite a bevel/overlay on a channel-excluded layer | Excluded channel stays at the entry backdrop; effect shows elsewhere | `pictura-render::excluded_channel_survives_effect_passes` |
| `brst-blend-group` | Composite a pass-through group with a restriction | Excluded channel equals the group-entry backdrop | `pictura-render::excluded_channel_survives_a_pass_through_group` |
| `brst-blend-translucent` | Composite over a semi-transparent backdrop | Normal output alpha; excluded channel is the entry straight value | `pictura-render::excluded_channel_keeps_a_translucent_backdrop` |
| `brst-gpu-declines` | Plan a stack containing an excluded-channel layer | `UnsupportedAdvancedBlending`; CPU result | `pictura-render::gpu_declines_a_channel_excluded_layer` |
| `dialog-contour-editor` | Open Bevel & Emboss, pick a bent preset, accept, cancel the style dialog | Bevel changes, then reverts; no history state | `tst_layer_style::contourPresetChangesTheBevelAndCancelDropsIt` |
| `dialog-gloss-contour` | Set Gloss Contour to a bent curve and accept | The effect reads the points and the canvas shading differs | `tst_layer_style::glossContourWritesTheEffect` |
| `dialog-contour-gate` | Uncheck the Contour checkbox | Contour, Range and Anti-aliased are disabled; render matches contour off | `tst_layer_style::contourCheckboxGatesTheSubControls` |
| `dialog-gradient-editor` | Open the Gradient Overlay editor, set three stops, accept | Swatch and effect stops update; canvas recomposites | `tst_layer_style::gradientEditorWritesTheOverlayStops` |
| `dialog-gradient-cancel` | Open the editor, change a stop, cancel | The gradient is unchanged | `tst_layer_style::gradientEditorCancelKeepsTheGradient` |
| `dialog-gradient-strict` | Drag two stops onto the same location | Locations stay strictly rising; the accepted list is valid | `tst_layer_style::gradientEditorKeepsLocationsStrict` |
| `dialog-gradient-simple` | Open the Gradient Overlay editor | Solid only; Noise and Smoothness unavailable | `tst_layer_style::gradientEditorHidesNoiseAndSmoothness` |
| `dialog-gradient-opaque` | Accept the Foreground to Transparent preset | The effect's stops are six-digit opaque and the swatch matches | `tst_layer_style::gradientEditorPresetStaysOpaque` |
| `dialog-copy-channel-only` | Give a layer only an Include restriction | Copy is enabled, paste carries the restriction, Clear stays disabled | `tst_layer_style::copyEnabledForChannelOnlyStyle`, `pictura-render::channel_exclude_survives_copy_and_paste` |
| `dialog-channel-include` | Uncheck the second channel on Blending Options, then cancel | The composite loses and then regains the channel; no history state | `tst_layer_style::channelIncludeChangesTheCompositeAndCancelRestores` |
| `dialog-channel-modes` | Open Blending Options on RGB, grayscale and CMYK documents | RGB shows three checkboxes; grayscale and CMYK show none | `tst_layer_style::channelRowFollowsTheDocumentMode` |
