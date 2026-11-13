# Test plan

This plan covers `tools/move-press-responsiveness` and
`tools/move-commit-responsiveness` for `shape-move-preview-resolution` (#1190).

**Fixtures.** The Rust view-preview tests use a 2400 × 2000 document with a
background and a large rectangle shape, standing in for a 17000² document. The
Qt move test uses the same proportions at 1/8 zoom so the canvas presents level
3. The commit test uses a 4200² document so its dirty union crosses the 16 MP
deferral threshold.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mvp_no_activation_bake` | Switch to Move on a duplicated layer, undo an edit, then press | No preview after the switch or the undo; the press builds one | `tst_move_feedback::switchingToMoveBuildsNoPreview` |
| `mvp_view_preview` | Build a view preview of a canvas-sized layer at level 2, at level 0 and off screen; drag one at 1/8 zoom in the app | Backdrop and moving image sized by the view and level; nothing for an off-screen layer; the drag commits its offset | `pictura-app::a_large_layer_previews_from_the_view_at_its_level`, `pictura-app::level_zero_composites_the_backdrop_exactly`, `pictura-app::a_layer_far_off_screen_starts_without_a_moving_image`, `tst_move_feedback::largeLayerPreviewsFromTheView` |
| `mvp_shape_view` | Build a view preview of a large rectangle shape at level 2, then press and drag it at 1/8 zoom in the app | Backdrop is the background with the shape hidden; moving image is the shape over its reach at level 2; the ants box the reach; the drag commits its offset | `pictura-app::a_large_shape_previews_from_the_view_at_its_level`, `tst_move_feedback::largeShapePreviewsFromTheView` |
| `mvp_shape_stroke` | Build a view preview at level 2 of a large shape with a 20 px layer-effect stroke | The moving image is at level 2 and its stroke is about a quarter of its document width | `pictura-app::a_stroked_shape_preview_scales_its_effect` |
| `mvp_exact_fallback` | Ask for a view preview with no presented level, an empty rect, two layers, a 1 MP layer, or a styled layer elsewhere in the document above level 0 | None is built, and the styled document still builds one at level 0 | `pictura-app::small_grouped_or_unviewed_moves_keep_the_exact_preview`, `pictura-app::a_document_a_reduction_would_misdraw_keeps_the_exact_preview_above_level_zero` |
| `mvp_faithful` | Check a plain document, one with an effects block, one with nested live type, and one whose styled layer is hidden | Only the plain and hidden-styled documents reduce faithfully | `pictura-render::only_a_scale_free_document_reduces_faithfully` |
| `mvp_follow` | Drag a view preview inside its margin, then half a view further | No rebuild, then a rebuilt image covering the layer on screen | `pictura-app::a_drag_past_the_margin_rebuilds_the_moving_image` |
| `mvp_shape_follow` | Drag a shape view preview inside its margin, then carry its reach past the margin | No rebuild inside the margin; a rebuilt image around the shifted view past it | `pictura-app::a_shape_drag_past_the_margin_rebuilds_the_moving_image` |
| `mvp_large_manual` | On a real display, a 17000 × 17000 document with a shape over half the canvas: press and drag with the Move tool at fit and at 100 % | The press starts the drag without a visible stall, and the shape shows its pixels while dragging | manual: offscreen Qt has no real compositor timing, and the document does not fit the test runners' memory |
| `mcr_deferred` | Commit a canvas-sized layer on a document at or above 16 MP with a dirty union at or above 16 MP | The commit returns with the composite pending, and when it lands the composite equals a full composite and history gained one Move state | `tst_large_document::aLargeMoveCommitLandsOffThread` |
| `mcr_shape_deferred` | Commit a shape whose before/after reach union is at least 16 MP on a 4200² document | The commit returns with the shape moved and the composite pending, and when it lands history gains one `Move Layer` state | `tst_large_document::aLargeShapeMoveCommitLandsOffThread` |
