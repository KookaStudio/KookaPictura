# Design: vendor-raw-cr3

## Modules

| File | Role | Provenance |
|---|---|---|
| `dng/cr3.rs` | box walk, track choice, `CMT1`/`CMT3`/`CTMD`, levels, `DngImage` | ISO/IEC 14496-12, public write-ups; track choice and `CTMD` walk follow LibRaw's `cr3_parser.cpp` |
| `dng/crx.rs` | `CMP1` header, tile/plane/subband headers, quantizer tables, per-plane decode, encoding 3, layout | port of LibRaw `crx.cpp` |
| `dng/crx_lines.rs` | bit reader, Golomb-Rice line decoders, inverse quantization | port of LibRaw `crx.cpp` |
| `dng/crx_wavelet.rs` | inverse 5/3 lifting over up to three levels and tile edges | port of LibRaw `crx.cpp` |

The maker-note readers (sensor information, colour data) are the CR2
reader's, now `pub(super)`. No new dependency; rayon (present) decodes the
four planes in parallel, as LibRaw does under OpenMP.

## Port notes

- LibRaw streams each band from the file in 64 KiB buffers, a multiple of
  four, so a word read never straddles two buffers. The port reads the
  band's bytes as one slice, which gives the same bits. Where LibRaw reads
  past a band's end, the port fails with `Truncated`.
- LibRaw's signed 32-bit arithmetic wraps on hostile data. The port uses
  wrapping operations wherever a coded value flows, so a hostile file
  decodes to noise or an error and never panics. A Golomb parameter outside
  0..31, or a negative quantizer, is an error rather than undefined
  behaviour.
- A level's quantizer tables are kept in one run, as LibRaw allocates them,
  because a band's row index may run into the next level's table.
- Band and level line buffers carry two guard values. LibRaw's buffers are
  packed contiguously, and a tile-edge read of one value past a band lands
  in the next band; in the port it lands in the guard. No real file reads
  there (see Verification).
- In the even-height tail of a level, LibRaw aliases rows 1 and 2 of the
  level. Neither is read again, so the port does not.
- The rounded-line coding (plane header bits 1–2, LibRaw's
  `crxDecodeLineRounded`) is ported, but no observed file uses it and no
  test covers it.
- Encoding 1 (signed samples) is refused: no still camera is known to
  write it, and its values do not fit an unsigned CFA plane.

## Levels and colour

As for CR2: the black is the masked-border mean per CFA position, and the
white is the coding's ceiling (`2^bits − 1`, or `2^median_bits − 1` for
encoding 3). The camera's saturation level is in ColorData, and the reader
does not use it (a `ponytail:` note in `read_cr3`, shared with CR2). The
as-shot neutral is in the `CTMD` track (record type 8), not in `CMT3`.

## Survey

raw.pixls.us, 2026-11-23: 110 CR3 files from 29 bodies. 52 of them were
downloaded (every body, RAW and C-RAW, Dual Pixel and crop modes). All are
encoding 0 and colour layout 0 at 14 bits. Lossless files are level 0 with
one or two tiles. C-RAW files are three-level: version 1 with per-band
quantizers (M50, 90D, 250D, R, RP, R5, R6, …), or version 2 with per-block
tables (1D X Mark III, R5 Mark II, PowerShot V1, …). Each file holds a
small CRAW preview track besides the full one, and the largest track wins.

## Verification

Synthetic files: `tests/camera_raw/raw/crx_encode.rs` mirrors the decoder's line coder
to write lossless and no-reference lines with runs, applies a textbook
forward 5/3 transform for the wavelet scenarios, and checks quantized
scenarios against a textbook inverse. `tests/camera_raw/raw/cr3_builder.rs` writes the
container. The LibRaw comparison is a one-off development check through
rawpy (LibRaw 0.22.1), which CI does not install: every sample's
`raw_image` was compared with the port's plane before levels.

## Patents (OVR-004)

Recorded in `assets/PROVENANCE.md`, status only; not a legal conclusion.
