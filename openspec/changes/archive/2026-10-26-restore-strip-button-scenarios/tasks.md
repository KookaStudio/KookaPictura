# Tasks

## 1. Tests first

- [x] Add `tst_layers_mask_commands::maskButtonRespectsSelection`.
- [x] Add `tst_layers_mask_commands::maskButtonAltHidesAll`.
- [x] Add `tst_layers_panel_fixes::fxButtonOpensBlendingOptions`.
- [x] Add `tst_layers_panel_fixes::fxButtonAltTogglesAll`.

## 2. Restore the scenarios and the seams

- [x] Re-add the five scenarios to `Layers panel action strip buttons`.
- [x] Give `addMaskFromButton` and the fx handler their Alt state as a parameter
  and add the test hooks.
- [x] Run `openLayerStyle`'s dialog through `runPanelDialog`.

## 3. Verification and archive

- [x] Run the mask/panel suites, the self-test, `cargo test -p pictura_app`,
  `openspec validate --all --strict`, and the guards.
- [x] Archive the change.
