# Test plan

This plan covers `compositing/composite-view` for
`hidden-layer-display-refresh`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cv_solo_hide` | Open a red Background image, add and paint a small layer, set fit zoom, Alt-click solo the small layer | The present at the selected level equals a full recomposite; the Background is gone outside the soloed layer | `tst_present_cache::soloHideRefreshesTheWholeCanvas` |
| `cv_solo_restore` | Restore the solo snapshot after the previous step | The present equals a full recomposite with the Background visible again | `tst_present_cache::soloHideRefreshesTheWholeCanvas` |
| `cv_hidden_at_fit` | Hide a layer (Background included) while the canvas presents a reduced level; repeat on a lazily opened PSD | The present crop equals a full recomposite at that level and the merged preview no longer shows the layer | `tst_present_cache::hiddenLayerHidesAtFitZoom`, `tst_present_cache::openedBackgroundHidesAtFitZoom`, `tst_present_cache::lazyOpenedBackgroundHidesAtFitZoom` |
