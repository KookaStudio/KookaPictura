# Proposal: Restore the strip-button scenarios dropped by the rename

## Why

`linked-layers-move` renamed `Layers panel action strip icons` to
`Layers panel action strip buttons` and dropped five scenarios — the three mask
button clicks (`lmk_strip`, `lmk_strip_sel`, `lmk_strip_alt`) and the two fx
button clicks. The migration note claimed they stayed specified in
`ui/layers-mask-commands` and `ui/layer-style-dialog`, but neither capability
carries a strip-click scenario, so the contract silently shrank. This change
restores them, and adds the tests that were missing for the four not already
covered.

## What Changes

- The `Layers panel action strip buttons` requirement again carries the three
  mask-button scenarios and the two fx-button scenarios.
- New tests: `maskButtonRespectsSelection`, `maskButtonAltHidesAll`,
  `fxButtonOpensBlendingOptions`, `fxButtonAltTogglesAll`.
- `addMaskFromButton` and the fx handler take their Alt state as a parameter
  (the button passes the process-global modifier), so a test can drive the
  Alt-click; `openLayerStyle` goes through the panel's dialog driver.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the five restored strip-button scenarios.

## Impact

- C++: `panels/layers_panel.{h,cpp}`, `panels/layers_panel_mask_menu.cpp`,
  `panels/layers_panel_actions.cpp`, `panels/layers_panel_test_extra.cpp`.
- Tests: `tst_layers_mask_commands`, `tst_layers_panel_fixes`.
