# Proposal

## Why

Opening a 0-byte `.jpg` through File ▸ Open crashed the app (#1650). The engine
probe classes an empty file as an unknown container, so it reaches
`decode_image_rgba`. There `QImageReader::size()` asks every installed image
plugin to recognise the device. The system KDE EXR plugin (`kimg_exr`) then
reads OpenEXR magic bytes that the empty device does not have, and the process
takes a SIGSEGV. Third-party plugins are outside our control, so the Qt decode
edge must not hand them input that cannot be an image. It also should not
probe them at all when the signature already names the format.

## What Changes

- `decode_image_rgba` refuses input shorter than 16 bytes before it builds a
  `QImageReader`. No supported raster format fits in fewer bytes.
- When the input starts with a PNG, JPEG, GIF, BMP, or TIFF signature, the
  reader's format is pinned to that format and content auto-detection is
  turned off. Only the matching handler is consulted.
- Input of 16 bytes or more with any other signature is still probed through
  every installed plugin, so ICO, PBM, XPM, SVG, TGA, and plugin formats keep
  opening.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `interop/image-import`: adds a requirement that the Qt decode edge refuses
  short input and pins known signatures.

## Impact

- `crates/pictura-app/cpp/decode_image.{h,cpp}`.
- New Qt Test cases in `tst_untrusted_input` and `tst_multi_file_open`.
- No dependency, build-system, or engine change. ROADMAP and PARITY coverage
  does not move.
