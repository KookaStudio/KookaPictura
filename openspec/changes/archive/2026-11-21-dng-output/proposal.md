# Proposal: DNG output (#402)

## Why

ACR "cannot save a camera raw file; it saves copies as DNG, JPEG, TIFF, or
PSD" (WF-012). Pictura opens DNG and CR2 files (#396–#400) and keeps their
settings (#401), but cannot write a DNG: there is no way to convert a CR2 to
the open container, to hand a raw file's develop settings to another raw tool
inside the file, or to keep a demosaiced (linear) copy. #402 asks for DNG
output matching ACR's subset, with the tags the reader round-trips and an
embedded JPEG preview, readable by our reader and a reference tool.

## What changes

- **DNG writer** (`pictura_codec::write_dng`). A little-endian TIFF/DNG with
  a 256-pixel 8-bit RGB thumbnail in IFD0 and, in SubIFDs, the full-resolution
  image and an optional JPEG preview. Two image kinds:
  - **mosaic** — the CFA plane (normalized 16-bit samples, black 0, white
    65535), lossless-JPEG tiles of 256 × 256;
  - **linear** — demosaiced camera RGB (`LinearRaw`), three-component
    lossless-JPEG tiles.
  Both carry the colour tags (`ColorMatrix1/2`, `ForwardMatrix1/2`,
  `CameraCalibration1/2`, `CalibrationIlluminant1/2`, `AnalogBalance`,
  `AsShotNeutral`, `BaselineExposure`), the default crop, the orientation, make
  and model, a `crs:` XMP packet, and optionally the original raw file
  (`OriginalRawFileName`, `OriginalRawFileData`: 64 KiB zlib blocks, the DNG
  layout). A compatibility level (Camera Raw 2.4, 4.1, 4.6, 5.4, 6.6 and
  later) sets `DNGVersion` / `DNGBackwardVersion` and drops tags newer than it.
- **Linear DNG reading.** `read_linear_dng` reads a three-sample `LinearRaw`
  image (uncompressed or lossless JPEG, 8–16 bit, per-sample black and white
  levels, active area), and `decode_raw_source` uses it without a demosaic, so
  a linear DNG we write opens again.
- **Save Image… in the raw open's Pictura Raw dialog.** A button opens
  **Save Options** (folder, file name, Compatibility, JPEG Preview None /
  Medium Size / Full Size, Embed Original Raw File, Convert to Linear Image);
  Save writes the DNG on a worker under a cancellable progress window and
  reports the result in the dialog, which stays open. The DNG embeds the
  dialog's current settings; the previews are developed with them.
- **Embed settings in DNGs we write only.** An existing DNG is still never
  modified (#401); the Camera Raw settings stores are not written by Save
  Image.

## Out of scope (follow-ups)

- `Export Settings To XMP` and rewriting settings inside an existing DNG
  (`Update DNG Previews`) — a follow-up issue.
- JPEG, TIFF and PSD from Save Options, file-naming templates, Workflow
  Options and multi-image saves (#633).
- Lossy DNG (DNG 1.4 / ACR 7.1), floating-point output, EXIF/maker-note copy,
  `OriginalRawFileDigest`, raw-value preservation (the mosaic is stored
  normalized).

## Capabilities

- New: `codec/dng-writer`, `document/raw-save-image`.
- Modified (ADDED requirements): `codec/dng-raw-reader` (linear DNG),
  `codec/raw-develop` (linear source).

## Provenance

Built from the public DNG 1.4 specification's tag list and ITU T.81 (lossless
JPEG process 14). The `OriginalRawFileData` block layout follows the DNG
specification as decoded by ExifTool's `Image::ExifTool::DNG`
(`ProcessOriginalRaw`). The Save Options controls and compatibility names are
those of ACR 7's Save Options dialog; the mapping of each level to a DNG
version is approximate and marked as such.
