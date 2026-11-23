# Proposal: colour for Panasonic RW2 cameras (#1542)

## Why

The RW2 reader (#1472, #1541) decodes every observed Panasonic coding, but no
RW2 opens through File ▸ Open: the raw open refuses a camera without colour
matrices, RW2 files carry none, and rawtoaces-data publishes spectral
sensitivities for one Panasonic body only (DC-GX9). No other openly licensed
Panasonic sensitivities were found; Adobe, dcraw/LibRaw and Adobe DNG
Converter matrices are off-limits (#1465).

## Owner decisions (2026-11-23)

- Bodies without published sensitivities take matrices **fitted from the
  camera's own JPEG**: a script maps one sample raw file's white-balanced
  camera RGB to the JPEG the camera embedded in the same file. The result
  reproduces Panasonic's rendering and is labelled approximate.
- The change closes #1542; bodies still without colour move to a follow-up
  issue.

## What changes

- The camera-matrix table names each matrix's light (EXIF `LightSource`)
  instead of assuming CIE A and D65, so a row can carry the camera's own
  white-balance presets.
- New `scripts/fit-camera-jpeg-matrices.py` (with the
  `raw_jpeg_blocks` example that feeds it) writes
  `scripts/camera-jpeg-fits.json`; `scripts/gen-camera-matrices.py` adds the
  accepted fits and the DC-GX9's spectral fit to the table.
- The Open dialog lists `*.RW2`, `*.RWL` and `*.RAW` under Camera Raw and All
  Formats; routing stays by signature, so a Photoshop Raw `.RAW` is not sent
  to the raw open.

## Capabilities

- Modified: `codec/camera-matrices`, `document/raw-open`,
  `codec/vendor-raw-containers`.
