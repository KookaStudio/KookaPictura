# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sg_toggle` | Open a document, trigger `View > Show > Smart Guides` off, drag a layer near the document centre, then trigger it on | The action starts checked; while off no guide is drawn and the layer keeps the pointer offset; while on a guide appears and the layer snaps; the session stores the choice | `tst_smart_guides::smartGuidesToggle` |
