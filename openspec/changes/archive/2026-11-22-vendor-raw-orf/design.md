# Design: vendor-raw-orf

## Modules

Inside `crates/pictura-codec/src/dng/`:

- `orf.rs`: signature, sensor-data layout, the 12-bit packings, the reader.
- `orf_meta.rs`: the maker note and the colour it yields.
- `tiff.rs` gains `Tiff::view`, a reader over bytes without a TIFF header
  (the ORF magic, and maker notes whose offsets count from the note).

## Sensor data

IFD0 is the sensor image. Its `Compression` is 1 and its `BitsPerSample` 16
even for compressed data, so the layout follows from the strips' total size
`s` for `n = width x height` photosites:

| Size | Data | Observed |
|---|---|---|
| `s = 2n` | 16-bit words in file order, left-aligned | E-1, E-10, E-20, E-400 |
| `s = 1.6n` | 16-byte groups: ten samples LSB-first in 15 bytes, a pad byte | E-300, E-330, E-500, E-M5 II and PEN-F high-res |
| `s = 1.5n`, `IIRO` | MSB-first 12-bit in little-endian 32-bit words | SH-2, XZ-2, XZ-10 |
| `s = 1.5n`, `IIRS` | MSB-first 12-bit byte stream in two fields (even rows, then odd), each ending in its own short strip | C-5050Z to SP-570UZ |
| one strip, `s < 1.5n` | Olympus compressed: refused (#1535) | E-410 to OM-5 II |

Anything else is refused as unsupported.

## Maker note

| Header | Directory at | Offsets count from |
|---|---|---|
| `OLYMP\0` + 2 bytes | 8 | file start |
| `OLYMPUS\0II` + 2 bytes | 12 | note start |
| `OM SYSTEM\0\0\0II` + 2 bytes | 16 | note start |

ImageProcessing (`0x2040`, an IFD pointer or the directory as the value) is
preferred, then RawInfo (`0x3000`), then the main note's older tags.

| Value | ImageProcessing | RawInfo | Main |
|---|---|---|---|
| as-shot R, B / 256 | `0x0100` | `0x0100` | `0x1017`, `0x1018` |
| daylight R, B | `0x010C` (6600 K) | `0x0122` (fine weather) | — |
| matrix / 256 | `0x0200` | `0x0200` | `0x1011` |
| black | `0x0600` | `0x0600` | `0x1012` |
| valid bits | `0x0611` | `0x0611` | `0x102C` |
| crop | `0x0612`-`0x0615` | `0x0612`-`0x0615` | — |
| white | `0x0805`, first value | — | — |

16-bit words scale black and white by `2^(16 - bits)`. Without `0x0805` the
white is the precision's ceiling. The CFA is Exif `0xA302`, measured from the
plane's origin. The whole plane is the active area; the crop is the default
crop.

## Colour

`M` (rows normalized to sum 1) takes white-balanced camera RGB to linear sRGB.
With `S` = sRGB → XYZ and `d` the daylight neutral whose illuminant has white
`w`:

- forward matrix = `Bradford(D65 → D50) · S · M`, so at any white balance a
  neutral stays neutral and colours follow the camera's matrix;
- colour matrix = `diag(d) · M⁻¹ · S⁻¹ · Bradford(w → D65)`, which gives the
  temperature and tint of a neutral.

`d` is the 6600 K balance as D65, else fine weather as D55, else the as-shot
balance as D65 (its Kelvin readout is then nominal). Without any balance, or
with a singular matrix, the file has no matrices and is refused at open.

## Decisions

- **Owner (#1471)**: colour from the embedded matrix, labelled approximate.
  Uncompressed only: a clean-agent attempt at the compressed coding stopped
  for provenance, and a patent question is open, so that coding is #1535.
- **Known limit**: on the 22 uncompressed sample files the matrix is closer
  to the camera JPEG than white balance alone every time, but on the SP-350,
  SP-510UZ and SP-550UZ it still turns warm tones pink and skies teal next to
  that JPEG. The camera's own processing evidently does more than the matrix.
- **Ponytails**: one black per 2 x 2 position in stored order; the green
  levels of the white balance are taken as 256; a single matrix.
