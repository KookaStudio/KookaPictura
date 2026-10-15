# Proposal

## Why

Kooka Pictura advertises WebP import and export (Open/Place/Open As Smart
Object, Save As, Export As), but every raster import and export goes through
Qt: `QImage::fromData` for decode and `QImageWriter` for encode. Qt ships its
PNG/JPEG/GIF/BMP/TIFF plugins in `qtbase`, but the **WebP plugin lives in the
optional `qt6-imageformats` package** (`libqwebp`). On a system without that
package — including this project's own development and CI images — both opening
and saving a `.webp` fail silently, even though the header probe, the file
dialogs, and the format dispatch already list WebP. Issue #483 reports exactly
this: "unable to open webp files".

## What Changes

- **New native WebP codec in the engine.** `pictura-codec` gains
  `decode_webp(bytes, budget) -> Option<(Vec<u8>, u32, u32)>` and
  `encode_webp(rgba, width, height) -> Option<Vec<u8>>`, backed by the pure-Rust
  `image-webp` crate (the same crate the `image` meta-crate uses). Decode covers
  VP8 (lossy), VP8L (lossless), and VP8X (extended, with alpha); encode is
  lossless VP8L only, because no pure-Rust lossy WebP encoder exists.
- **WebP import no longer depends on Qt.** The app's decode edge routes a WebP
  container to `pictura_codec::decode_webp` and falls back to Qt only for the
  other formats. This fixes Open, Place, Open As Smart Object, and OS
  drag-and-drop for WebP with no runtime Qt plugin.
- **WebP export no longer depends on Qt.** `save`/`save_as` and `export_image`
  route a `WEBP` target to `pictura_codec::encode_webp` (after the boundary
  rescales, so the percent-scale contract is unchanged) and write atomically.
- **WebP in the Export As dialog.** `File ▸ Export As…` gains a `WebP` format
  entry that encodes each checked target through the engine (in memory, so the
  size estimate and the write share one encoder), with the transparency control
  enabled and the quality control off (the engine encoder is lossless).
- **One new dependency**, `image-webp = "0.2"` (MIT OR Apache-2.0, pure Rust),
  added to `pictura-codec`. It is the underlying WebP crate the `image`
  meta-crate wraps and is already named in `docs/01-architecture/file-formats.md`.

## Capabilities

### Modified Capabilities

- `interop/image-import`: the "application decodes images to RGBA8888 at the Qt
  boundary" requirement is narrowed to the non-WebP formats; a new requirement
  places WebP decode in the engine.
- `interop/raster-export`: the "raster output is encoded at the Qt boundary"
  requirement is narrowed to the non-WebP formats and its engine-encoder
  prohibition is carved out for WebP; a new requirement places WebP encode in
  the engine.

## Impact

- **Engine**: new `crates/pictura-codec/src/webp.rs` module and two `pub`
  re-exports; `image-webp` dependency in `crates/pictura-codec/Cargo.toml`.
- **App Rust**: `crates/pictura-app/src/cxxqt_object/helpers.rs::decode_import`
  routes WebP to the engine; `crates/pictura-app/src/cxxqt_object/export.rs`
  gains the WebP encode dispatch and the `scale_rgba` bridge declaration;
  `impl_core.rs::save` delegates its raster branch to a shared encode helper
  (kept within its file-size ceiling).
- **App C++**: `encode_image.{h,cpp}` gains `scale_rgba`, the boundary rescale
  extracted so the native WebP path can reuse it.
- **Dependencies**: `image-webp` (decode + lossless encode), stated above.
- **Ceiling**: WebP export is lossless only (no quality control); animated WebP
  imports frame 0, matching the existing first-frame rule for GIF/APNG.
