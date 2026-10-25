# Colour management and picker bundle

## Summary

Profile-aware CMYK conversion plus the Color Settings, Convert to Profile, profile-warning, ICC-embedding, raster-ICC, and color-picker surfaces. The engine now uses lcms2 for CMYK whenever a document or working CMYK profile is available; the profile-free formula is only the fallback.

## Issues

- Closes #568 — Convert to Profile options, warnings, embed toggle
- Closes #809 — CMYK conversions use the document/working ICC profile
- Part of #569 — Color Settings spaces, policies, conversion options; `.csf`, Gray/Spot spaces, and installed-profile discovery remain
- Part of #600 — picker web/gamut alerts and preference; Color Libraries remain
- Part of #604 — out-of-gamut triangle and CMYK working-space fields; 16/32-bit picker fields remain

## What shipped

- `ConversionOptions` (intent/BPC/dither) and deterministic 8-bit dither.
- CMYK read/write/mode conversion through the embedded or working profile; `OpenOptions` and RGB/CMYK incoming policies.
- Color Settings: fixed sRGB RGB space, working CMYK space, RGB/CMYK policies, warning asks, conversion options, existing text gamma.
- Convert to Profile: intent/BPC/dither, flatten, on-canvas preview, one history state.
- PSD/JPEG/TIFF/PNG profile probes; mismatch/missing warnings at open and Place; cancel aborts.
- PSD/PSB/JPEG/TIFF Save As `Embed Color Profile` (resource 1039, APP2, TIFF tag 34675).
- JPEG/TIFF/PNG imports apply Preserve/Convert/Off; Place converts to the host working space.
- Picker: web-safe and gamut alerts, editable CMYK through the working profile, Adobe/System preference.

## Deferred

`.csf`, Color Libraries (PANTONE), 16/32-bit picker fields, Gray/Spot working spaces, configurable RGB working space, profile-aware Selective Color, and CMYK-to-CMYK Convert to Profile.

## Verification

- `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, workspace tests, doctests
- `cmake --build build --parallel 4`, `ctest --test-dir build -R '^tst_'`
- `./build/pictura --headless --self-test`
- `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh`
- `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`
- New suites: `tst_color_settings`, `tst_profile_dialog`, `tst_profile_warnings`, `tst_save_options`, `tst_image_import`; extended `tst_color_picker`
- Rust oracles: CMYK profile-bearing fixture vs lcms2, JPEG/TIFF ICC embed vs exiftool, raster import policy

## Notes

The bundled CC0 `Photocraft Coated CMYK` is a modelled press, not SWOP; CMYK parity claims are lcms2 parity. The profile-free CMYK formula remains the documented fallback when no profile is available.
