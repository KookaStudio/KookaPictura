# Design: Nikon NEF reader

## Container

A NEF is a classic TIFF in either byte order. IFD0 is a small thumbnail; its
`SubIFDs` hold a JPEG preview and the sensor image (`NewSubfileType` 0,
photometric 32803, TIFF/EP `CFARepeatPatternDim`/`CFAPattern`). The reader
takes the first SubIFD with subfile type 0 and photometric CFA; the CFA comes
from those tags through the DNG reader's `cfa_pattern`.

Sniffing reads only IFD0's entry table plus the `Make` value
(`Tiff::ascii_in`), so the open route's 256 KiB prefix suffices. Requiring
`SubIFDs` keeps Nikon scanner TIFFs (make `Nikon`, no SubIFDs) on the raster
route.

The Nikon maker note (Exif tag 0x927C) starts `Nikon\0`, two version bytes and
two pad bytes, then an embedded TIFF header at byte 10 whose offsets are
relative to that header; it is parsed as its own `Tiff`. Any other maker-note
shape counts as absent.

## Sensor data

| Compression | Maker note 0x0096 | Layout |
|---|---|---|
| 1 | — | strips of 16-bit containers in file order when a strip holds 2 bytes per sample, else packed MSB-first at `BitsPerSample` (the DNG reader's `decode_plane`) |
| 34713 | present | Nikon Huffman (`nef_decode::decode_huffman`) |
| 34713 | absent | 16-bit containers as above, else packed LSB-first (D850 12-bit "uncompressed") |

Observed: D200 12-bit (MSB packed, 11-row strips), D5100/D800E 14-bit (16-bit
big-endian containers), D850 14-bit (16-bit little-endian), D850 12-bit
uncompressed (34713, LSB-first; its samples are ¼ of the 14-bit file's).

### Huffman scheme

Linearization table 0x0096 (in the maker note's byte order): bytes 0–1 the
version, 2–9 four initial vertical predictors `[row parity][column]`, 10–11 the
curve size. Version byte 0 = 0x46 is lossless (identity curve, lossless
table). Otherwise lossy: version 0x44 0x20 lists the curve at every
`2^bits / (size − 1)` input and interpolates linearly between, and its split
row is the 16-bit value at byte 562; other versions list `size` curve entries
directly. Tables: lossy, lossy after split, lossless, for 12 then 14 bits.

Each sample's symbol `s` gives a length `len = s & 15` and dropped low bits
`shl = s >> 4`; `len − shl` bits follow. With `shl` 0 the difference is the
ITU T.81 `EXTEND` of those bits; with `shl > 0` the dropped bits are restored
at their midpoint and a clear top bit subtracts `2^len`. Columns 0 and 1 of a
row predict from the running vertical predictor of the row's parity (which
takes the difference), later columns from the previous sample of the same
colour two columns left. The output is the curve at the prediction clamped to
the curve. The bitstream is continuous across rows with no byte stuffing
(`ljpeg::Bits::plain`).

## Levels and colour

- Black: maker note 0x003D, four values per 2 × 2 position on a 14-bit scale,
  times `2^(bits − 14)` (D850: 400 → 12-bit samples sit at 102, 14-bit at 409).
  Absent → 0 (older bodies subtract it in camera).
- White: `2^bits − 1` (the same ponytail as CR2: not the sensor saturation).
- Active area and default crop: the whole plane.
- As-shot neutral: 0x000C `WB_RBLevels` `[R, B, …]` → `(1/R, 1, 1/B)`.
- Matrices: the generated table keyed `NIKON` + model without the leading
  `NIKON` word (`NIKON D850` → `D850`, `NIKON Z f` → `Z F`).

## Alternatives

- Reading `NEFCompression` (0x0093) to pick the layout: present on fewer
  bodies than the linearization table (absent on the D200/D800E samples) and
  overlapping in meaning; it is used only to refuse small and high-efficiency
  NEFs.
