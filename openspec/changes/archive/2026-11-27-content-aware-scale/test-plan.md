# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cas_session` | Open on a raster layer; drag a handle, edit W/H, pivot and aspect lock. | Preview bounds and controls agree with geometry. | `tst_content_aware_scale::sessionGeometry` |
| `cas_refusal` | Try unsupported, multiple and locked targets, linked and unlinked masks of different bounds, converted-mode layers lacking retained source channels, and selections at 16/32-bit depth. | Refusal preserves pixels, masks, selection and history. | `tst_content_aware_scale::refusesTargets` |
| `cas_selection` | Transform a feathered 8-bit selection over marked unselected destination pixels; undo and redo. | Complementary source coverage remains, destination is composited over, border clears, and undo/redo restores both states. | `tst_content_aware_scale::selectionLimitsTarget` |
| `cas_exclusive` | Attempt edits and competing transforms while pending; switch tool and layer with each resolution choice, including Cancel. | Edits and competing sessions refuse; switches follow the resolution and Cancel keeps the session. | `tst_content_aware_scale::exclusiveSession` |
| `cas_amount` | Resize a nonuniform fixture at 0, 50 and 100 Amount. | Endpoints match ordinary and carved outputs; midpoint blends them. | `pictura_render::content_aware_amount_endpoints_and_midpoint` |
| `cas_protect` | Shrink a fixture with competing seams, with and without a protection channel. Compare the channel before and after. | Protected seam is avoided when an unprotected alternative exists; channel is unchanged. | `pictura_render::content_aware_protect_biases_seams` |
| `cas_protect_ui` | Open a session in a document with named alpha channels. | Amount is 100; Protect is None and lists the channels. | `tst_content_aware_scale::protectControls` |
| `cas_axes` | Shrink and enlarge both axes, repeat with color and alpha data. | Dimensions and plane alignment are correct and results deterministic. | `pictura_render::content_aware_axes_and_enlargement` |
| `cas_native` | Resize 16-bit and 32-bit non-RGB whole layers with distinct components and out-of-range float samples, plus newly created unconverted high-depth layers without a native store. | Native depth and component alignment remain; floats are not clamped. | `pictura_render::content_aware_native_components` |
| `cas_invalid` | Request zero, non-finite, overflow and over-budget dimensions and inject scratch allocation and admission refusal. | Refusal preserves the document and history. | `pictura_render::content_aware_invalid_is_atomic`, `tst_content_aware_scale::sessionGeometry` |
| `cas_preview` | Hold one preview job, request another, then release jobs out of order; repeat with cancellation. | Stale results cannot publish or commit; commit waits for the current result. | `tst_content_aware_scale::supersededPreview` |
| `cas_cancel` | Change preview, cancel and compare full state. | Original pixels, geometry, selection and history are exact. | `tst_content_aware_scale::cancelRestoresState` |
| `cas_history` | Commit a change, undo and redo. | One history state; original and committed states restore exactly. | `tst_content_aware_scale::commitUndoRedo` |
