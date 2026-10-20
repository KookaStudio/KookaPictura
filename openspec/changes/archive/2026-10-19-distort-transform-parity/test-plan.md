# Test plan — Distort transform parity

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| An identity quad is the source rect | Apply a quad equal to the source corners | Layer rect and planes are unchanged | `pictura-render::quad_identity_is_bit_identical` |
| A translated quad matches the similarity translation | Apply an integer-translated quad | Result equals the similarity translation | `pictura-render::quad_integer_translation_matches_similarity` |
| A perspective quad maps the corners onto the targets | Apply a non-affine quad | Source corners land on the targets | `pictura-render::projective_quad_maps_source_corners_onto_targets` |
| Degenerate quads are refused | Apply collinear, non-finite, and zero-area quads | Refused with the document bit-identical | `pictura-render::quad_degenerate_inputs_are_refused_bit_identically` |
| Pixels outside the source are transparent | Apply a quad pulling the destination outside the source | Outside pixels have zero alpha | `pictura-render::quad_out_of_source_pixels_are_zero` |
| `quad_front_side` | Apply a quad whose source straddles the horizon | Far-side pixels are transparent; the front side is drawn | `pictura-render::quad_past_horizon_draws_only_the_front_side` |
| `preview_front_side` | Build the preview clip for a matrix whose source straddles the horizon | The clip contains the front side and excludes the far side | `tst_transform_distort::projectivePreviewClipsAtTheHorizon` |
| Distort commits the dragged corner | Grab a Distort corner, move, and commit | The corner is at start plus delta; one state | `pictura-app::distort_corner_keeps_the_grab_offset`, `tst_transform_distort::cornerCommitUsesTheDelta` |
| `distort_grab_offset` | Press 3 px off the corner and drag 10 px | The corner moves 10 px, not to the pointer | `pictura-app::distort_corner_keeps_the_grab_offset`, `tst_transform_distort::cornerKeepsTheGrabOffset` |
| `distort_convex` | Drag a corner across the opposite diagonal | The quad stays convex and the other corners stay put | `pictura-app::distort_corner_stops_before_the_box_folds`, `tst_transform_distort::cornerStopsBeforeFolding` |
| `distort_pivot` | Drag a Distort corner | The pivot keeps its place within the box | `pictura-app::distort_carries_the_reference_point`, `tst_transform_distort::pivotFollowsTheCorner` |
| `distort_guide_snap` | Drag a corner near a visible guide | The corner and committed quad land on the guide | `tst_transform_distort::cornerSnapsToVisibleGuides` |
| `distort_type` | Begin Distort on editable type | No session begins; the document is unchanged | `tst_transform_distort::distortIsRefusedForEditableType` |
| Perspective mirrors the opposite corner | Drag a Perspective corner | The opposite corner moves by the negated delta | `pictura-app::perspective_mirrors_the_opposite_corner` |
| `skew_corner` | Drag a Skew corner mostly horizontally, then vertically | Only that corner moves along the chosen edge | `pictura-app::skew_corner_follows_its_dominant_edge`, `tst_transform_skew::cornerMovesAlongItsDominantEdge` |
| Skew slides one edge | Drag Skew top and left edges diagonally | Each edge moves only along its axis | `pictura-app::skew_edge_slides_along_itself`, `tst_transform_skew::edgeSlidesOnlyAlongItself` |
| `skew_alt` | Drag a Skew corner and edge with Alt | The opposite corner/edge moves the other way | `pictura-app::skew_alt_mirrors_the_opposite_corner_or_edge`, `tst_transform_skew::altMirrorsTheOppositeEdge`, `tst_transform_skew::altMirrorsTheOppositeCorner` |
| `skew_type` | Drag a Skew corner on editable type | The followed edge slides; the quad stays a parallelogram | `pictura-app::skew_keeps_editable_type_a_parallelogram`, `tst_type_tools::skewKeepsType` |
| `skew_convex` | Drag a Skew corner past the fold point | The stored quad stays convex | `pictura-app::skew_corner_stops_before_the_box_folds` |
| Escape cancels a projective mode exactly | Apply a projective gesture and cancel | Document and history are unchanged | `tst_transform_options::applyCommitsCancelRestores` |
| A degenerate drag is refused | Drag a Distort corner onto its neighbour, then pass a non-finite point | The quad stays convex; the non-finite gesture is refused | `pictura-app::degenerate_gesture_is_refused` |
| The Distort command enters a Distort session | Trigger `Edit > Transform > Distort` on a raster layer | A Distort session begins | `tst_transform_distort::distortCommandEntersTheMode` |
| `distort_menu_type` | Trigger Distort on editable type | The command is disabled and no session begins | `tst_transform_distort::distortCommandDisabledForEditableType` |
| A non-transformable target leaves the command disabled | Trigger Distort on a group | The command is disabled and no session begins | `tst_transform_distort::distortCommandDisabledForGroup` |
| Rotating about a moved pivot keeps it fixed | Move the pivot, move the layer, set the angle | The pixel at the pivot is unchanged | `pictura-app::rotation_keeps_the_pivot_image_point_fixed`, `tst_transform_options::perimeterRotatesAndFarPressDoesNotCommit` |
| Relative X/Y is an offset from the origin | Turn on relative mode and set X | X is the origin plus the value | `tst_transform_options::relativePositioningOffsetsFromTheOrigin` |
| Link applies one factor to both axes | Set W to 150 with the link on | Both scales are 1.5 | `tst_transform_options::widthLinkScalesBothAxes` |
| H skew shears the layer | Set H to 30° and commit | Vertical edges deviate 30°; one state | `tst_transform_options::angleAndSkewSettersReachTheSession`, `pictura-app::h_shear_moves_the_left_edge` |
| `distort_pivot_carry` | Drag a Distort corner | The pivot moves with the box and keeps its place | `pictura-app::distort_carries_the_reference_point`, `tst_transform_distort::pivotFollowsTheCorner` |
