# Test plan — Perspective transform gesture

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `perspective_corner` | Begin Perspective, drag the top-right corner mostly right, commit | Top corners spread symmetrically, bottom fixed, one state | `pictura-app::perspective_corner_spreads_its_edge`, `tst_transform_perspective::cornerSpreadsAndCommits` |
| `perspective_switch` | Drag a corner sideways, then vertically in the same drag | The vertical edge spreads and the horizontal edge is back at its press position | `pictura-app::perspective_repicks_the_edge_mid_drag` |
| `perspective_tilted` | Drag a corner of a box whose top edge is tilted | Both top corners move along the tilted line about its midpoint | `pictura-app::perspective_tilted_edge_spreads_along_itself` |
| `perspective_bowtie` | Drag a corner past its partner | The corners swap sides and the crossed quad is kept | `pictura-app::perspective_corners_cross_into_a_bowtie`, `tst_transform_perspective::cornersCrossIntoABowtie` |
| `perspective_edge` | Drag the top edge handle diagonally | Only the top corners move, along the top edge | `pictura-app::perspective_edge_slides_only_its_edge`, `tst_transform_perspective::edgeSlidesOnlyItsEdge` |
| `perspective_pivot` | Drag a corner with the pivot at the centre | Pivot equals the corner average | `pictura-app::perspective_carries_the_pivot_bilinearly` |
| `perspective_pivot_carry` | Spread one edge with the pivot at the centre | Pivot stays at the corner average | `pictura-app::perspective_carries_the_pivot_bilinearly`, `tst_transform_perspective::pivotStaysAtTheCornerAverage` |
| `perspective_type` | Begin Perspective on editable type | No session; document unchanged | `tst_transform_perspective::perspectiveIsRefusedForEditableType` |
| `perspective_menu_type` | Select editable type and look at Edit > Transform > Perspective | Disabled; triggering begins nothing | `tst_transform_perspective::perspectiveCommandDisabledForEditableType` |
| `free_perspective` | Ctrl+Alt+Shift-press a Free corner through the tool controller and drag sideways | The horizontal edge spreads symmetrically | `pictura-app::free_perspective_drag_matches_the_mode`, `tst_transform_perspective::freeShortcutDragsPerspective` |
| `free_perspective_scale` | After the shortcut drag, drag the right edge handle plainly | Left edge fixed, right corners move, side edges stay non-parallel | `pictura-app::free_scale_anchors_on_the_base`, `tst_transform_perspective::plainDragScalesThePerspectiveBox` |
| `free_perspective_commit` | Commit a Free session with a base | One state; the layer bounds follow the live quad | `pictura-app::free_base_composes_with_the_similarity`, `tst_transform_perspective::freePerspectiveCommits` |
| `free_perspective_type` | Ctrl+Alt+Shift-drag a corner on editable type | Ordinary scale, no base | `tst_transform_perspective::freeShortcutIgnoredOnType` |
| `perspective_readout` | Drag a 100 × 100 box's top-right corner 100 px right | Readout angle −45.0° | `pictura-app::perspective_readout_angle_signs`, `tst_transform_perspective::readoutShowsAndClears` |
| `perspective_readout_clear` | Release the drag | Readout empty | `tst_transform_perspective::readoutShowsAndClears` |
| A degenerate drag is refused | Drag a Perspective corner exactly onto its partner; pass a non-finite point | Last quad kept | `pictura-app::perspective_degenerate_quad_is_refused`, `pictura-app::degenerate_gesture_is_refused` |
| Distort commits the dragged corner | Grab a Distort corner, move, and commit | Corner at start plus delta; one state | `pictura-app::distort_corner_keeps_the_grab_offset`, `tst_transform_distort::cornerCommitUsesTheDelta` |
| `distort_grab_offset` | Press off the corner and drag | Corner moves by the delta | `pictura-app::distort_corner_keeps_the_grab_offset`, `tst_transform_distort::cornerKeepsTheGrabOffset` |
| `distort_convex` | Drag a corner across the diagonal | Quad stays convex | `pictura-app::distort_corner_stops_before_the_box_folds`, `tst_transform_distort::cornerStopsBeforeFolding` |
| `distort_pivot` | Drag a Distort corner | Pivot keeps its place within the box | `pictura-app::distort_carries_the_reference_point`, `tst_transform_distort::pivotFollowsTheCorner` |
| `distort_pivot_carry` | Drag a Distort corner | Pivot moves with the box | `pictura-app::distort_carries_the_reference_point`, `tst_transform_distort::pivotFollowsTheCorner` |
| `distort_guide_snap` | Drag a corner near a visible guide | Corner lands on the guide | `tst_transform_distort::cornerSnapsToVisibleGuides` |
| `distort_type` | Begin Distort on editable type | No session | `tst_transform_distort::distortIsRefusedForEditableType` |
| `distort_menu_type` | Trigger Distort on editable type | Disabled; no session | `tst_transform_distort::distortCommandDisabledForEditableType` |
| The Distort command enters a Distort session | Trigger Edit > Transform > Distort on a raster layer | Distort session | `tst_transform_distort::distortCommandEntersTheMode` |
| A non-transformable target leaves the command disabled | Trigger Distort on a group | Disabled; no session | `tst_transform_distort::distortCommandDisabledForGroup` |
| `skew_corner` | Drag a Skew corner | Only that corner moves along the chosen edge | `pictura-app::skew_corner_follows_its_dominant_edge`, `tst_transform_skew::cornerMovesAlongItsDominantEdge` |
| Skew slides one edge | Drag Skew edges diagonally | Each edge moves only along its axis | `pictura-app::skew_edge_slides_along_itself`, `tst_transform_skew::edgeSlidesOnlyAlongItself` |
| `skew_alt` | Drag a Skew corner and edge with Alt | Opposite moves the other way | `pictura-app::skew_alt_mirrors_the_opposite_corner_or_edge`, `tst_transform_skew::altMirrorsTheOppositeEdge` |
| `skew_type` | Drag a Skew corner on editable type | Parallelogram kept | `pictura-app::skew_keeps_editable_type_a_parallelogram`, `tst_type_tools::skewKeepsType` |
| `skew_convex` | Drag a Skew corner past the fold point | Quad stays convex | `pictura-app::skew_corner_stops_before_the_box_folds` |
| Escape cancels a projective mode exactly | Apply a projective gesture and cancel | Document and history unchanged | `tst_transform_options::applyCommitsCancelRestores` |
| Rotating about a moved pivot keeps it fixed | Move the pivot and layer, set the angle | Pivot pixel unchanged | `pictura-app::rotation_keeps_the_pivot_image_point_fixed` |
| Relative X/Y is an offset from the origin | Relative mode, set X | X is origin plus value | `tst_transform_options::relativePositioningOffsetsFromTheOrigin` |
| Link applies one factor to both axes | W 150 with link | Both scales 1.5 | `tst_transform_options::widthLinkScalesBothAxes` |
| H skew shears the layer | H 30° and commit | Vertical edges deviate 30° | `tst_transform_options::angleAndSkewSettersReachTheSession`, `pictura-app::h_shear_moves_the_left_edge` |
| `quad_bowtie` | Apply a quad with swapped top corners to an opaque 100 × 100 layer | Inside the hourglass opaque; beside it transparent | `pictura-render::crossed_quad_fills_an_hourglass`, `pictura-render::crossed_quads_are_detected`, `pictura-render::hourglass_round_trips` |
| `preview_bowtie` | Warp a white source through a homography onto a crossed quad | Hourglass drawn; beside and above it empty | `tst_transform_perspective::bowtiePreviewFillsAnHourglass` |
| An identity quad is the source rect | Apply a quad equal to the source corners | Layer unchanged | `pictura-render::quad_identity_is_bit_identical` |
| A translated quad matches the similarity translation | Apply an integer-translated quad | Equals the similarity translation | `pictura-render::quad_integer_translation_matches_similarity` |
| A perspective quad maps the corners onto the targets | Apply a non-affine quad | Corners land on the targets | `pictura-render::projective_quad_maps_source_corners_onto_targets` |
| Degenerate quads are refused | Apply collinear, non-finite, zero-area quads | Refused, bit-identical | `pictura-render::quad_degenerate_inputs_are_refused_bit_identically` |
| Pixels outside the source are transparent | Pull the destination outside the source | Outside pixels zero | `pictura-render::quad_out_of_source_pixels_are_zero` |
| `quad_front_side` | Apply a quad whose source straddles the horizon | Far side transparent | `pictura-render::quad_past_horizon_draws_only_the_front_side` |
| `preview_front_side` | Preview clip for a horizon-crossing matrix | Front side only | `tst_transform_distort::projectivePreviewClipsAtTheHorizon` |
| `distort_preview_full` | Distort a filled rectangle to a steep quad | Whole quad drawn | `tst_transform_distort::steepQuadKeepsTheWholeShape` |
| `distort_preview_layers` | Distort masked, vector-masked, clipped layers | Appearance kept | `tst_transform_distort::steepMaskedQuadKeepsTheWholeShape`, `tst_transform_distort::steepVectorMaskQuadKeepsTheWholeShape`, `tst_transform_distort::steepClippedFillQuadKeepsTheWholeShape` |
| `distort_preview_front` | Warp a source with a horizon | Front side only | `tst_transform_distort::projectivePreviewClipsAtTheHorizon` |
| `distort_preview_scale` | Warp a 1000-px layer at 10% | ~100 px a side | `tst_transform_distort::projectivePreviewSamplesAtTheViewScale` |
| `distort_preview_viewport` | Zoom out mid-preview | Newly visible parts drawn | `tst_transform_distort::projectivePreviewFollowsTheViewport` |
| `warp_preview_whole` | Distort past the box, then Arc warp | Outside content drawn | `tst_transform_distort::warpPresetPreviewKeepsContentOutsideTheBox` |
| `quad_bowtie_straight` | Map a lopsided crossed quad and test every pixel centre | Drawn exactly inside the two straight triangles | `pictura-render::a_lopsided_bowtie_fills_two_straight_triangles`, `pictura-render::hourglass_round_trips` |
| `preview_bowtie_straight` | Warp through a homography onto a lopsided crossed quad | Every off-edge pixel centre drawn iff inside a triangle | `tst_transform_perspective::lopsidedBowtiePreviewHasStraightEdges` |
| `lock_layer_menu` | Begin Free Transform; read Layer menu actions; dispatch New Layer; commit | Disabled and refused while pending; enabled after | `tst_transform_lock::layerMenuIsLockedWhilePending` |
| `lock_layers_panel` | Begin Perspective; read the Layers panel; cancel | Disabled while pending; enabled after | `tst_transform_lock::layersPanelIsGreyedWhilePending` |
| `switch_apply` | Move the box, choose Brush, answer Apply | One state, no session, Brush active | `tst_transform_lock::applyCommitsAndSwitches` |
| `switch_dont_apply` | Move the box, choose Gradient, answer Don't Apply | No state, no session, Gradient active | `tst_transform_lock::dontApplyCancelsAndSwitches` |
| `switch_cancel` | Choose Brush, answer Cancel | Session pending, old tool active, still locked | `tst_transform_lock::cancelKeepsTheTransform` |
| `switch_shortcut` | Press B with Cancel, then with Don't Apply | Asked each time; Cancel keeps, Don't Apply switches | `tst_transform_lock::toolShortcutAsksToo` |
| `switch_idle` | Choose Brush with no transform | Not asked; switches | `tst_transform_lock::noPromptWithoutAPendingTransform` |
| (manual) Prompt look and keys | Ctrl+T a layer, click the Brush tool | A warning box "Apply the transformation?" with Apply (default), Cancel (Esc), Don't Apply, as in Photoshop | manual: native modal dialog appearance and keyboard focus |
| (manual) Greyed panel look | Ctrl+T a layer and look at the Layers panel; try eye icons, drag, buttons | Panel greyed and unresponsive; restored after Enter or Esc | manual: visual greying |
| (manual) Perspective feel against CS6 | Drag each corner and edge handle of a textured layer in Perspective and with Ctrl+Alt+Shift in Free Transform; compare with the CS6 recording | Shapes, edge switching, bowtie, pivot, and readout match the recording | manual: subjective comparison with the owner's CS6 recording |
