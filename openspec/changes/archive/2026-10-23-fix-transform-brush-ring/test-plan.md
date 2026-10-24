# Test Plan

The ring is a hover affordance: the scenario is that a transform session takes
the canvas and the ring goes with it, then comes back when the session ends.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `bt_transform_clears` | Brush active with the ring drawn; open Free Transform | Outline state is cleared and the first transform frame no longer contains the ring pixel | `tst_transform_overlay::freeTransformClearsTheBrushRing` |
| `bt_transform_zoom` | Zoom the canvas while the transform session is open | No brush ring is drawn | `tst_transform_overlay::freeTransformClearsTheBrushRing` |
| `bt_transform_restores` | Cancel the session, then hover the canvas | The brush ring is drawn again at the pointer | `tst_transform_overlay::freeTransformClearsTheBrushRing` |
