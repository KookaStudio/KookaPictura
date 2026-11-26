# Design: vendor-raw-raf

## Modules

Inside `crates/pictura-codec/src/dng/`:

- `raf.rs`: signature, header sections, directory records, sensor-data
  layouts, the reader.
- `demosaic/xtrans.rs`: the X-Trans demosaic.
- `vendor.rs`: `RawFormat::Raf`, sniff and dispatch.

## Container

The header is big-endian. Observed on 15 raw.pixls.us files (FinePix X100
to X-T5, GFX 50S) and the owner's FinePix S7000 and XF1:

| Offset | Value |
|---|---|
| `0x00` | `FUJIFILMCCD-RAW ` |
| `0x1C` | camera name, 32 bytes, NUL-padded |
| `0x54`, `0x58` | camera JPEG offset, length |
| `0x5C`, `0x60` | directory offset, length |
| `0x64`, `0x68` | sensor section offset, length |
| `0x78` | a second directory's offset: non-zero only on EXR bodies |

The directory is a 32-bit record count, then records of a 16-bit tag, a
16-bit size and the value. Used: `0x0110` crop top and left, `0x0111` crop
height and width, `0x0130` layout, `0x0131` X-Trans pattern (36 bytes).

The sensor section is a TIFF whose IFD has one entry, `0xF000`, pointing at
the raw directory (offsets from the section start): `0xF001` width, `0xF002`
height, `0xF003` bits, `0xF005` row length in 32-bit words, `0xF007` data
offset, `0xF008` data length, `0xF00A` black levels, `0xF00E` as-shot green,
red and blue levels, `0xF00C` two white-balance presets as (green, red, blue,
EXIF light) quadruples (light 17, Standard A, and 21, D65).

## Sensor layouts

`0x0130`:

| Value | Sensor | Observed |
|---|---|---|
| `0C 0C 0C 0C` | X-Trans, pattern in `0x0131` | X-Pro1 to X-T5, X20, XQ1 |
| `0A 0B 09 08` | Bayer RGGB from the plane's origin | X100, X-A1 to X-A10, X-T100, X-T200, XF10, GFX |
| `08 09 0B 0A` | EXR: refused (#1621) | XF1, X10, X-S1, HS50EXR |
| other | SuperCCD: refused (#1621) | S5 Pro (`01 02 01 00`) |

An EXR file may carry the Bayer value (F550EXR), so a non-zero second frame
is refused first. A sensor section that is not a TIFF (S7000) is refused as
SuperCCD.

**X-Trans phase.** `0x0131` lists the pattern from its last photosite back to
its first: reversed, it is the CFA at the plane's origin. Checked against the
DNG Adobe's converter wrote from the same X100S file (its CFAPattern2 at
active-area column 0, which is plane column 4). **Bayer phase.** Correlating
each 2 × 2 position of the crop with the camera JPEG's channels puts red at the
plane's origin on the X100, X-A1, X-A5 and GFX 50S.

## Sensor data

For `n = width × height` photosites and `s` stored bytes:

| `s` | Data | Observed |
|---|---|---|
| `2n`, row length 0 | 16-bit words in the section's byte order | X100S, X-E2, X-T1 to X-T3, X-H1, GFX 50S |
| row length × height, 12 bits | 12-bit samples, LSB first | X100, X-A1, X-E1, X-Pro1 |
| row length × height, 14 bits | 14-bit samples, MSB first over little-endian 32-bit words | X-A5 |
| below the packed size | Fujifilm compressed: refused (#1620) | X-H1 compressed, X-T5 |

The packed bit orders were found by unpacking candidate orders and taking the
one whose same-colour neighbours differ least (by 10× or more).

## Levels and colour

The whole plane is the active area; the directory crop is the default crop.
The black is one value, 2 × 2 or 6 × 6 (`0xF00A`), from the plane's origin;
the white is the precision's ceiling (the X100S DNG agrees: 16383 at 14
bits). The baseline exposure is minus `0x9650`, the raw exposure bias
(signed 16-bit numerator and denominator): the dynamic-range modes
underexpose the raw (-1.72 EV on the DR200 X100 sample) and the camera lifts
its JPEG back, so the develop does too. Adobe's DNG of the X100S sample has
BaselineExposure 0.15 for a bias of -0.72, so Adobe adds a per-camera offset
of its own; that offset is not used. The as-shot neutral is `(G/R, 1, G/B)` of `0xF00E` (equal to the
X100S DNG's `AsShotNeutral`).

RAF embeds no matrix. `fit-camera-jpeg-matrices.py` gains the RAF container:
the camera JPEG is the header's JPEG section, make and model its Exif, and
the presets `0xF00C`'s Standard A (code 17) and D65 (code 21) levels. The
fitted rows go into `camera_jpeg_matrices.rs` under make `FUJIFILM`.

## X-Trans demosaic

Pictura's own edge-directed method (`xtrans.rs` documents it):

1. Green at red and blue photosites: linear interpolation between the nearest
   greens along the row and along the column (within 3), blended by
   `1 / (1 + activity)²`, activity being the mean difference of same-colour
   photosites one or two apart along that direction over three lines.
2. Red and blue: green plus a weighted mean of colour differences in the
   5 × 5 window, the weights falling with distance and with green
   difference.

It reproduces flat fields and linear ramps exactly, so a grey edge produces
no false colour. `DemosaicMethod::Ahd` routes 6 × 6 X-Trans patterns to it
rather than adding a method, so the app's existing callers (which always ask
for `Ahd`) get it without change. Rows are processed in bands of 32 with a
green plane two rows deeper on each side, as AHD does, so no full-size
intermediate is allocated.
