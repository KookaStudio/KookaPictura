# Design: vendor-raw-raf-superccd-exr

## Samples

Nineteen files, kept outside the repository: the S7000, E550 and XF1 the
owner holds, and one raw.pixls.us file each of the S2 Pro, S3 Pro, S5 Pro,
S100FS, S5600, S9500, E900, F700, F550EXR, HS20EXR, HS50EXR, S200EXR, X10,
X-S1, S1 and SL1000.

| Kind | Bodies | Sensor section | Layout `0x0130` |
|---|---|---|---|
| SuperCCD HR | S7000, E550, E900, S9500, S5600 | LE words | `80 81 82 83`, `83 82 81 80` |
| SuperCCD SR | F700 (frames side by side), S3 Pro, S5 Pro | LE words | `80 81 82 83`, `00 01 02 01`, `01 02 01 00` |
| SuperCCD | S100FS (LE), S2 Pro (BE, 12-bit) | words | `01 02 01 00`, `00 01 02 01` |
| EXR, plain Bayer | F550EXR, HS20EXR, S1, SL1000 | TIFF, 12-bit packed | `0A 0B 09 08` |
| EXR, plain Bayer | S200EXR | LE words | `0A 0B 09 08` |
| EXR layout | X10, X-S1, XF1, HS50EXR | TIFF, 12-bit packed | `08 09 0B 0A` |

## Layout record

LibRaw reads `fuji_layout = b0 >> 7` and `fuji_width = !(b1 & 8)` (a turned
lattice when bit 3 of the second byte is clear). For an upright 2 × 2
layout the four bytes, last first, list the CFA from the plane's origin;
the low two bits are 0 red, 1 or 3 green, 2 blue. That decode is ours: it
gives red, green, green, blue for `0A 0B 09 08` and blue, green, green, red
for `08 09 0B 0A`, which LibRaw's `raw_pattern` confirms on every upright
sample. `0C 0C 0C 0C` stays X-Trans.

## Word sections

The section has no TIFF signature. LibRaw reads it with `unpacked_load_raw`
as `0x0100`'s height by width 16-bit words in the byte order of the last TIFF
it parsed: the camera JPEG's Exif (big-endian only on the S2 Pro). With a
second directory and `fuji_layout` set (the F700) LibRaw multiplies the row
length by the frame count, so the row holds the S frame then the R frame;
the section length the header records covers one frame, so the data is
bounded by the file instead. All nineteen samples match LibRaw's
`raw_image` word for word.

Levels: record `0x4000` holds 4 or 8 big-endian values in green, red,
green, blue order (8: two sets, averaged as LibRaw does); the white is
LibRaw's `0x3E00` for these files, and 4095 for the big-endian S2 Pro's
12-bit words. The as-shot neutral is record `0x2FF0` (green, red, green,
blue), the presets records `0x2100` (Fine Weather) and `0x2400` (Tungsten).

## SuperCCD geometry (LibRaw port)

1. Active area: the inset (`0x0110`, `0x0111`) when it fits the plane;
   otherwise, for a plane 2944 wide without a `0xC000` record (the S2 Pro),
   2880 × 2144 centred with orientation 6; otherwise `0x0121`'s height
   shifted left and width shifted right by `fuji_layout`, centred.
2. `fw = w >> !fuji_layout`; the grid is `W = (h >> fuji_layout) + fw` wide
   and `W − 1` high. Photosite `(row, col)` of the active area goes to
   `(fw − 1 − col + row/2, col + (row + 1)/2)` with `fuji_layout`, else
   `(fw − 1 + row − col/2, row + (col + 1)/2)`, over LibRaw's loop bounds.
   The rest of the grid is black.
3. The grid's CFA is red, green, green, blue when `fw` is odd, and green,
   blue, red, green when even (LibRaw's `0x94949494` / `0x49494949`).
4. `DngImage::fuji_width` carries `fw`; the reader's default crop is the
   whole rotated image, whose size it computes with the demosaic's formula.
5. After the demosaic, `fuji_rotate`: with `f = fw − 1` and `s = √½`, an
   image `⌊f / s⌋` wide and `⌊(H − f) / s⌋` high samples the grid bilinearly
   at `(f + (row − col)·s, (row + col)·s)`, leaving pixels whose sample
   falls outside the grid's last row or column black.

A numpy prototype of steps 1–5 reproduced LibRaw's rendered size exactly,
with block-mean correlations of 1.0000 in each channel, on the S7000,
S5 Pro, S3 Pro, S2 Pro, F700, E900 and S100FS. (A first reading of
`0x49494949` as green, red, blue, green kept green exact and swapped red
and blue, which only a per-channel comparison shows.)

## Where the rotation lives

`demosaic` returns upright camera RGB for every caller (the open, the full
preview of Save Image, the fitter's example), so nothing downstream needs to
know about the lattice. Save Image's mosaic DNG cannot express the turned
lattice (DNG's staggered `CFALayout` values are not written by our writer),
so a SuperCCD raw is saved as a linear DNG whatever the checkbox says.

## Out of scope

- Merging SuperCCD SR's R frame for highlight range (follow-up issue).
- LibRaw's model exceptions not met by any sample (the S6000fd/S6500fd inset
  exception, the HS10 CFA override).
