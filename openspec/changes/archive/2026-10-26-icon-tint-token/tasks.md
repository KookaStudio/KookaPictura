# Tasks

## 1. Theme token

- [x] 1.1 Add `Ramp::icon` with `#FFFFFF` on the dark family and `#2B2B2B` on
  the light family.
- [x] 1.2 Expose it as the `icon` token in `Theme::tokens()` and list it in the
  design-system palette page.
- [x] 1.3 Mark `icon` paint-only in `tst_design_system` and assert its
  brightness-constant dark value.

## 2. Icon engine

- [x] 2.1 Read `Theme::tokenColor("icon")` in `SvgIconEngine::colorFor()` for
  normal/active modes.
- [x] 2.2 Keep the disabled palette colour, the 80 % opacity, and the explicit
  colour override.

## 3. Tests

- [x] 3.1 Add `defaultTintIsWhite` to `tst_icon_assets`: strongest pixel is
  `#FFFFFF` at ~80 % alpha.
- [x] 3.2 Add `disabledTintStaysDimmed`: strongest disabled pixel is `#686868`.
- [x] 3.3 Add `lightFamilyTintIsDark`: the light family renders `#2B2B2B`.

## 4. Verification

- [x] 4.1 `cmake --build build` and run `tst_icon_assets` + `tst_design_system`.
- [x] 4.2 Regenerate `design-system.html` with `scripts/gen-styleguide.py` and
  commit the diff.
- [x] 4.3 `bash scripts/verify-fast.sh` — all gates pass; the one failure is the
  pre-existing ImageMagick `ramp_Divide` oracle, which this C++-only change does
  not touch.
- [x] 4.4 Archive the change with `openspec archive icon-tint-token -y`, so the
  pull request leaves no open change.
