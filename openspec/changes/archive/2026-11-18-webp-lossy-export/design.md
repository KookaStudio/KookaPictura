# Design

## Context

`crates/pictura-codec/src/webp.rs` decodes VP8/VP8L/VP8X through `image-webp`
and encodes VP8L only. `image-webp` has no lossy encoder. The Export As dialog
therefore disables Quality for WebP and writes a lossless file (#807).

The app has two WebP writers:

- `File ▸ Save As` / `export_image` go through `encode_raster`, which writes a
  temp file and renames.
- Export As encodes per-entry bytes in memory through the `webp_encode` bridge,
  so its size estimate and its write share one encoder.

## Goals / non-goals

- **Goal**: Export As offers a lossy WebP with a quality setting and a lossless
  WebP, both with alpha.
- **Goal**: keep the engine Qt-free and keep the atomic-write contract.
- **Non-goal**: make Save As lossy or add a WebP Save As options dialog. Save As
  keeps the exact round trip specified today.
- **Non-goal**: match Photoshop's encoder bit for bit. VP8 quality tuning is
  upstream work; the tests assert decodable output, alpha, and size ordering.

## Decisions

### D1 — `gamut-webp`, not a `libwebp` FFI

The issue suggests a `libwebp` binding. `gamut-webp` 0.3 is a better fit for
this tree: pure Rust with `#![forbid(unsafe_code)]`, MIT OR Apache-2.0 (allowed
by `deny.toml`), VP8 lossy and VP8L lossless, and `VP8X`/`ALPH` alpha. It needs
no C toolchain, so the CI, AppImage, Flatpak and package builds do not gain a
native build step or a runtime `libwebp`. Upstream validates both directions
against `libwebp`.

Keep `image-webp` for decode and for the lossless encoder. The lossless bytes
therefore do not change, and the existing Save As round-trip tests and golden
files stay valid.

### D2 — Two codec entry points

- `encode_webp(rgba, width, height)` — lossless VP8L, unchanged.
- `encode_webp_lossy(rgba, width, height, quality: u8)` — VP8 lossy, quality
  clamped to 0..=100, alpha through `ALPH`, `None` for a zero dimension or a
  buffer that is not exactly `width * height * 4`.

Both validate before allocating and never panic. The lossy function builds a
`gamut_core::ImageRef<Rgba8>` and calls `gamut_webp::WebpEncoder::lossy`.

### D3 — Export As controls

Add `exportAsLossless`, a checkbox in File Settings, visible only for WebP.
Quality is enabled for JPG and for WebP while Lossless is unchecked. WebP
defaults to lossy at quality 90 (the spin box already defaults to 90). The
Transparency control stays enabled for WebP in both modes.

The `webp_encode` bridge gains `quality: i32` and `lossless: bool`. A lossless
request or a non-positive quality selects `encode_webp`; otherwise
`encode_webp_lossy` receives the clamped quality.

### D4 — Save As stays lossless

`encode_raster` keeps calling the lossless encoder for `WEBP`. Save As has no
format dialog (`interop/save-format-options`), and
`interop/raster-export` requires the reopened pixels to match the composite.
Changing that would be a separate capability change.

### D5 — Atomic writes

The Export As path already writes bytes through `ProtectedOutput`; the Save As
path already writes temp file plus rename. The lossy path changes only which
bytes are produced, not how they land.

## Risks

- `gamut-webp` 0.3 is young. The Rust tests round-trip through `image-webp`,
  and the oracle test decodes our output with ImageMagick. A malformed or
  undecodable stream fails the suite rather than shipping silently.
- The quality-to-quantizer mapping is coarse. Tests assert ordering (lower
  quality is smaller) rather than a fixed size.
