# Tasks

## 1. Engine WebP codec

- [x] 1.1 Add `image-webp = "0.2"` to `crates/pictura-codec/Cargo.toml` and
  update `Cargo.lock`; note the dependency rationale (Qt's WebP plugin is
  optional) in the change record.
- [x] 1.2 Add `crates/pictura-codec/src/webp.rs` with
  `decode_webp(bytes, budget) -> Option<(Vec<u8>, u32, u32)>` (VP8/VP8L/VP8X,
  first frame, Rgb8 expanded to RGBA, budget checked before allocation) and
  `encode_webp(rgba, width, height) -> Option<Vec<u8>>` (lossless VP8L). Export
  both from `lib.rs`. Verify with unit tests: a lossless round trip is
  byte-exact, an opaque decode gets alpha 255, a non-WebP/malformed input and an
  over-budget header return `None`, and a zero dimension or short buffer is
  refused.

## 2. WebP import at the app edge

- [x] 2.1 Route `helpers.rs::decode_import` to `pictura_codec::decode_webp` when
  the probe names `ImageFormat::WebP`, falling back to `decode_image_rgba` only
  when the native decoder returns `None`; keep the post-decode budget check
  shared. Verify with a `pictura-app` test that a WebP decodes through the
  native path (no Qt plugin) and a non-WebP container still reaches Qt.

## 3. WebP export at the app edge

- [x] 3.1 Extract the boundary rescale from `cpp/encode_image.cpp` into
  `scale_rgba(rgba, width, height, scale, out_width, out_height)` and declare it
  in `encode_image.h`; register it in the `export.rs` cxx bridge.
- [x] 3.2 Add `encode_raster` to `export.rs` dispatching `WEBP` to the engine
  (rescale via `scale_rgba`, encode via `pictura_codec::encode_webp`, temp file
  plus rename) and everything else to `encode_image_rgba`; have `export_image`
  and `impl_core.rs::save` call it, keeping `impl_core.rs` within its file-size
  ceiling. Verify a `.webp` save writes a file that reopens through the native
  decoder with matching pixels, and an unwritable path leaves no partial file.

## 4. WebP in the Export As dialog

- [x] 4.1 Add a `WebP` entry to `export_as_dialog.cpp` (format combo, suffix,
  `formatForPath`, `retargetPath`), enable the transparency control for it, and
  leave the quality control off (the engine encoder is lossless). Add a
  `webp_encode` bridge in `export.rs` so the dialog's in-memory encode and size
  estimate use the engine. Verify with `tst_export_as`: the combo lists WebP, its
  controls gate correctly, and `encoded`/`saveTo` write a RIFF/WEBP container.

## 5. Oracle and integration checks

- [x] 5.1 Add `crates/pictura-codec/tests/webp_oracle.rs`: encode a fixture with
  `encode_webp`, decode it with `magick` and compare through
  `pictura-testkit::compare`; also have `magick` write a lossy WebP and decode
  it with `decode_webp`, comparing to the source. Self-skip when `magick` is
  absent and include a check that a wrong output fails.
- [x] 5.2 Add a Qt Test case (new `tst_webp.cpp` or an existing suite) that
  saves a document to `.webp`, reopens it through `open_image`, and asserts the
  dimensions and a pixel survive the round trip with no Qt WebP plugin.

## 6. Verify and record

- [x] 6.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo nextest run --workspace`, `cargo test --workspace --doc`,
  `openspec validate --all --strict`, and `bash scripts/verify-full.sh`.
- [x] 6.2 Update `docs/dev/STATE.md` with the change record (commit carries
  `TASK-ALLOWS-DOCS`).
