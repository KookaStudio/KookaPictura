# Design

## Context

Raster import and export both sit at the Qt boundary today:

- Import: `helpers.rs::decode_import` probes the header (Qt-free), then calls
  `decode_image_rgba`, which is `QImage::fromData` in
  `cpp/decode_image.cpp`.
- Export: `save` and `export_image` flatten the composite to packed RGBA8888,
  then call `encode_image_rgba`, which is `QSaveFile` + `QImageWriter` in
  `cpp/encode_image.cpp`.

Qt's runtime decoders/writers are the authoritative allow-list. That works for
PNG/JPEG/GIF/BMP/TIFF because their plugins ship in `qtbase`; WebP is a
separate, optional package. The bug is environmental but the fix must be
in-code so WebP works on any supported install.

## Goals / Non-goals

- **Goal**: WebP opens (Open, Place, Open As Smart Object, drag-and-drop) and
  saves (Save, Save As, Export As) with no Qt WebP plugin present.
- **Goal**: keep the engine Qt-free and keep the boundary responsible for the
  scale/atomic-write contract.
- **Non-goal**: replace the Qt path for the other raster formats. They work;
  changing them is a larger, unrequested refactor.
- **Non-goal**: lossy WebP export. No pure-Rust lossy encoder exists; adding a
  `libwebp` FFI binding would add a C dependency for a format whose lossless
  mode already satisfies the capability.
- **Non-goal**: WebP animation. The import edge already imports frame 0 of a
  multi-frame file; WebP follows the same rule.

## Decisions

### D1 — `image-webp` over the `image` meta-crate

`docs/01-architecture/file-formats.md` names the `image` crate as the intended
Rust raster baseline, but the app only needs WebP, and `image` pulls its whole
codec set (PNG/JPEG/TIFF/GIF/…), duplicating the Qt path. `image-webp` is the
pure-Rust crate `image` delegates to for WebP; it is MIT OR Apache-2.0 (allowed
by `deny.toml`) and adds only `byteorder-lite` and `quick-error`. Use it
directly.

### D2 — Decode in the engine, dispatched from the app edge

`pictura_codec::decode_webp(bytes, budget)` returns packed RGBA8888 and
dimensions. It validates the declared dimensions against `budget` before
allocating, sets the decoder's memory limit, expands a no-alpha (Rgb8) decode to
RGBA8888, and returns `None` on any malformed container. `decode_import` keeps
the probe as the fast-path guard: when the probe names `ImageFormat::WebP` it
calls the native decoder and falls back to Qt only if that returns `None`; every
other container still goes to Qt, so an unknown-but-Qt-decodable file is
unchanged.

### D3 — Encode in the engine, rescale at the boundary

`pictura_codec::encode_webp(rgba, width, height)` writes a lossless VP8L
container. The percent scale is a Qt concern (`SmoothTransformation`, matching
the other formats), so the boundary gains `scale_rgba`, extracted from
`encode_image_rgba`, which returns the rescaled RGBA8888 and its dimensions. The
app's `encode_raster` helper then: for `WEBP`, rescale via the boundary and
encode with the engine, writing temp-file + rename; for anything else, call
`encode_image_rgba` unchanged.

### D4 — Spec carve-out

Both `interop/image-import` and `interop/raster-export` currently make Qt the
sole raster codec and forbid an engine encoder. That is exactly what broke WebP,
so the requirements are narrowed: Qt remains the codec for every format except
WebP; WebP is decoded and encoded by the engine because its Qt plugin is
optional. The engine stays Qt-free either way.

### D5 — Export As dialog format list is unchanged

`export_as_dialog.cpp` deliberately offers a CC-parity subset (PNG, JPG, PNG-8,
GIF, SVG) and already omits TIFF and BMP as well as WebP. WebP is reachable the
same way TIFF and BMP are — the Save As path's extension and the
`export_image` bridge — so WebP is at parity with those formats without touching
the dialog. Adding WebP (but not TIFF/BMP) to the combo would be an inconsistent
partial change; reconciling the whole list with the spec is separate work.

## Risks

- **Lossy WebP files**: handled by the decoder (VP8), so opening a normal
  lossy `.webp` works; only *writing* lossy is unavailable.
- **Attacker-declared dimensions**: the probe caps declared dimensions and the
  engine decoder re-checks against the same budget and sets a memory limit, so a
  crafted header cannot force an unbounded allocation.
- **Determinism**: `image-webp`'s lossless encoder is deterministic; the oracle
  compares our encode through an independent decoder (`magick`).
