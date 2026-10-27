# Design: Restore the strip-button scenarios dropped by the rename

## Contract

The five scenarios are re-added verbatim to `Layers panel action strip buttons`;
no behavior changes.

## Testability

The mask and fx buttons read `QApplication::keyboardModifiers()`, which a
synthetic `QTest` click cannot set. `addMaskFromButton` and the new
`applyFxButton` therefore take the Alt state as a parameter: the button's
`clicked` lambda passes the process-global modifier, and the panel test hooks
(`altClickMaskButtonForTest`, `clickFxButtonForTest(bool)`) pass it directly.

`openLayerStyle` now runs its dialog through `runPanelDialog`, the wrapper that
honors `setDialogDriverForTest`, so `fxButtonOpensBlendingOptions` can assert the
dialog opened without a modal event loop.

## Coverage

`lmk_strip` was already covered by `maskButtonFollowsTheMasks`; the other four
get new cases (`maskButtonRespectsSelection`, `maskButtonAltHidesAll`,
`fxButtonOpensBlendingOptions`, `fxButtonAltTogglesAll`).
