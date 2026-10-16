# Proposal: save-for-web

## Why

Issue #61: import Save for Web from photorust
(`shell/src/dialogs/SaveForWebDialog.cpp`, `GifWriter.cpp`). File > Save for Web
& Devices (Ctrl+Alt+Shift+S) was an inert menu placeholder; Qt ships no GIF or
WBMP writer.

## What Changes

- `pictura_codec::web` (new): `quantize` — the Color Reduction methods
  (Perceptual, Selective, Adaptive as median-cut variants; Restrictive (Web),
  Black and White, Grayscale), 2–256 colours, Diffusion / Pattern / Noise
  dithering with an amount, transparency over a matte; `encode_gif` (GIF89a,
  global table, transparency, interlacing, hashed LZW); `encode_wbmp`.
- `cxxqt_object/web_export.rs` (new bridge): `web_flattened` (sRGB),
  `web_quantize`, `web_encode_gif`, `web_encode_wbmp`.
- `save_for_web_dialog.{h,cpp}` (new): Original / Optimized / 2-Up previews
  (4-Up disabled), CS6's presets, the format and its settings (GIF / PNG-8
  palette, JPEG quality / optimized / progressive, PNG-24 transparency, WBMP
  dither), matte, Image Size (W / H / percent, constrained, resampling), the
  optimised size and 56.6 Kbps time; Save writes it, Done keeps the settings
  for the session.
- File > Save for Web & Devices… is wired (enabled with a document).
- Tests: unit tests, an ImageMagick oracle (`web_oracle`) decoding our GIFs
  (plain and interlaced) and WBMP pixel for pixel, and `tst_save_for_web`.

## Capabilities

### New Capabilities

- `interop/save-for-web`: the Save for Web & Devices dialog and encoders.

## Impact

- `pictura-codec`, `pictura-app` (bridge, C++). No new dependency.

## Provenance

Ported from photorust's `SaveForWebDialog.cpp` and `GifWriter.cpp`, which left
palette and dither to Qt and searched the LZW table linearly. Adobe's
reduction methods are unpublished: Perceptual / Selective / Adaptive are
median-cut approximations. Ceiling (`ponytail:`): no 4-Up, slices / HTML
output, colour-table editing, Lossy, Web Snap, JPEG Blur, metadata choice,
gamma preview, or PNG interlacing; settings last for the session only.
