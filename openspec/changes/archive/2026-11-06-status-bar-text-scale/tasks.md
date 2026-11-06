# Tasks

## 1. Test

- [x] 1.1 Add `tst_ui_scale::statusBarTextFollowsScale`: assert the status bar
  and its size label, backend label, and tool hint strip use the application
  font minus two pixels, that each readout's height holds its font metrics, and
  that both still hold after `setUiScalePercent(75)`; verify it fails on the
  current tree (the labels render at the application font and the bar font does
  not follow the scale) and passes after the fix.

## 2. Fix

- [x] 2.1 In `applyStatusBarScale()`, derive the footer font from the live
  application font (2 px under, floored at 1 px) and set it on the status bar
  and on every readout, hint strip, and the search hint, in the same loop that
  applies the item heights.
- [x] 2.2 Call `applyStatusBarScale()` at the end of `buildStatusBar()`,
  replacing the old bar-only font block, so the initial footer text and the
  readout heights come from the same owner as a live scale change.

## 3. Verification

- [x] 3.1 Build with CMake and run
  `ctest --test-dir build -R '^tst_ui_scale' --output-on-failure`; confirm the
  suite passes, and confirm `statusBarTextFollowsScale` fails when the helper is
  reduced to the old bar-only `setFont`.
- [x] 3.2 Run `bash scripts/guard.sh`, `bash scripts/check-file-size.sh`,
  `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`;
  confirm all pass. The full `verify-fast.sh` gate (fmt/clippy/all suites) is
  left to CI.
