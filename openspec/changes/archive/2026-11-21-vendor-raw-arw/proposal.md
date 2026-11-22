# Proposal: vendor raw containers — Sony ARW (#1470)

## Why

Raw open (#86) reads DNG and Canon CR2 (#399 part 1). Sony is the next
container family (#1470, split out of #399). ARW is a TIFF container like
CR2, but it has no signature of its own, three sample codings, and on older
bodies its levels and white balance only exist in an enciphered private
block. Without a reader, File ▸ Open of an `.ARW` falls through to the raster
route and opens the embedded preview at best.

## What changes

- `sniff_raw` names a TIFF whose IFD0 `Make` begins with `SONY` (and that has
  no `DNGVersion`) as `RawFormat::Arw`; `sniff_raw_prefix` and so the app's
  open route recognise it from a file prefix. `read_raw` reads it into the
  same `DngImage` as every other container.
- ARW: the CFA image among IFD0's SubIFDs, read uncompressed (one sample per
  16-bit word), ARW2 block-compressed (compression 32767) through the tone
  curve of tag `0x7010`, or as tiles of lossless JPEG (compression 7, four
  components to a frame sample). Black, white and as-shot multipliers come
  from the raw IFD (tags `0x7310`, `0xC61D`, `0x7313`), else from the SR2
  private block, deciphered in-house (tags `0x7310`/`0x7300`, `0x787F`,
  `0x7313`/`0x7303`). The default crop is tags `0xC61F`/`0xC620`; a tiled
  plane is cut to tag `0x7038`'s sensor size.
- Refused as unsupported: ARW 1.0 compression (DSLR-A100), the YCbCr
  small/medium lossless raws, packed uncompressed samples and other
  compressions.
- Colour: the 11 Sony bodies rawtoaces-data publishes sensitivities for
  (ILCE-7M3, 7M4, 7RM2, 7RM3, 7RM4, 7SM2, 7SM3, 7CM2, ILCE-9, ILCE-6400,
  DSC-RX100M4) join the fitted matrix table, with rawtoaces' same-sensor
  aliases (ILCE-7RM3A, 7RM4A, 7RM5). Other Sony bodies read but keep being
  refused at open as unsupported camera colour.
- The Open dialog's Camera Raw and All Formats filters list `*.ARW`.

## Capabilities

- Modified: `codec/vendor-raw-containers`, `codec/camera-matrices`,
  `document/raw-open`.

## Provenance

Built from the TIFF 6.0, TIFF/EP and DNG specifications, ITU T.81, ExifTool's
public documentation of the Sony tags and the SR2 cipher, and observation of
real files (raw.pixls.us samples, kept outside the repository). The ARW2
decoder was checked by decoding one scene shot compressed and uncompressed on
the same body. No code or tables from dcraw, LibRaw or rawspeed. Matrices are
fitted from rawtoaces-data (Apache-2.0) by the existing generator.
