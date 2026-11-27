# Proposal: Fujifilm SuperCCD and EXR RAF sensors (#1621)

## Why

The RAF reader (#1473) refuses every file whose sensor is neither plain
Bayer in a TIFF sensor section nor X-Trans: the SuperCCD bodies (FinePix
S2 Pro to S5 Pro, S100FS, S5600, S7000, S9500, E550, E900, F700), whose
photosites sit on a 45°-turned lattice in a sensor section of plain words,
and the EXR bodies (X10, X-S1, XF1, F550EXR, HS20EXR, HS50EXR, S200EXR),
which name a second directory at header offset `0x78`, a non-Bayer layout
record, or a word sensor section. None of them opens.

## What changes

- A sensor section that is not a TIFF is read as 16-bit words in the camera
  JPEG's byte order, sized by directory record `0x0100`, with black from
  record `0x4000` and the as-shot neutral from record `0x2FF0`.
- A second directory at `0x78` no longer refuses a file: the primary frame
  is read (owner decision), and a SuperCCD SR body that stores both frames
  side by side reads the left one. Merging the S and R frames into extended
  highlight range is a follow-up.
- A 2 × 2 layout record is read in reverse order, its low two bits naming
  red, green or blue, so the EXR layout `08 09 0B 0A` reads blue, green,
  green, red.
- A SuperCCD layout is repacked onto LibRaw's 45° grid, which is a regular
  2 × 2 CFA; the demosaic then rotates its camera RGB upright, as LibRaw's
  `fuji_rotate` does.
- 12-bit packed rows flagged `0xF006` = 1 (FinePix S1, SL1000) are read most
  significant bit first, as LibRaw reads them; they were misread as least
  significant bit first since #1473.
- Save Image writes a SuperCCD raw as a linear DNG, since a mosaic DNG cannot
  carry the turned lattice.
- The SuperCCD and EXR sample bodies take camera-JPEG-fitted colour matrices,
  their presets read from the directory records when there is no raw
  directory.

## Capabilities

- Modified: `codec/vendor-raw-containers`, `codec/raw-demosaic`,
  `codec/camera-matrices`, `document/raw-save-image`.

## Provenance

By owner decision, the SuperCCD geometry is an open, attributed port of
LibRaw 0.22.1 (LGPL-2.1 side of its LGPL-2.1/CDDL-1.0 dual licence,
distributed under GPL-3.0-or-later), as for the compressed RAF decoder
(#1620): the `0x0130` layout bits and `0x0121` size of `parse_fuji`
(`src/metadata/fuji.cpp`), the `fuji_width` grid of `identify`
(`src/metadata/identify.cpp`) and `raw2image` (`src/preprocessing/raw2image.cpp`),
and `fuji_rotate` (`src/postprocessing/aspect_ratio.cpp`). The port is
checked against LibRaw's own output, through rawpy, on raw.pixls.us sample
files kept outside the repository.
