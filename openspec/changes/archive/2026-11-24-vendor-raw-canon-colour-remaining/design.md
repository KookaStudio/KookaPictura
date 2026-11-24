# Design: vendor-raw-canon-colour-remaining

The fit method, acceptance limits (60 blocks, a*b* spread 6, median 4 ΔE),
preset placement and the generator's choice among a body's samples (the first
with both matrices, else the first) are #1574's and #1579's, unchanged.

## Samples and fits

The owner supplied eight files from the dpreview sample galleries in
`~/Downloads/PHOTOSHOP/RAW/Canon`. The run reads copies (CR3s and CR2s in
separate directories); earlier entries of `camera-jpeg-fits.json` are kept.

| Body | Sample | Version | Blocks | Median / p90 ΔE | Spread | Verdict |
|---|---|---|---|---|---|---|
| EOS R8 Mark II | canon_eos_R8_mk2.CR3 | 67 | 279 | 4.71 / 9.23 | 19.5 | rejected (#1579) |
| EOS R8 Mark II | canon_eos_r8_mk2_pic2.CR3 | 67 | 471 | 3.00 / 5.66 | 12.5 | fitted, pair; the row |
| EOS R8 Mark II | canon_eos_r8_mk2_pic3.CR3 | 67 | 471 | 0.76 / 2.30 | 11.9 | fitted, pair; not taken |
| EOS M6 Mark II | canon_eos_M6_mk2.cr3 | 19 | 487 | 2.17 / 5.41 | 21.7 | fitted, pair; the row |
| EOS M6 Mark II | canon_eos_M6_mk2_pic2.cr3 | 19 | 520 | 1.02 / 2.69 | 15.7 | fitted, pair; not taken |
| PowerShot G7 X Mark III | canon_g7_x_mk3_pic1.cr3 | 19 | 365 | 4.07 / 7.62 | 35.0 | rejected (median over 4) |
| PowerShot G7 X Mark III | canon_g7_x_mk3_pic2.cr3 | 19 | 318 | 2.61 / 5.05 | 20.2 | fitted, pair; the row |
| PowerShot G1 X Mark III | canon_g1_x_mk3.cr2 | -4 | 490 | 1.48 / 2.66 | 4.6 | rejected (too grey) |
| PowerShot G1 X Mark III | canon_g1_x_mk3_pic2.cr2 | -4 | 499 | 2.54 / 7.50 | 17.3 | fitted, D55 alone; the row |

The generator's rule picks the first sample with a pair in recorded order, so
the R8 Mark II takes `pic2`, not the lower-error `pic3`. That is deliberate:
`pic3`'s scene is the less colourful one (spread 11.9), and its matrix has
large off-diagonal terms (camera blue = 1.80 X − 1.51 Y + 0.51 Z under D55)
that a narrow scene constrains poorly; `pic2`'s matrix is of the usual shape.
The M6 Mark II likewise takes its first sample. The R8 proxy the owner allowed
as a fallback is not needed.

With their own fits, the M6 Mark II and G7 X Mark III leave the 90D's and
G5 X Mark II's rows. No proxy is left, so the generator's `JPEG_PROXIES` list
and the code that wrote it are removed.

## CR2 in the fit script

A CR2 is a little-endian TIFF: IFD0's strip (`StripOffsets`,
`StripByteCounts`) is the full-size camera JPEG, and the Exif IFD's maker note
(`0x927C`) is an IFD whose offsets count from the file's start, holding the
colour data (`0x4001`). Make and model come from IFD0. The block extractor
(`raw_jpeg_blocks`) already reads any container `read_raw` does.

## Colour-data version -4

The G1 X Mark III writes version -4 (65532 unsigned), ExifTool's "M100/M5/M6"
layout. Its preset records are 8 words long, not 5: four RGGB levels, three
other words, then a temperature. The as-shot record is at word 71 (where the
codec already reads it for negative versions), then Auto, Measured and a copy
of Daylight; Daylight is at 103, followed by Shade (111), Cloudy (119),
Tungsten (127) and Fluorescent (135). On both samples the values agree with
ExifTool's reading (Daylight 1434 1024 1024 1305 at 5405 K, Cloudy 6283 K,
Tungsten 974 1024 1024 1899 at 3037 K).

These temperatures are the camera's estimate of each preset's white, not the
nominal 5200 / 6000 / 3200 K the version-64 layout records. The script keeps
its rule unchanged: D55 is interpolated in mired between the recorded
Daylight and Cloudy temperatures, and Tungsten is used only when recorded at
3200 K. So the G1 X Mark III gets a D55 matrix alone, which the colour stage
uses at every temperature, as for other single-matrix bodies. A Tungsten
matrix would need a temperature the file does not record; inventing one
would be a guess.

## Colour data stored as bytes

On the G1 X Mark III the maker note's `0x4001` entry is type 7 (UNDEFINED),
5120 bytes, not SHORT. The codec read it with `reals`, which yields one value
per byte, so the version and offsets were wrong and the as-shot neutral was
empty. A new `Tiff::words` reads SHORT values, or BYTE and UNDEFINED bytes in
pairs in the file's byte order; the CR2 (and CR3) as-shot reader uses it. A
colour-data entry of any other type now leaves the neutral empty instead of
being read as numbers, and never fails the read.

## White level (out of scope)

The CR2 reader normalizes to the frame's precision ceiling (a `ponytail:` in
`cr2.rs`). The G1 X Mark III saturates at 11892 of 16383 (black 512), so its
clipped highlights end near 0.72, not 1.0. This does not affect the fit:
refitting with only blocks below 0.2 of the ceiling changes the median by
0.06 ΔE and the matrix little. It does affect how clipped highlights render,
and is left to #1587 (reading Canon white levels).
