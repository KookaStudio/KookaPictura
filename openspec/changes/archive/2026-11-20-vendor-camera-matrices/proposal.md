# Proposal: per-camera colour matrices for vendor raw files (#1465)

## Why

Vendor raw containers (CR2 today, the rest as #399 lands them) carry no DNG
`ColorMatrix`, so the camera colour stage (#398) refuses them and File ▸ Open
(#400) reports "camera colour for <camera> is not supported yet". Only DNG
files open. dcraw/LibRaw's `adobe_coeff` table and the matrices Adobe DNG
Converter writes are Adobe-derived and cannot be used.

## What changes

- A generated table of per-camera XYZ → camera matrices, one under CIE
  illuminant A and one under D65, **fitted** from measured camera spectral
  sensitivities (AcademySoftwareFoundation/rawtoaces-data, Apache-2.0) by
  `scripts/gen-camera-matrices.py`. It is seeded with 14 of the dataset's
  Canon bodies that write CR2, plus their alias names (EOS 400D / Rebel XTi /
  Kiss X); the 600D waits for a CR2 CFA phase fix (#1476), and other families
  add rows as their readers land.
- A vendor reader fills `DngImage::color` from the table by make and model,
  with calibration illuminants A and D65, so `as_shot_transform` and
  `decode_raw_source` accept the file. A camera not in the table keeps the
  empty matrices and is refused with the #400 message.
- CR2 reports its as-shot white balance: the Canon maker note's colour data
  (tag `0x4001`) holds the camera's RGGB multipliers, at an index set by the
  array's length or version; they become `AsShotNeutral`. An unknown layout
  leaves it empty (D50).

## Capabilities

- New: `codec/camera-matrices`.
- Modified: `codec/vendor-raw-containers` (a CR2's colour tags are no longer
  empty).

## Provenance

Matrices are computed, not copied: least-squares fits of camera sensitivities
against the CIE 1931 observer over rawtoaces-data's 190-patch training set,
under CIE illuminant A (from its Planckian definition) and CIE D65 (CIE
dataset doi:10.25039/CIE.DS.hjfjmt59). No Adobe, dcraw, LibRaw or rawspeed
matrix or code is used. The colour-data layout comes from ExifTool's public
Canon tag documentation and was confirmed by observation on 16 real CR2s from
raw.pixls.us and the owner's samples (kept outside the repository). Parity with
Adobe Camera Raw is approximate.
