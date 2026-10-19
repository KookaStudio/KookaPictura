# Tasks

## 1. Tests first

- [x] 1.1 Extend `LayersPanel::rowGutterColorForTest` with an `x` offset (default 1) and update `tst_layers_panel::layerSurfaceCompleteness` to assert the labeled column is the exact label color at `x = 1` and `x = kEyeColumn - 2`, the labeled row's thumbnail and name left match the unlabeled sibling, and selecting the labeled row keeps the fill; verify the test fails against the current chip/tint code and keep `tst_layers_panel.cpp` within the 1400-line test cap.

## 2. Chip removal and full-column fill

- [x] 2.1 Remove `rowLabelChipColorForTest` from `layers_panel.h` and `layers_panel_test.cpp`, and update the stale label-chip comments in `layers_panel.h`, `layers_panel_test.cpp`, `tst_layers_panel.cpp`, `selftest_layers_round3.cpp`, and `selftest_layers_drag.h`; verify `rg -i 'rowLabelChipColorForTest' crates/` returns nothing.
- [x] 2.2 Remove `kLabelChipWidth`, `kLabelChipGap`, `labelChipAdvance()`, `labelChipRect()`, the chip fill, and both advances; verify `rg -i 'label.?chip' crates/` returns nothing and the thumbnail/name math has one source.
- [x] 2.3 Fill the whole visibility column with the opaque label color before the row-bottom rule and the eye glyph; verify `tst_layers_panel::layerSurfaceCompleteness` and self-test token `lpr_label_tint` (300) pass.
- [x] 2.4 Add the optional `paintAsset` tint and choose a dark or light eye glyph with `qGray(label.rgb()) >= 85`, with a `ponytail:` comment naming the sheet-specific ceiling; verify all seven labels with `tst_layers_panel_fixes::labelGlyphTintFollowsFill` and spot-check yellow and violet in the running panel.

## 3. Verification

- [x] 3.1 Run `ctest --test-dir build -R '^tst_layers_panel$' --output-on-failure`, `./build/pictura --headless --self-test`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`.
- [x] 3.2 Archive the change with `openspec archive layer-label-eye-cell` and verify only `openspec/changes/archive/` remains, the canonical spec has the replacement requirement, `lpr_label_chip` is gone, and the older requirements' "label tint" wording still means the full-column fill.
- [x] 3.3 Run `bash scripts/verify-fast.sh` after the archive so the zero-open guard passes; fix any file-size, guard, or report failure.
