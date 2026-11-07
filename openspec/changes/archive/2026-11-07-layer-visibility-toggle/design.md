# Design

## Why the region path is not enough

`set_layer_visible` and `set_layers_visible` return `Edit::regions(..., false)`.
`presentation_of` turns that into `Rebuild::None, notify: false`, so only
`regionDamaged` is emitted and no `changed` notification reaches the panels.
The frame's `afterPresent` still starts the 120 ms panel timer, and because
`panelsRefreshedByChanged_` is false the pass runs `refreshDock(true)`, which
calls `LayersPanel::refresh()` and rebuilds the snapshot and every thumbnail.

## The signal

`PictureView::layerVisibilityChanged(QStringList paths, bool visible)` is emitted
synchronously by `set_layer_visible` and `set_layers_visible` after the edit,
only when the edit changed something. It carries the same paths and target state
the bridge applied, so the panel does not need to read the document (a path read
would itself rebuild a thumbnail-free snapshot).

`apply_visibility` (solo) keeps `batch_changed`, which emits `changed`; the panel
refreshes itself and every row is rebuilt, as before. An unbounded visibility
edit (a knockout document or a canvas-sized adjustment) returns `Edit::Full`,
which always rebuilds the display and notifies; that rebuild is left alone
because the full recomposite already dominates it.

## The one-pass skip

`LayersPanel` patches the rows through `LayerModel::setVisible` and records that
it is current. The frame's debounced pass asks the panel whether that patch is
still pending:

```cpp
if (includeSelfRefreshing) {
    if (!layersPanel_ || !layersPanel_->consumeVisibilityPatch()) {
        timed("cxx_layersPanel_refresh", layersPanel_);
    }
    timed("cxx_historyPanel_refresh", historyPanel_);
}
```

`consumeVisibilityPatch()` returns the flag once and clears it, so the skip is
scoped to the single pass that follows the visibility edit. Any full
`refresh()`, a rebind, or a `changed` notification clears it as well. History
still refreshes because the edit records an undo state.

The panel's own eye handler stops patching the rows a second time: the signal is
synchronous, so the model already carries the new value when the handler returns.

## Alternatives considered

- **Emit `changed` for visibility edits.** It reuses the existing
  `panelsRefreshedByChanged_` skip, but `changed` also runs the synchronous
  command-registry refresh, status and title fan-out, and the Paths/Histogram
  refresh — more work than the rebuild we are removing.
- **Have `LayersPanel::refresh()` early-return on a flag.** The dock pass has
  other callers (rebind, document switch, stroke commit); a panel-local skip
  could swallow a legitimate refresh.
- **Skip the whole dock pass.** History would go stale after an undoable edit.

## Test strategy

A Qt test in `tst_edit_refresh` drives the real window: it settles the debounce,
toggles through both `set_layers_visible` and `set_layer_visible`, and asserts
`refreshCountForTest()` does not move while `rowVisibleForTest()` follows the
document. A following rename must still rebuild once, and a solo edit must
rebuild. No new test suite is registered.
