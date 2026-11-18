# Proposal

## Why

WebP export is lossless-only. `File ▸ Export As…` writes a VP8L stream whatever
the user chooses, so a user who expects a small lossy WebP gets a file several
times larger and has no quality control. Issue
[#807](https://github.com/KookaStudio/KookaPictura/issues/807) reports the gap
against Photoshop's Save for Web behaviour.

## What Changes

- Add a lossy VP8 encoder to the engine. `pictura-codec` gains
  `encode_webp_lossy(rgba, width, height, quality) -> Option<Vec<u8>>`, backed
  by the pure-Rust `gamut-webp` crate. The existing `encode_webp` stays the
  lossless VP8L path.
- Export As offers both modes. WebP gains a **Lossless** checkbox and enables
  **Quality** while it is unchecked. The default is lossy at quality 90;
  checking Lossless writes the same VP8L stream as today.
- Lossy WebP preserves transparency through the extended `VP8X` + `ALPH`
  container, so the Transparency control keeps working.
- `File ▸ Save As` to `.webp` stays lossless. It has no options dialog and
  keeps the exact engine round-trip contract in `interop/raster-export`.

## Capabilities

### Modified Capabilities

- `interop/raster-export`: Export As gains WebP lossy/lossless controls, and
  the engine WebP requirement gains the lossy entry point. Save As remains
  lossless.

## Impact

- **Engine**: `crates/pictura-codec/src/webp.rs` gains the lossy entry point;
  `crates/pictura-codec/Cargo.toml` gains `gamut-webp` and `gamut-core`.
- **App Rust**: the `webp_encode` bridge takes a quality and a lossless flag.
- **App C++**: `ExportAsDialog` gains the Lossless checkbox and the quality
  gate; `encodedAs` chooses the engine entry point.
- **Dependencies**: `gamut-webp` 0.3 and `gamut-core` 2 (MIT OR Apache-2.0,
  pure Rust, `#![forbid(unsafe_code)]`), stated in the change record.
- **Generated files**: `Cargo.lock`, `flatpak/cargo-sources.json`, and
  `THIRD-PARTY-LICENSES`.
- **Ceiling**: the lossy quality value maps coarsely onto VP8's base quantizer;
  the change does not claim Photoshop-identical quality or rate-distortion
  tuning.
