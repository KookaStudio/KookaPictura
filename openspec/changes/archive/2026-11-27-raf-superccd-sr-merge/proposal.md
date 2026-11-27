# Proposal: Merge SuperCCD SR frames for highlight range (#1648)

## Why

The FinePix S3 Pro, S5 Pro and F700 carry a second, low-sensitivity (R)
photosite beside every primary (S) one. #1621 reads their S frame only, as
LibRaw does by default, so everything above the S photosites' clip is lost,
which is the reason these bodies exist. Fujifilm's and Adobe's converters
merge the R frame into extended highlight range; Kooka Pictura should too.

## What changes

- A SuperCCD word-section RAF whose header names a second directory reads
  its R frame from the header's second sensor offset (`0x80`): a second
  section on the S3 Pro and S5 Pro, the second half of each row on the F700.
- The R frame is repacked onto the same LibRaw grid and merged with the S
  frame: below the S clip the S samples are kept; towards and above it the
  R samples, scaled by an S/R ratio fitted per image and per CFA colour
  (record `0x9200`'s nominal ratio, 16, when too few samples fit), take over.
- The merged range keeps only the headroom the scene needs, carried as
  `BaselineExposure`, so mid-tones develop as before and the saved DNG keeps
  the highlights.
- The header names a second directory only when the camera JPEG starts after
  `0x78`, as LibRaw checks; the FinePix S2 Pro's shorter header no longer
  reads JPEG bytes as one.
- The raw dialog's Exposure joins `BaselineExposure` in the develop's linear
  stage, before its clip, so negative Exposure recovers highlights above the
  baseline in every raw (owner decision). Unclipped pixels develop as
  before.

The merge is approximate: Adobe's and Fujifilm's merges are closed and
LibRaw reads one frame at a time, so there is no oracle.

## Capabilities

- Modified: `codec/vendor-raw-containers` (the second-directory test, the
  LibRaw corpus comparison), `codec/raw-demosaic` (the LibRaw corpus render).
- Added: `codec/vendor-raw-containers` (SR merge), `document/raw-open`
  (Exposure in linear light).
