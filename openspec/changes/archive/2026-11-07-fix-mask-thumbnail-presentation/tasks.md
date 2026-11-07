# Tasks

## 1. Regression tests before implementation

- [x] 1.1 Write `tst_canvas_mask_domain::importedPortraitMaskCardIsOpaque` for opened PSDs, all thumbnail sizes, opaque source pixels, painted card edges and fractional display scales.
- [x] 1.2 Write `tst_canvas_mask_domain::maskThumbnailUsesOnlyBrackets` and `tst_canvas_mask_domain::maskBracketsFollowRasterAndLayerClicks` for black outlines, white brackets and real target transitions.
- [x] 1.3 Write `tst_canvas_mask_domain::maskModifierIndicatorsRemain` for retained mask/vector link, cross and solo behavior; map existing square-card and solo-exit checks in test-plan.md.

## 2. Presentation fix

- [x] 2.1 Fill ordinary document-aspect mask cards directly; preserve fitting for square cards and verify the portrait/card regression.
- [x] 2.2 Replace the cached-target white rectangle with a standard black outline; verify selection-aware brackets and all modifier checks.

## 3. Verification and archive

- [x] 3.1 After all source edits, build with 12 jobs, run the focused mask suites and the full Qt suite, and replay the production screenshot reproduction.
- [x] 3.2 Sync and archive this change, run verify-fast and strict spec/test-plan/format checks, and record results with no manual rows.
