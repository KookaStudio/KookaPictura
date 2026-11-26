# Proposal: vendor raw containers — Fujifilm RAF and X-Trans demosaic (#1473)

## Why

Raw open (#86) reads DNG, CR2, CR3, NEF, ARW, ORF and RW2. Fujifilm bodies
write RAF, a container with its own header that no reader recognises, and
most of them use the 6 × 6 X-Trans filter, which the demosaic only fills
bilinearly (#1473, split out of #399).

## What changes

- `sniff_raw` names a file starting `FUJIFILMCCD-RAW ` as `RawFormat::Raf`;
  `read_raw` reads it into the same `DngImage` as every other container.
- Sensor data: uncompressed 16-bit words and the 12- and 14-bit packings, for
  X-Trans and plain Bayer sensors. Fujifilm-compressed data is recognised and
  refused as unsupported (#1620); SuperCCD and EXR sensors are refused
  (#1621). Both by owner decision.
- Levels, crop, CFA and white balance come from the RAF directory and the
  sensor section's raw directory; make, model and orientation from the
  camera JPEG's Exif.
- Demosaic: `DemosaicMethod::Ahd` takes a new edge-directed X-Trans method of
  Pictura's own design on 6 × 6 X-Trans patterns instead of bilinear.
- Colour: RAF files embed no matrix, so twelve sample bodies get matrices
  fitted from their own camera JPEGs (by owner decision), placed under the
  file's two white-balance presets (Standard A and D65).
- The Open dialog's Camera Raw and All Formats filters list `*.RAF`.
- The develop lifts a RAF by the raw exposure bias the camera recorded (its
  dynamic-range modes underexpose the raw), as the camera's JPEG does.
- Pictura Raw's photo information reads a RAF's shooting data from its
  camera JPEG's Exif, and every raw file's size is the developed (cropped,
  oriented) size.

## Capabilities

- Modified: `codec/vendor-raw-containers`, `codec/raw-demosaic`,
  `codec/camera-matrices`, `codec/pictura-raw`, `document/raw-open`.

## Provenance

Container, packings and tags: observation of real files (raw.pixls.us, kept
outside the repository) and ExifTool's public RAF tag documentation. No code
or tables from dcraw, LibRaw or rawspeed. The X-Trans demosaic is designed
here, not taken from a published or third-party method.
