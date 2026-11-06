# Tasks

## 1. Test

- [x] 1.1 Add `tst_panels_parity::kindBadgesHidpi`: paint a shape row and a
  smart-object row at a device pixel ratio of 2, crop each row's `kindBadgeRect`,
  and assert the badge pixels match a 2x-rendered reference more closely than a
  1x-rendered pixmap upscaled to 2x; verify it fails on the current tree (both
  badges are drawn from a 1x pixmap) and passes after the fix.

## 2. Fix

- [x] 2.1 In `layers_panel_internal.h`, add `kindBadgeRect(itemRect, index)` and
  a shared `paintKindBadge` helper built on `paintAsset`, and route both the
  shape and smart-object badges through it.
- [x] 2.2 Add the optional device-pixel-ratio parameter to
  `LayersPanel::rowImageForTest` (default 1.0) so a test can paint a row at 2x.

## 3. Verification

- [x] 3.1 Build with CMake and run
  `ctest --test-dir build -R '^tst_panels_parity' --output-on-failure`; confirm
  the suite passes, and confirm `tst_panels_parity::kindBadgesHidpi` fails when
  the badge is forced back to a 1x pixmap.
- [x] 3.2 Run `bash scripts/guard.sh`, `bash scripts/check-file-size.sh`,
  `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`;
  confirm all pass. The full `verify-fast.sh` gate (fmt/clippy/all suites) is
  left to CI.
