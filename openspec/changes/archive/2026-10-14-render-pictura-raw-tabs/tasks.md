# Tasks

## 1. Settings model correction (`pictura-core`)

- [x] 1.1 Move `legacy_red_hue`, `legacy_red_saturation`, `legacy_green_hue`, `legacy_green_saturation`, `legacy_blue_hue`, `legacy_blue_saturation` from `Hsl` to `CameraCalibration` as `red_hue`, `red_saturation`, `green_hue`, `green_saturation`, `blue_hue`, `blue_saturation`; move `legacy_blacks` from `PicturaRawSettings` to `CameraCalibration.shadow_tint`; add `Effects.dehaze`. Verify with `cargo test -p pictura-core` and `cargo clippy -p pictura-core --all-targets -- -D warnings`.
- [x] 1.2 Update the `raw_settings` unit tests to assert the calibration controls live under `CameraCalibration` and Dehaze under `Effects`.

## 2. Fltr codec (`pictura-codec`)

- [x] 2.1 Decode `RHue`/`RSat`/`GHue`/`GSat`/`BHue`/`BSat` and `BlkB` into `CameraCalibration`; drop them from `decode_hsl` and the top-level `legacy_blacks`; encode `Dhze` from `Effects.dehaze`. Verify with `cargo nextest run -p pictura-codec pictura_raw`.
- [x] 2.2 Update `pictura_raw/tests.rs`: remove `Dhze` from `PRESERVE_KEYS`, update the relocated-key round-trip and re-attach tests, and keep default-encodes-no-keys and decode-tolerance green.

## 3. Fixture and docs

- [x] 3.1 Update `examples/make_smart_fixtures.rs` to author Dehaze as a modeled `Effects` control and the primaries/shadow tint under `calibration`; regenerate `assets/test_with_smart_object02.psd` with `cargo run -p pictura-codec --example make_smart_fixtures`.
- [x] 3.2 Correct `docs/dev/camera-raw-cc-notes.md` §Fltr key map: move `RHue`/`RSat`/`GHue`/`GSat`/`BHue`/`BSat` and `BlkB` to Camera Calibration and `Dhze` to modeled; the commit carries `TASK-ALLOWS-DOCS`. Verify `bash scripts/guard.sh` passes.
- [x] 3.3 Drop `Dhze` from the unmodeled-key examples in `codec/psd-smart-filters` (delta under this change), since it is now modeled.

## 4. Render module split

- [x] 4.1 Move `crates/pictura-adjust/src/pictura_raw.rs` to `pictura_raw/mod.rs` with the 11 Basic stages in `basic.rs`, as a pure move; keep the existing tests green.
- [x] 4.2 Add a whole-settings no-op guard in `mod.rs` and a test that default settings are byte-identical for a value with a single zero-set control in each tab.
- [x] 4.3 Add `tone_curve.rs` (parametric regions + point curves) and its check.
- [x] 4.4 Add `detail.rs` (USM variation + luminance/chroma noise reduction) and its check.
- [x] 4.5 Add `hsl.rs` (eight ranges + grayscale mix + `CtoG`) and its check.
- [x] 4.6 Add `split_toning.rs` and its check.
- [x] 4.7 Add `lens.rs` (manual distortion, defringe, vignette) and its check.
- [x] 4.8 Add `effects.rs` (grain, Dehaze, post-crop vignette) and its check.
- [x] 4.9 Add `calibration.rs` (primaries + shadow tint) and its check.
- [x] 4.10 Confirm `render_pictura_raw` stays the public entry point and the smart-filter chain still compiles unchanged.

## 5. Integration verification

- [x] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`, and `openspec validate --all --strict`; all must pass.
- [x] 5.2 Run `bash scripts/verify-fast.sh` and confirm the `pictura` self-test reports unchanged passed/failed/skipped counts.

## 6. Review follow-ups

- [x] 6.1 Fix defringe hue windows: map the stored 0..100 slider positions onto a band centred on each fringe's hue (purple 300°, green 120°) instead of the whole wheel, so "Purple Amount" no longer targets greens/cyans; exercise the default windows in the test.
- [x] 6.2 Gate the HSL range and grayscale-mix edits by a pixel's chromatic presence so a hue-targeted range never moves a neutral grey; add a neutral-input test.
- [x] 6.3 Add the missing per-stage checks (lens distortion, post-crop vignette, green/blue calibration primaries) and a "every tab changes the render" test.
- [x] 6.4 Record the modeled-but-not-applied controls (noise sub-controls, lens profile/perspective, vignette roundness) in `design.md`.

