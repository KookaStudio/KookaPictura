# Tasks

## Tests

- [x] Tests: `tst_image_adjustments` Levels Auto on a ramp, Curves Auto on a
      ramp, Brightness/Contrast Auto on a ramp, Auto no-op on a flat image, and
      Vibrance has no Auto button
- [x] Tests: the existing Brightness/Contrast case expects an enabled Auto
      button

## Shell

- [x] `AdjustmentDialog`: add the shared histogram bounds helper, a virtual
      `applyAuto`, and Auto wiring in `buttonColumn` / `bottomButtons`
- [x] `LevelsDialog`: move the inline Auto onto the shared helper and button
- [x] `CurvesDialog`: add the Auto button and the two-point stretch
- [x] `SliderDialog`: replace the disabled-button list with extra buttons and
      implement the Brightness/Contrast Auto solve
- [x] `makeAdjustmentDialog`: give Brightness/Contrast an Auto button and keep
      Vibrance without one

## Docs

- [x] Tick `ROADMAP.md` item for #578
- [x] Close `ADJ-8` in `docs/dev/doc-code-parity-gaps.md`
- [x] Record the change in `docs/dev/STATE.md`

## Verify

- [x] `openspec validate --all --strict`
- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `bash scripts/verify-fast.sh`
