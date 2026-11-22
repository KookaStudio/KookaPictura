# Design: vendor-raw-arw

## Modules

Inside `crates/pictura-codec/src/dng/`, beside the CR2 reader:

- `arw.rs` — sniffing, the raw IFD, coding dispatch, levels, white balance,
  crop. Uncompressed data go through the DNG reader's `layout`/`decode_plane`,
  lossless tiles through `decode_quad_tiles`; the levels through
  `levels::normalize`.
- `arw_decode.rs` — pure functions: the ARW2 block decoder, the tone curve,
  and the SR2 decipher.
- `tiff.rs` gains `ascii_in` (read one ASCII value from an IFD's entry table
  in a file prefix); the same helper the NEF reader (#1469) adds.

## Signature

ARW has the plain `II*\0` header. A TIFF without `DNGVersion` whose IFD0
`Make` begins with `SONY` is named ARW; every Sony TIFF (SR2, SRF, ARW 1.0
included) therefore reaches the reader and is refused by name rather than
opened as a raster TIFF. The app reads a 256 KiB prefix; `Make` sits in the
first kilobyte of every observed file.

## Raw IFD

The full-resolution IFD (`NewSubfileType` 0) found the DNG way: IFD0, its
SubIFDs, then the chain. It must be `PhotometricInterpretation` 32803 with one
sample per pixel; the A7 IV's medium and small lossless raws are YCbCr (6)
with three samples and are refused. CFA from `CFARepeatPatternDim`/`CFAPattern`
(2 × 2 only), RGGB without them.

| Compression | Data | Observed |
|---|---|---|
| 1 | one sample per little-endian 16-bit word, any strips | A7 III/IV, A7R II/III/IV; 12- or 14-bit values (the A7R IV tags 12 bits but writes the 14-bit scale; its levels say so) |
| 32767, strips ≥ w·h bytes | ARW2: rows of 16-byte blocks, one byte per photosite | A7 III, A7 IV, A7R II |
| 32767, shorter strip | ARW 1.0 (DSLR-A100) | refused |
| 7 | 512 × 512 tiles of lossless JPEG, SOF3 256 × 256 × 4 at 14 bits, predictor 1 | A7 IV, A1 |

**ARW2 block.** 32 photosites of a row are two blocks: the even columns,
then the odd. A block read as a 128-bit little-endian number: max (11 bits),
min (11), index of max (4), index of min (4), then fourteen 7-bit deltas for
the other positions in order. Shift `s` is the smallest in 0..4 with
`128 << s > max − min`; a value is `min + (delta << s)`, capped at 2047.
Width must be a multiple of 32 (every observed body).

**Tone curve.** Tag `0x7010` holds four knees. Over half-code steps
`j = 0..4095` the curve rises with slope 1, 2, 4, 8, 16 between `0, k₁/4,
k₂/4, k₃/4, k₄/4, 4095`; code `c` reads `curve[2c]`, giving the 14-bit scale
directly (black 512). The table goes in `Levels::linearization`, so codes
stay in the plane and the curve is applied with the levels. Checked: the
A7R II's 12-bit compressed and 14-bit uncompressed files of one scene give
per-position means 1247/2488/2487/1816 against 1245/2481/2480/1813.

**Lossless tiles.** Each frame sample's four components are one 2 × 2 quad
of photosites, row-major (`decode_quad_tiles`). Placing them as two
consecutive sensor rows, as the DNG reader's tile path would, gave the ILCE-1
flat per-position means (8075/8087/8048/8060) and three times the
neighbour step of its compressed twin; the quad placement restores the CFA. The plane is padded to whole tiles; tag `0x7038` (e.g. 7032 × 4688 in a
7168 × 5120 plane) is the active area.

## Levels and white balance

Read from the raw IFD first, then the SR2 block, field by field:

- black: `0x7310` (4 values, one per 2 × 2 position), else `0x7300`; else 0.
- white: `0xC61D` (16383 on recent bodies; data saturates there), else the
  smallest of the SR2's `0x787F` (15360, conservative); else the sample
  precision's ceiling (the tone curve's top for ARW2).
- as-shot: `0x7313` (R G G B), else `0x7303` (G R B G); the neutral is
  `(G/R, 1, G/B)` with `G` the green mean.

**SR2 block.** IFD0 `DNGPrivateData` (`0xC634`, 4 bytes in file order) points
at the SR2Private IFD: `0x7200` offset, `0x7201` length and `0x7221` key (4
bytes in file order) of the SR2SubIFD. The block is deciphered as big-endian
32-bit words XORed with a key stream: four words seeded by `k ← k·48828125 +
1`, the fourth folded with the first and third (`w₃ = w₃<<1 | (w₀^w₂)>>31`),
then `w[p] = (w[p−4]^w[p−2])<<1 | (w[p−3]^w[p−1])>>31` to 127 words, and per
data word `w[p&127] = w[(p+1)&127] ^ w[(p+65)&127]` from `p = 127`. The
deciphered SR2SubIFD's offsets count from the file start, so the reader
copies the file up to the block's end (refused beyond 4 MiB) and parses it
there. Observed key on every file: bytes `11 22 33 44`.

## Crop

`DefaultCropOrigin`/`DefaultCropSize` (`0xC61F`/`0xC620`) relative to the
active area (e.g. 12, 12, 6000 × 4000 in 6048 × 4024); the whole area
without them (the A7R II).

## Colour

`camera_color` with make `SONY` and the Exif model (`ILCE-7M3` → key
`ILCE 7M3`). `scripts/gen-camera-matrices.py` gains the 11 Sony stems;
rawtoaces' aliases add the 7RM3A, 7RM4A and 7RM5. SR2's own `0x7800` matrix
is a camera → output matrix of unknown basis and is not read.

## Decisions

- **Owner (#1470)**: uncompressed, ARW2 and lossless in one PR, ARW 1.0
  refused; the SR2 block deciphered in-house from ExifTool's description;
  11 Sony bodies seeded; branched from master, rebased after NEF (#1524).
- **Ponytails**: one black per 2 × 2 position; ARW2 is single-pass per row
  (rows in parallel); an unknown Sony body reads but is refused at open
  until it has a matrix.
