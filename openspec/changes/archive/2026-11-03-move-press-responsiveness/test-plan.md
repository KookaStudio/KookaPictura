# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mvp_no_activation_bake` | Switch to Move on a duplicated layer, undo an edit, then press | No preview after the switch or the undo; the press builds one | `tst_move_feedback::switchingToMoveBuildsNoPreview` |
| `mvp_view_preview` | Build a view preview of a canvas-sized layer at level 2, at level 0 and off screen; drag one at 1/8 zoom in the app | Backdrop and moving image sized by the view and level; nothing for an off-screen layer; the drag commits its offset | `pictura_app::a_large_layer_previews_from_the_view_at_its_level`, `pictura_app::level_zero_composites_the_backdrop_exactly`, `pictura_app::a_layer_far_off_screen_starts_without_a_moving_image`, `tst_move_feedback::largeLayerPreviewsFromTheView` |
| `mvp_exact_fallback` | Ask for a view preview with no level, an empty rect, two layers, a 1 MP layer, and a styled document | None built; the styled document builds one at level 0 | `pictura_app::small_grouped_or_unviewed_moves_keep_the_exact_preview`, `pictura_app::a_document_a_reduction_would_misdraw_keeps_the_exact_preview_above_level_zero` |
| `mvp_faithful` | Check a plain document, one with an effects block, one with nested live type | Only the plain one reduces faithfully | `pictura-render::only_a_scale_free_document_reduces_faithfully` |
| `mvp_follow` | Drag a view preview inside its margin, then half a view further | No rebuild, then a rebuilt image covering the layer on screen | `pictura_app::a_drag_past_the_margin_rebuilds_the_moving_image` |
| `mvp_large_manual` | On a real display, a new 16507×16196 document with a duplicated background: switch to Move, drag at fit and at 100 % | No visible stall on the switch or the press; the layer shows its pixels while dragging | manual: offscreen Qt has no real compositor timing, and the document does not fit the test runners' memory |
| `ldr_undo_no_warm` | Switch to Move, undo an edit, press | Nothing built until the press | `tst_move_feedback::switchingToMoveBuildsNoPreview` |
