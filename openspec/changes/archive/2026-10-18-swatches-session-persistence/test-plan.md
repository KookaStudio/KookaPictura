# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sw_persist_restart` | Add a swatch in one window, destroy it, create a new window | The new panel contains the added swatch | `tst_swatches_panel::addedSwatchesSurviveARestart` |
| `sw_persist_empty` | Clear the grid in one window, destroy it, create a new window | The new panel is empty | `tst_swatches_panel::emptyPanelSurvivesARestart` |
| `sw_persist_default` | Load a store with no swatch data | The panel holds the default 48 swatches | `tst_swatches_panel::freshStoreUsesDefaultPalette` |
| `sw_persist_unrelated_save` | Load a store with swatches, change another setting, save | The swatches remain | `tst_session_store::swatchesSurviveAnUnrelatedSave` |
