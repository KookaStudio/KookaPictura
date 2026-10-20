# Test plan — Distort preview warp

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `distort_preview_full` | Distort a filled rectangle to the reported steep quad | The preview shows black across the whole quad | `tst_transform_distort::steepQuadKeepsTheWholeShape` |
| `distort_preview_layers` | Distort a layer-masked, vector-masked, and clipped-fill layer to the same quad | The preview keeps each layer's appearance across the quad | `tst_transform_distort::steepMaskedQuadKeepsTheWholeShape`, `tst_transform_distort::steepVectorMaskQuadKeepsTheWholeShape`, `tst_transform_distort::steepClippedFillQuadKeepsTheWholeShape` |
| `distort_preview_front` | Warp a source whose map has a horizon | The front side is drawn and the far side is not | `tst_transform_distort::projectivePreviewClipsAtTheHorizon` |
