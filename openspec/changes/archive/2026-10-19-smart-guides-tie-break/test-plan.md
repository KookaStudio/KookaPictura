# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sg_two_lines` | Snap a layer so its left edge meets a reference's left edge and its centre meets that reference's right edge | Two vertical guide lines are reported | `tst_smart_guides::twoLines` |
| `sg_tie` | Place a moving edge equally close to a document target and a reference layer's target | The solver snaps to the reference layer's target, not the document's | `tst_smart_guides::tiePrefersLayer` |
