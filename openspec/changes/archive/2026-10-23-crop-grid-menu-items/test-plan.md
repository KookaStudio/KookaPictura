# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The button follows the selection | Crop tool; open the grid menu; click each overlay; set the overlay from code | No entry is checkable; each overlay has its own icon; the button shows the chosen overlay's icon | `tst_crop_options::overlayMenuShowsIcons` |
