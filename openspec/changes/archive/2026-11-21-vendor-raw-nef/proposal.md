# Proposal: vendor raw containers, part 2 — Nikon NEF (#1469)

## Why

Raw open (#86) reads DNG and Canon CR2 (#399 part 1, #1462). Nikon bodies write
NEF, a TIFF container whose sensor plane is stored uncompressed or in Nikon's
own Huffman-coded prediction scheme. #1469 adds an in-house NEF reader so
NEFs reach the demosaic (#397), the camera colour stage (#398) and File ▸ Open
(#400) unchanged. NEFs embed no colour matrix, so the per-camera table of
#1465 gains the Nikon bodies whose spectral sensitivities are published, and
the maker note's as-shot white balance is read, so those NEFs open end to end.

## What changes

- `sniff_raw` names `RawFormat::Nef`: a TIFF without `DNGVersion` whose IFD0
  `Make` begins with `NIKON` and lists `SubIFDs`. `read_raw` reads it with the
  NEF reader; `sniff_raw_prefix` (the open route's test) agrees.
- NEF reader (`dng/nef.rs`, `dng/nef_decode.rs`): the CFA SubIFD; compression
  1 (12/14-bit packed most-significant bit first, or 16-bit containers in the
  file's byte order); compression 34713 as Nikon Huffman coding (lossless and
  lossy, with the maker note's linearization curve and split row) when the
  maker note carries the linearization table, otherwise as samples packed
  least-significant bit first. High-efficiency (13, 14) and small (8) NEFs
  are refused as unsupported.
- Levels: black from the maker note's `BlackLevel` (0x003D, on a 14-bit
  scale), white `2^bits − 1`; the whole plane is the active area.
- Colour: `scripts/gen-camera-matrices.py` seeds 11 Nikon bodies (D70, D200,
  D700, D800E (and D800), D810, D850, D3300, D5100, D5300, D7000, Z f (and
  Z 6II)); the as-shot neutral comes from the maker note's `WB_RBLevels`
  (0x000C).
- File ▸ Open lists `*.NEF` in Camera Raw and All Formats.

## Capabilities

- Modified: `codec/vendor-raw-containers` (dispatch; new NEF requirements),
  `codec/camera-matrices` (Nikon rows, NEF as-shot white balance),
  `document/raw-open` (NEF routed and listed).

## Provenance

The container comes from the TIFF 6.0, TIFF/EP and Exif specifications and
public maker-note tag write-ups (ExifTool's Nikon tag tables); byte orders,
packings and the black-level scale were established by observation on real
files. The decoder is written in-house. The six Nikon Huffman tables (code
counts and symbols) are interoperability constants of the format, the same in
every NEF decoder; by owner decision (#1469) they are taken as format facts,
noted in `assets/PROVENANCE.md`. No dcraw, LibRaw or rawspeed code is used.
Fixtures are synthetic, written by the tests; real camera files stay outside
the repository and are checked opt-in through `PICTURA_RAW_CORPUS`.
