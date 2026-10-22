# Test plan — Move and transform shape layers as vectors

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `slt_move` | Add a rectangle shape; Move tool drag from inside it by (20, 15); undo | Fill moved; still a shape; one `Move Layer` state; controls box follows; undo restores | `tst_shape_transform::moveToolMovesTheShape` |
| `slt_move_live` | `translate_layer_path` a live rectangle by (3, 5) | Outline and live bounds move by (3, 5); layer rect unchanged | `pictura-render::a_shape_layer_moves_and_transforms_its_outline` |
| `slt_lock` | Position-lock a shape; translate it, then transform it by a quad | Both return false; document unchanged | `pictura-render::a_position_locked_shape_layer_does_not_move` |
| `slt_rotate` | Dispatch Rotate 90° Clockwise on a shape over (10, 10)–(30, 20) | One state; still a shape; box (15, 5)–(25, 25) | `tst_shape_transform::transformCommandsMapTheOutline` |
| `slt_free` | Begin Free Transform on the shape; set width 200%; commit | Session box is the outline; box doubles in width about its centre | `tst_shape_transform::transformCommandsMapTheOutline` |
| `slt_quad_warp` | `transform_layer_quad`, then `transform_layer_warp` with a moved frame | Outline box lands on each quad; no pixel channels | `pictura-render::a_shape_layer_moves_and_transforms_its_outline` |
| `slt_target` | `can_free_transform` on a shape, then on it position-locked | True, then false | `pictura-app::a_shape_layer_is_a_transform_target_unless_position_locked` |
| `lsb_one_thumbnail` | Add a rectangle shape; take the Layers snapshot at 32 px | Shape row with a thumbnail; null vector-mask and mask thumbnails | `tst_shape_transform::shapeRowShowsOneThumbnail` |
| Visual check | Draw a shape in the app; look at its Layers row; drag it with Move; Ctrl+T rotate it | One thumbnail with the shape badge; shape moves and rotates | manual: pixel-level look of the panel row and live drag feel |
