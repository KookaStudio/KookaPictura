# Test plan: type-layer-move-locks

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Drag moves the selected pixels | Select pixels on a pixel layer and drag with the Move tool. | The covered pixels move by the offset and the source area is cleared. | `tst_mask_edit::moveSelectionOnMaskMovesCoverage` |
| The selection follows the pixels | Commit a selected-pixel drag by `(dx, dy)`. | The selection mask is translated by the same offset. | `tst_canvas_mask_domain::selectedMaskMoveRoutesCoverage` |
| No selection still moves the layer | Drag a layer with no selection. | The whole layer moves. | `tst_move_feedback::dragShowsAntsAndReadout` |
| `scm_type_whole` | Make a type layer active with a selection over it, then Move-drag by (36, 18). | The whole type layer previews and lands 36 × 18 away; the selection stays; one `Move Layer` state. | `tst_move_feedback::typeLayerMovesWholeWithASelection` |
| `scm_locked_clean` | Position-lock a type layer with and without a selection, then press and drag. | No Move preview starts, the layer rect is unchanged, and no history state is added. | `tst_move_feedback::positionLockedTypeLayerRefusesCleanly` |
| A plain pixel layer keeps the content move | Select pixels on an unlocked pixel layer and Move-drag. | The selection-content move still runs and records one `Move Selection` state. | `tst_move_feedback::pixelSelectionStillContentMoves` |
| `move_drag_feedback` | Drag a type layer 36 px right and 18 px down at 72 ppi in inches, then release. | The ants box the text and the readout matches; the type layer lands at the offset. | `tst_move_feedback::dragShowsAntsAndReadout` |
| `move_drag_units` | Read a 36 × 72 px offset in pixels, centimetres, and percent. | The readout uses each ruler unit. | `tst_move_feedback::readoutFollowsTheRulerUnit` |
| `move_drag_type_ink` | A paragraph type layer whose box fills the canvas, moved by the Move tool. | The moving crop and the damage are the ink bounds, not the paragraph box. | `pictura-app::a_type_move_previews_and_damages_its_ink_not_the_whole_box` |
