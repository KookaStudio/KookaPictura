# Proposal: vendor raw containers — Olympus ORF (#1471)

## Why

Raw open (#86) reads DNG, CR2, NEF and ARW. Olympus and OM System bodies write
ORF (and ORI beside a high-resolution shot), a TIFF with its own magic that no
reader recognises (#1471, split out of #399).

## What changes

- `sniff_raw` names a file starting `IIRO`, `IIRS` or `MMOR` as
  `RawFormat::Orf`; `read_raw` reads it into the same `DngImage` as every
  other container.
- Sensor data: 16-bit words and the three 12-bit packings. The Olympus
  compressed stream is recognised and refused as unsupported (#1535).
- Levels, bit depth, crop and white balance come from the Olympus maker note
  (`OLYMP`, `OLYMPUS` and `OM SYSTEM` layouts).
- Colour: the maker note's camera → sRGB matrix becomes DNG-style colour and
  forward matrices (approximate). rawtoaces-data has no Olympus body, so the
  fitted table is not used.
- The Open dialog's Camera Raw and All Formats filters list `*.ORF` and
  `*.ORI`.

## Capabilities

- Modified: `codec/vendor-raw-containers`, `document/raw-open`.

## Provenance

Container, packings and tags: observation of real files (raw.pixls.us, kept
outside the repository) and ExifTool's public tag documentation. No code or
tables from dcraw, LibRaw or rawspeed.

No compressed decoder ships. The authoring session saw a web-search summary
quoting fragments of third-party decoders of that coding and opened none; a
separate agent limited to patents and samples stopped when it recognised
training memory of an existing decoder. Nothing from either is in this change
(#1535).
