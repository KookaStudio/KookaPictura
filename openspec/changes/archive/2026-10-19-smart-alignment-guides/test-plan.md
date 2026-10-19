# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sg_document_centre` | Press on a layer, drag its content centre to within eight screen pixels of the document centre, inspect the canvas mid-drag, then release | A magenta vertical and/or horizontal guide sits at the document centre; after release the layer's content centre equals the document centre | `tst_smart_guides::documentCentre` |
| `sg_layer_edge` | With two layers, drag one so its left content edge is near the other's left content edge | A vertical guide is drawn at that x; after release the two left edges coincide | `tst_smart_guides::layerEdge`, `tst_smart_guides::moveToolTextLayers` |
| `sg_threshold` | Drag the same edge to just outside eight screen pixels | No guide is drawn and the layer keeps the pointer's offset | `tst_smart_guides::threshold` |
| `sg_clears` | Release after a snapping drag | The canvas reports no smart guides | `tst_smart_guides::documentCentre` |
| `sg_type_move` | With the Type tool, type text and drag it so its ink-box centre approaches the document centre, then commit | A guide is drawn during the drag and the committed layer's ink box is centred | `tst_smart_guides::typeMove` |
| `sg_gap` | Put a reference layer 36 px below the moving one, align the facing edges, and read the gap label at 72 ppi with inches | The label reads `0.500 in` | `tst_smart_guides::gapReadout` |
| `sg_two_lines` | Snap a layer so its left edge meets a reference's left edge and its centre meets that reference's right edge | Two vertical guide lines are reported | `tst_smart_guides::twoLines` |
