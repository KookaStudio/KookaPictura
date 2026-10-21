# Test Plan

Covers the `tools/move-transform-controls` delta. The scenarios are tool and
panel-state checks, so they are automated with Qt Test against the real
`PicturaMainWindow`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lhe_box` | Show the Move tool's controls box on a raster layer, then hide the layer | The box and its session disappear | `tst_transform_controls::hiddenLayerShowsNoControlsOrMove` |
| `lhe_move` | Hide the active layer, then press and drag on the canvas with the Move tool | No move preview starts, the layer rect is unchanged, and no history state is added | `tst_transform_controls::hiddenLayerShowsNoControlsOrMove` |
| `lhe_transform` | Hide the layer, then invoke Free Transform and the Move preview directly, and repeat with an active selection | The action is disabled, both entry points refuse, no session starts, and the selection cannot lift the hidden layer | `tst_transform_controls::hiddenLayerRefusesTransforms` |
