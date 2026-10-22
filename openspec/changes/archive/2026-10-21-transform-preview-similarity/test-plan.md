# Test plan — Draw the layer under a Free Transform rotate or scale

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ftp_similarity` | Begin Free Transform on a shape, set 90°; repeat on a raster layer | The drawn preview maps (10, 10) to (25, 5) for both | `tst_shape_transform::rotationPreviewTurnsTheLayer` |
| Visual check | Ctrl+T a shape and a pixel layer; drag the rotate band and a scale handle | The layer turns and scales with the box while dragging | manual: the drawn frame's look during a live drag |
