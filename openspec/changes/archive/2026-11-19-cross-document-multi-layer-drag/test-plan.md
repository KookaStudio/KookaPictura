# Test Plan — cross-document-multi-layer-drag

One row per scenario in `specs/ui/layers-panel/spec.md`.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `lpr_drag_to_document` | Drag `Sky` from document A onto document B's tab and release | B gains selected `Sky` with the same opacity in one undo step; A keeps its layers | `tst_layers_panel::dragLayerOntoAnotherDocument` |
| `Move-tool drag from the canvas onto another document's tab` | Press the Move tool on A's canvas, drag to B's tab, release | B gains a copy of every selected layer, or of the active layer when only it is selected; A's layers did not move | manual: the gesture is a blocking QDrag from a canvas mouse drag; the same drop handler is covered by tst_layer_target::crossDocumentMultiCopySelectsCopies |
| `Drop a layer on another document's canvas` | Drag a layer from A and release on B's canvas | B gains a copy of the layer | `tst_layers_panel::dragLayerOntoAnotherDocument` |
| `A mode mismatch refuses the drop` | Drag an RGB layer onto a Grayscale document's tab | The Grayscale document is unchanged | `tst_layers_panel::dragLayerOntoAnotherDocument` |
| `The destination panel ignores the foreign drag` | Release a foreign layer drag on the destination panel rows | The drop is refused and the destination is unchanged | `tst_layers_panel::dragLayerOntoAnotherDocument` |
| `lpr_drag_to_document_multi` | Select `Sky` and `Sea` in A, drag `Sky` onto B's tab, release; repeat with a selected group plus a sibling, with an empty destination target, and with an RGB multi-selection onto a Grayscale tab | B gains both selected layers in source order, `Sky`'s copy active, one undo step, A unchanged; the group and empty-anchor cases keep the source order; the Grayscale destination gains nothing and records no state | `tst_layer_target::crossDocumentMultiCopySelectsCopies` |

## Manual checks

- Move-tool drag with a multi-layer target across documents: the source keeps
  all layers and the destination gains the whole selection. It shares the drop
  handler above but needs a real canvas gesture.
