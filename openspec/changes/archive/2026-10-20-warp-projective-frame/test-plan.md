# Test plan — Keep the box's projective frame in Warp mode

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tsw_commit` | Enter Warp, drag the top-left anchor to (4, 2), Apply | Rect starts at (4, 2); one "Free Transform" state | `tst_transform_warp::anchorDragCommitsOneState` |
| `tsw_identity` | Enter Warp, commit untouched | No history state | `tst_transform_warp::identityWarpCommitsNothing` |
| `tsw_escape` | Enter Warp, pick Wave, press Escape | Rect and history unchanged | `tst_transform_warp::escapeRestoresAWarpedSession` |
| `tsw_menu` | Dispatch `edit.transform.warp` | Warp mode, `custom`; 4 boundaries, 4 guides, 12 points, 4 anchors | `tst_transform_warp::menuOpensWarpMode` |
| `tsw_toggle` | Click the warp toggle twice | Corner hit is an anchor, then a scale handle again | `tst_transform_warp::toggleSwitchesBetweenBoxAndWarp` |
| `tsw_carry` | Rotate 30°, enter Warp, leave Warp | Mesh corners = box corners; 30° box restored | `tst_transform_warp::boxTransformCarriesIntoTheMesh` |
| `tsw_carry_projective` | Show the identity mesh through a perspective frame; warp a gradient through a perspective quad | Edge midpoint at the homography's point; interior samples match the inverse homography | `pictura-app::the_grid_follows_a_perspective_box_exactly`, `pictura-app::a_point_drag_follows_the_pointer_through_the_frame`, `pictura-render::a_projective_box_is_kept_exactly` |
| `tsw_release` | Drag the top-left anchor on the canvas, then release | Two dragged handles during the drag, none after | `tst_transform_warp::releaseHidesTheDraggedHandles` |
| `tsw_grab` | Press anchor, handle, inside, far outside | Anchor, handle, surface, nothing | `pictura-app::a_press_grabs_a_point_the_surface_or_nothing`, `pictura-app::handles_belong_to_their_anchor` |
| `tsw_preset_press` | Bulge; press outside, then inside and drag | Outside grabs nothing; inside pulls, style becomes custom with the same shape | `tst_transform_warp::pressOnAPresetMakesItCustom`, `pictura-app::grabbing_a_preset_turns_it_into_the_same_custom_shape` |
| `tsw_preset_hidden` | Bulge 50%; press each control point off its surface | Each press grabs nothing | `pictura-app::a_preset_hides_its_points_from_the_press` |
| `twe_op_identity` | Run the op with none / identity mesh / Arc at 0 | Returns false; document unchanged | `pictura-render::identity_warps_are_refused_and_change_nothing` |
| `twe_op_affine` | Shift the box by (10, -2) with a custom mesh under three kernels | Rect shifted; channels byte-identical | `pictura-render::a_translating_mesh_moves_the_pixels_exactly` |
| `twe_op_arc` | Arc 50% on a raster layer | Bottom corners fixed; rect spreads past the top and sides | `pictura-render::arc_keeps_the_bottom_corners_and_spreads_the_top` |
| `twe_op_thin` | Stretch a 3 px column 100× into an arch | Middle lifted; under the arch empty | `pictura-render::a_thin_layer_stretched_into_a_curve_keeps_the_curve` |
| `twe_op_shrink` | Warp a 1 px line grid to ~1/5 size | Opaque pixels within 0.25 of 9/16 | `pictura-render::a_shrinking_warp_is_prefiltered` |
| `twe_op_fold` | Mirror the left half onto the right half's strip | The right half shows | `pictura-render::a_fold_draws_the_later_cell_on_top` |
| `twe_op_mask` | Shift a masked layer by 5 px | Mask rect shifts; default and pixels kept | `pictura-render::the_layer_mask_moves_and_keeps_its_default` |
| `twe_op_native` | Warp a 16-bit layer with a retained store | Store rect = layer rect; planes sized; saves 16-bit | `pictura-render::depth16_warp_resamples_and_keeps_the_native_store` |
| `twe_op_projective` | Warp a gradient layer with an identity mesh and a perspective quad | Four interior columns within 3 levels of the inverse homography | `pictura-render::a_projective_box_is_kept_exactly` |
| `twe_op_mask_beyond` | Shift a layer whose mask reaches 6 px past it | Whole mask rect shifts; pixels kept | `pictura-render::a_mask_wider_than_the_layer_moves_with_it` |
| `twe_op_mask_refuse` | Shift a layer 1000 px whose mask reaches the coordinate limit | Returns false; document unchanged | `pictura-render::a_mask_that_cannot_follow_refuses_the_warp` |
| `twe_op_area` | Map a 4 px box onto 65536 px square | Triangulation and op refuse; document unchanged | `pictura-render::a_warp_past_the_result_area_ceiling_is_refused` |
