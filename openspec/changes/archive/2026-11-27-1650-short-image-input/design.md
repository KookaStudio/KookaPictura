# Design

## Context

`decode_import` (`crates/pictura-app/src/cxxqt_object/helpers.rs`) sends every
container except WebP and PICT to `decode_image_rgba`. A container the engine
probe recognises only gets there after its header parsed. An unknown or empty
container gets there unchecked. `QImageReader` with default settings runs
`canRead()`/`capabilities()` on every plugin in `imageformats/`, including
system plugins such as kimageformats, and some of those trust the device to
hold their magic bytes.

## Decisions

- **16-byte floor.** The shortest signatures are 2 bytes, but a file that small
  holds no image of any format Qt or its plugins decode. 16 bytes covers the
  0-byte and 3-byte cases from the issue, plus every signature-only stub. The
  check runs before the `QBuffer` and `QImageReader` are built.
- **Pin known signatures.** PNG, JPEG, GIF, BMP, and TIFF get
  `setFormat(<name>)` + `setAutoDetectImageFormat(false)`. Qt then looks up only
  the handler keyed by that name and never runs the content scan over the
  other plugins. The signatures match `pictura_codec::probe`'s sniffer. WebP is
  not pinned because the engine decodes it, and Qt only sees it as a fallback.
- **Unknown signatures still probe.** Qt's runtime decoders stay the allow-list
  for formats the engine does not sniff (the existing image-import
  requirement). Restricting those to built-in handlers would drop formats that
  open today.
- The signature to format mapping is a small free function,
  `pictura::qtFormatForSignature`, declared in `decode_image.h`. The Qt Test
  checks the mapping directly, because which plugins get probed cannot be
  observed from outside.

## Risks

- A file that starts with a known signature but is really another format is
  now decoded only by the pinned handler. The engine probe already treats such
  a file as the signature's format, so nothing that opened before is lost.
- A missing Qt TIFF plugin now refuses TIFF instead of trying other plugins.
  The TIFF plugin ships with Qt Image Formats, which the app already needs.
