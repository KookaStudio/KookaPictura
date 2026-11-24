# Proposal: colour for the remaining Canon CR3 bodies (#1574)

## Why

The CR3 reader (#1474) decodes every observed CR3 body, but only eight of
them (EOS R, RP, R5, R5 Mark II, R6, R6 Mark II, R10, 200D II and their
aliases) open through File ▸ Open: CR3 files carry no colour matrix, the raw
open refuses a camera without one, and rawtoaces-data publishes no further
Canon sensitivities at the pinned commit. Adobe, dcraw/LibRaw and Adobe DNG
Converter matrices stay off-limits (#1465).

## Owner decisions (2026-11-23)

- Bodies without published sensitivities take matrices **fitted from the
  camera's own JPEG**, the method #1542 introduced for Panasonic; a body whose
  sample is unusable but that shares its sensor with a fitted body takes that
  body's matrices (a same-sensor proxy). Both are labelled approximate.
- Scope: the CR3 bodies with a sample in the local corpus. Bodies still
  without colour move to #1579, and the change closes #1574.

## What changes

- `scripts/fit-camera-jpeg-matrices.py` reads CR3 files: the full-size JPEG
  track as the camera rendering, make and model from `CMT1`, and the Daylight,
  Cloudy and Tungsten presets of the `CTMD` maker note's colour data. It
  keeps the recorded fits of samples it is not given, so one maker can be
  refitted without the other's samples.
- `scripts/gen-camera-matrices.py` copies the accepted Canon fits and adds
  camera-JPEG proxies; 14 rows (15 bodies plus regional names) join
  `camera_matrices.rs`. No existing row changes.

## Capabilities

- Modified: `codec/camera-matrices`.
