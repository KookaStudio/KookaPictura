# Proposal: an in-house DNG sensor reader (#396)

## Why

Kooka cannot open camera raw files (#86). The pure-Rust raw decoders are
LGPL-2.1 and excluded by `deny.toml`, so the decoder is built in-house from the
public DNG specification, DNG first. This change is the first stage: read a
DNG's container and tags and produce its normalized sensor plane. Demosaic
(#397), the colour transform (#398) and the File ▸ Open route (#400) consume
it later; nothing in the app changes yet.

## What changes

- `pictura_codec::read_dng(bytes, budget)` parses a TIFF or BigTIFF container,
  little- or big-endian, requires `DNGVersion`, and finds the full-resolution
  CFA image (`NewSubFileType` 0, photometric 32803) in IFD0 or its SubIFDs.
- Strips or tiles decode for compression 1 (uncompressed, 8–16 bit, bit-packed
  below 16) and 7 (lossless JPEG, ITU T.81 process 14, predictors 1–7, any
  component count). Deflate (8), PackBits (32773) and lossy JPEG (34892), linear
  DNGs, floating-point samples and non-RGB CFAs are refused with
  `PsdError::Unsupported`; they are deferred.
- Levels: `LinearizationTable`, `BlackLevel` with `BlackLevelRepeatDim`,
  `BlackLevelDeltaH/V` and `WhiteLevel` map each sample to 0–65535, cropped to
  `ActiveArea`. The CFA pattern (`CFARepeatPatternDim`, `CFAPattern`,
  `CFAPlaneColor`) is reported with its origin at the active area.
- The result carries `DefaultCropOrigin/Size`, `Orientation`, make and model,
  and the colour tags (`ColorMatrix1/2`, `ForwardMatrix1/2`,
  `CameraCalibration1/2`, `CalibrationIlluminant1/2`, `AsShotNeutral`,
  `AnalogBalance`, `BaselineExposure`) for #398.
- Declared sizes are charged against `ImageBudget` before allocation; a hostile
  file fails with a typed error.

## Capabilities

- New: `codec/dng-raw-reader`.

## Provenance

Built from the DNG 1.6 specification and ITU T.81 only; no code or tables
from dcraw, LibRaw or rawspeed. Colour matrices come from the file's own tags.
