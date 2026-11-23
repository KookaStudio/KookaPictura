# Proposal: vendor raw containers — Panasonic RW2, plain packings (#1472)

## Why

Raw open (#86) reads DNG, CR2, NEF, ARW and ORF. Panasonic and the Leica
bodies Panasonic builds write RW2, RWL or RAW, a TIFF with its own magic and
tags that no reader recognises (#1472, split out of #399).

## What changes

- `sniff_raw` names a file starting `IIU\0` as `RawFormat::Rw2`; `read_raw`
  reads it into the same `DngImage` as every other container.
- Sensor data in three plain codings is decoded: 16-bit words, ten 12-bit
  samples in 16 bytes, and nine 14-bit samples in 16 bytes. Every other coding
  is recognised and refused by name (#1541).
- Levels, sensor borders, CFA, crop and the as-shot white balance come from
  IFD0's Panasonic tags.
- Decode only: the files embed no colour matrix and the fitted table has no
  row for these bodies, so File ▸ Open refuses them naming the camera (#1542).
  The Open dialog's filters are unchanged until such files open.

This is part of #1472 and does not close it.

## Capabilities

- Modified: `codec/vendor-raw-containers`, `document/raw-open`.

## Provenance

Container, tags and the three packings: observation of real files
(raw.pixls.us, kept outside the repository) and ExifTool's public tag
documentation. No code or tables from dcraw, LibRaw or rawspeed.

No other decoder ships. The authoring model has training exposure to
open-source decoders of those codings, so one it wrote would not be
independent (#1541). The research also printed the header of an
Adobe-converted DNG from the sample index, with truncated matrix values; the
file was deleted and nothing from it is used.
