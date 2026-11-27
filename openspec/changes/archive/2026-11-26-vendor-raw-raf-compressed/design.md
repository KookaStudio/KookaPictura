# Design: vendor-raw-raf-compressed

## Module

`crates/pictura-codec/src/dng/raf_compressed.rs`, ported from LibRaw 0.22.1
(`fuji_compressed.cpp`, commit b860248a); `raf.rs` dispatches to it.

## Detection

When the stored size is not two bytes per photosite, `raf.rs` looks for the
compressed header at the data offset (`0xF007`). With it, the header's
width, height and precision replace the raw directory's, and are charged
against the image budget before the plane is allocated. Without it, the
uncompressed codings are tried as before.

## Data layout

Observed on 13 raw.pixls.us files (X-H1, X-T5, X-S10, X-E4, X-H2,
GFX 50S II, GFX100S); every value is big-endian:

| Offset (from the data) | Value |
|---|---|
| 0 | `IS` (`0x4953`) |
| 2 | 1 lossless, 0 lossy |
| 3 | 16 X-Trans, 0 Bayer |
| 4 | precision: 12, 14 or 16 |
| 5 | height |
| 7 | width rounded up to whole strips |
| 9 | width |
| 11 | strip width, always `0x300` |
| 13 | strips |
| 14 | lines of six rows |
| 16 | 32-bit strip sizes, padded to 16 bytes |
| then, lossy only | one quantizer base per strip and line, each strip's run padded to 16 |
| then | the strips' bit streams, back to back |

Header checks are LibRaw's: height a multiple of 6 up to `0x4002`, width a
multiple of 24 from `0x300` to `0x4200`, the rounded width a whole number of
strips less than one strip wider, lines equal to height / 6. A header whose
sensor type disagrees with the layout record (6 × 6 against 2 × 2) is
invalid.

## Decode

Each strip is 768 photosites wide (the last one narrower) and decodes six
rows at a time into 18 line buffers (five red, eight green, five blue, two
of each holding the previous rows). A line runs six passes over two buffers
each; X-Trans passes interpolate some even positions from the rows above
instead of reading them. Samples are coded as adaptive Golomb-like codes:
leading zeros, then a gradient-sized remainder, or an escaped raw value.
Lossy data rebuilds the main quantizer when a line's base changes and uses
three fixed fine quantizers where the local gradient is small. The rows are
copied out through the CFA: X-Trans positions map to buffer indices by
LibRaw's `(2x/3 & ~1) | (x mod 3 & 1)) + (x mod 3) / 2`, Bayer by `x / 2`.

The decode is table-driven where LibRaw unrolls: the six passes are a
table of (lines, even-position rule, gradient set). All strips decode one
line at a time in parallel, so the only full-size allocation is the plane.

## Bit reader

MSB first. Past a strip's bytes one zero byte reads; moving beyond it fails
as truncated, as LibRaw's buffer refill does. LibRaw only warns about a
code outside its range; the port, like it, decodes it.

## Verification

- A test encoder (`tests/camera_raw/raw/raf_encode.rs`) runs the same
  prediction and adaptation and picks each code, lossless and lossy, so
  synthetic files decode to a known plane.
- `compressed_rafs_match_libraw_when_present` decodes each compressed RAF in
  `PICTURA_RAW_CORPUS` and compares it with LibRaw's raw plane through
  rawpy, normalized alike: every sample must be equal.
