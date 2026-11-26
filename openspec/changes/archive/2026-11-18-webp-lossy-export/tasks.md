# Tasks

## 1. Prepare the regression tests

- [x] 1.1 Add `pictura-codec::lossy_webp_round_trips_within_tolerance`,
  `pictura-codec::lossy_webp_is_smaller_than_lossless`,
  `pictura-codec::lossy_quality_orders_file_size`,
  `pictura-codec::lossy_webp_preserves_alpha`, and
  `pictura-codec::lossy_webp_refuses_a_bad_buffer` in
  `crates/pictura-codec/src/webp.rs`.
- [x] 1.2 Add `pictura-codec::magick_decodes_our_lossy_webp_within_tolerance`
  to `crates/pictura-codec/tests/save/webp_oracle.rs`; self-skip without `magick`.
- [x] 1.3 Add `tst_export_as::webpQualityControls`,
  `tst_export_as::webpLossyIsSmallerThanLossless`, and
  `tst_export_as::webpLosslessRoundTrips`; update `formatsAndControls` and
  `webpExport` for the new default.
- [x] 1.4 Add `tst_webp::saveStaysLossless`; a Save As `.webp` keeps every
  pixel exact even when Export As defaults to lossy.

## 2. Engine lossy encoder

- [x] 2.1 Add `gamut-webp = "0.3"` and `gamut-core = "2"` to
  `crates/pictura-codec/Cargo.toml` with the dependency rationale.
- [x] 2.2 Add `encode_webp_lossy` beside `encode_webp`, validate the buffer,
  clamp quality, and preserve alpha through `VP8X`/`ALPH`. Re-export it from
  `lib.rs`.

## 3. Export As wiring

- [x] 3.1 Change the `webp_encode` bridge to take `quality: i32` and
  `lossless: bool`, and dispatch to the lossless or lossy codec entry point.
- [x] 3.2 Add the `exportAsLossless` checkbox to `ExportAsDialog`, gate Quality
  for JPG and non-lossless WebP, keep Transparency enabled for WebP, and pass
  the choice to the bridge.

## 4. Generated files and verification

- [x] 4.1 Update `Cargo.lock`, regenerate `flatpak/cargo-sources.json`, and
  regenerate `THIRD-PARTY-LICENSES`.
- [x] 4.2 Run `cargo fmt --all`, clippy, the codec/app/Qt tests, and
  `python3 scripts/check-test-plan.py`; then `openspec validate --all --strict`
  and archive the change.
