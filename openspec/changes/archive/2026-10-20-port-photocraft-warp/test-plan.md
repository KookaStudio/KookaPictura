# Test plan — Port Edit ▸ Transform ▸ Warp from photocraft

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `twe_identity` | Evaluate every preset at bend 0 | Each maps every point to itself | `pictura-core::presets_at_bend_zero_are_identity` |
| `twe_presets` | Map Photoshop-measured source points through each preset | Each lands within tolerance of the measured point | `pictura-core::presets_match_photoshop`, `pictura-core::preset_fits_closely_as_mesh` |
| `twe_split` | Split a bent mesh on both axes; invert a point | Surface unchanged within 1e-9; `param_at` inverts `eval` | `pictura-core::splits_are_exact`, `pictura-core::identity_mesh_is_exact_and_fit_reproduces_affine` |
| `twe_pull` | Pull the surface; drag anchors and handles | The grabbed point lands exactly; lines stay smooth | `pictura-core::pull_moves_the_grabbed_surface_point_exactly_and_stays_local`, `pictura-core::pull_keeps_the_surface_smooth_across_the_grid_lines`, `pictura-core::dragging_an_anchor_or_handle_keeps_the_surface_smooth_across_the_lines` |
| `twe_op_identity` | Run the op with none / identity mesh / Arc at 0 | Returns false; document unchanged | `pictura-render::identity_warps_are_refused_and_change_nothing` |
| `twe_op_affine` | Shift the box by (10, -2) with a custom mesh under three kernels | Rect shifted; channels byte-identical | `pictura-render::a_translating_mesh_moves_the_pixels_exactly` |
| `twe_op_arc` | Arc 50% on a raster layer | Bottom corners fixed; rect spreads past the top and sides | `pictura-render::arc_keeps_the_bottom_corners_and_spreads_the_top` |
| `twe_op_thin` | Stretch a 3 px column 100× into an arch | Middle lifted; under the arch empty | `pictura-render::a_thin_layer_stretched_into_a_curve_keeps_the_curve` |
| `twe_op_shrink` | Warp a 1 px line grid to ~1/5 size | Opaque pixels within 0.25 of 9/16 | `pictura-render::a_shrinking_warp_is_prefiltered` |
| `twe_op_fold` | Fold every column onto one | The right-most column shows | `pictura-render::a_fold_draws_the_later_cell_on_top` |
| `twe_op_mask` | Shift a masked layer by 5 px | Mask rect shifts; default and pixels kept | `pictura-render::the_layer_mask_moves_and_keeps_its_default` |
| `twe_op_native` | Warp a 16-bit layer with a retained store | Store rect = layer rect; planes sized; saves 16-bit | `pictura-render::depth16_warp_resamples_and_keeps_the_native_store` |
| `tsw_commit` | Enter Warp, drag the top-left anchor to (4, 2), Apply | Rect starts at (4, 2); one "Free Transform" state | `tst_transform_warp::anchorDragCommitsOneState` |
| `tsw_identity` | Enter Warp, commit untouched | No history state | `tst_transform_warp::identityWarpCommitsNothing` |
| `tsw_escape` | Enter Warp, pick Wave, press Escape | Rect and history unchanged | `tst_transform_warp::escapeRestoresAWarpedSession` |
| `tsw_menu` | Dispatch `edit.transform.warp` | Warp mode, `custom`; 4 boundaries, 4 guides, 12 points, 4 anchors | `tst_transform_warp::menuOpensWarpMode` |
| `tsw_toggle` | Click the warp toggle twice | Corner hit is an anchor, then a scale handle again | `tst_transform_warp::toggleSwitchesBetweenBoxAndWarp` |
| `tsw_carry` | Rotate 30°, enter Warp, leave Warp | Mesh corners = box corners; 30° box restored | `tst_transform_warp::boxTransformCarriesIntoTheMesh` |
| `tsw_grab` | Press anchor, handle, inside, far outside | Anchor, handle, surface, nothing | `pictura-app::a_press_grabs_a_point_the_surface_or_nothing`, `pictura-app::handles_belong_to_their_anchor` |
| `tsw_preset_press` | Bulge; press outside, then inside and drag | Outside grabs nothing; inside pulls, style becomes custom with the same shape | `tst_transform_warp::pressOnAPresetMakesItCustom`, `pictura-app::grabbing_a_preset_turns_it_into_the_same_custom_shape` |
| `tsw_preview` | Choose Arc | Preview image taller than the plain layer; no state | `tst_transform_warp::presetStyleAndValuesFromTheBar` |
| The locator moves the pivot | Click the bottom-right locator square | Pivot at the quad's bottom-right | `tst_transform_options::pivotLocatorMovesTheReferencePoint` |
| Apply commits and records one state | Change the angle, click Apply | One "Free Transform" state | `tst_transform_options::applyCommitsCancelRestores` |
| Cancel restores the document | Change the angle, click Cancel | Document unchanged | `tst_transform_options::applyCommitsCancelRestores` |
| The interpolation combo reaches the session | Choose Nearest Neighbor | Session reports `nearest` | `tst_transform_options::interpolationComboReachesTheSession` |
| `tob_warp_fields` | Enter Warp mode | Toggle checked; Warp fields shown; angle hidden | `tst_transform_warp::menuOpensWarpMode`, `tst_transform_warp::toggleSwitchesBetweenBoxAndWarp` |
| `tob_warp_values` | Choose Arc, type Bend -25 and H 10, toggle orientation, choose None | Arc, -25, 10, vertical; None disables Bend | `tst_transform_warp::presetStyleAndValuesFromTheBar` |
| Handles reveal near the pointer | Hover near a handle, then away; drag an anchor | Handles show near the pointer and while dragging | manual: a visual hover reveal on a real display; the point kinds are covered by `tst_transform_warp::menuOpensWarpMode` |
| Drag responsiveness on a large layer | Drag a warp anchor on a 4000×3000 layer | The preview follows the pointer | manual: a timing judgement on real hardware (`ponytail:` full-resolution preview) |
