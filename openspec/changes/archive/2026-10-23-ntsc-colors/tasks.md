# Tasks

## 1. Spec and test plan

- [x] 1.1 Write the `imaging/video-filters` delta with the gamut-clamp and command requirements
- [x] 1.2 Pair it with `test-plan.md`, one row per scenario
- [x] 1.3 Add the named tests (`pictura-color`, `pictura-filters`, `pictura-app`, `tst_filter_menu`) before the implementation they cover

## 2. Colour math in `pictura-color`

- [x] 2.1 Add `src/ntsc.rs` with `NTSC_MAX_CHROMA`, `rgb_to_yiq`, `yiq_to_rgb` and `clamp_to_ntsc_gamut`, documenting the matrix and the approximation
- [x] 2.2 Expose the module from `lib.rs`
- [x] 2.3 Unit-test the round trip, the clamp, luma/hue preservation, the in-gamut no-op and the output range

## 3. Engine kernel in `pictura-filters`

- [x] 3.1 Add the `pictura-color` dependency and mark the new `Filter::NtscColors` variant, its engine row and its property rows (`moves_alpha`, `cpu_outruns_gpu`, `preview_apron`)
- [x] 3.2 Add `src/video.rs` with `ntsc_colors` (validate, per-pixel clamp over the three colour planes, alpha untouched) and dispatch it from `apply`
- [x] 3.3 Unit-test saturation reduction, the in-gamut no-op, alpha preservation and malformed-buffer refusal
- [x] 3.4 Add the variant to the exhaustive test tables (`tests/common/instances.rs`, `tests/kernel_digests.rs`, `tests/oracle/mapping.rs`)

## 4. App wiring

- [x] 4.1 Add `filter_map/descriptor/video.rs` with the `ntsc-colors` row and register it in the descriptor table
- [x] 4.2 Add the `ntsc-colors` mapping arm and the descriptor/mapping tests (including the kind count)
- [x] 4.3 Add `tst_filter_menu::ntscColorsIsAParameterlessMenuEntry` (enabled, no ellipsis, applies directly, one history state, disabled without a filter target)

## 5. Cleanup and verification

- [x] 5.1 Tick NTSC Colors in `ROADMAP.md` and keep De-Interlace (#344) open
- [x] 5.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`
- [x] 5.3 Build once, then run the affected Rust tests, `ctest -R '^tst_filter_menu$'`, and `verify-fast.sh`
- [x] 5.4 Archive the change with `openspec archive ntsc-colors`
