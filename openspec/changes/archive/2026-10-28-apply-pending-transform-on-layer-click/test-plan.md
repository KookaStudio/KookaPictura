# Test Plan

One row per spec scenario in this change. Coverage names the automated test
that proves the scenario, or `manual:` with the reason it cannot be automated.
Tests that do not exist yet are created by the tasks named in `tasks.md`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `layer_click_apply` | Begin a Free Transform on a layer, nudge it, answer `Apply`, then click another layer in the panel | One `Free Transform` state is recorded, no session remains, and the clicked layer is the active layer | `tst_transform_lock::layerClickAppliesThePendingTransform` |
| `layer_click_dont_apply` | Begin a Free Transform, nudge it, answer `Don't Apply`, then click another layer | No state is recorded, no session remains, and the clicked layer is the active layer | `tst_transform_lock::layerClickDontApplyCancels` |
| `layer_click_cancel` | Begin a Free Transform, answer `Cancel`, then click another layer | The session is still pending, the previous layer is still active, and the editing controls stay disabled | `tst_transform_lock::layerClickCancelKeepsTheTransform` |
| `lock_layer_menu` | Begin a Free Transform, then read the Layer menu actions | `Layer > New > Layer`, `Layer > Delete > Layer`, `Layer > Hide Layers`, and `Layer > Arrange > Send Backward` are disabled, New Layer dispatch adds no layer, and they are enabled after the commit | `tst_transform_lock::layerMenuIsLockedWhilePending` |
| `lock_layers_panel` | Begin a Free Transform, then read the panel's enabled state | The panel and its tree are enabled; the blend mode, opacity, fill, lock toggles, and strip buttons are disabled; all are enabled again after Cancel | `tst_transform_lock::panelEditingControlsLockedWhilePending` |
| `lock_document_edits` | With a lifted transform pending, dispatch Edit > Fill/Stroke and call the direct fill/stroke/filter APIs, then commit the transform | Each edit is refused, the document and history are unchanged, and the transform's own commit records one state | `tst_transform_lock::documentEditsDuringPendingTransform` |
| `lock_controls_box` | Turn on the Move tool's `Show Transform Controls`, start a controls gesture, then run Edit > Fill and a filter | Both stay enabled and the controls session stays live | `tst_transform_lock::documentEditsDuringPendingTransform` |
