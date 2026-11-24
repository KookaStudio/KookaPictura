# Design: vendor-raw-cr3-colour

## Sources

rawtoaces-data `e9b8503c` publishes Canon sensitivities for no CR3 body beyond
the eight #1474 added. Every other corpus body takes a camera-JPEG fit (owner
decision), the #1542 method unchanged: `examples/raw_jpeg_blocks.rs` reads
the raw file with our own `read_raw` and the JPEG with `image`;
`scripts/fit-camera-jpeg-matrices.py` fits chromaticities over a 32 × 24
grid, trims the worst 15 %, and places the matrix under the file's presets.

## CR3 inputs

- **Camera rendering.** A CR3's first track holds the full-size JPEG (the
  body's full resolution); the script takes the largest first sample of any
  track that starts with a JPEG SOI. The R5 Mark II full-frame and R8 samples
  have no JPEG track (rejected as "no embedded JPEG"); both bodies are
  spectral rows anyway.
- **Make and model** from `CMT1`, the IFD0 TIFF in Canon's `moov/uuid` box.
- **Presets** from the colour data (`0x4001`) of the first `CTMD` maker note
  that has one. Each preset is four RGGB levels and the temperature Canon
  records with it. Daylight's word index per colour-data version was located
  on the samples by matching ExifTool 13.50's `WB_RGGBLevelsDaylight`, and
  each preset's recorded temperature confirms the order (Daylight 5200 K,
  Shade 7000 K, Cloudy 6000 K, Tungsten 3200 K on every sample):

  | Versions | Bodies (samples) | As shot | Daylight |
  |---|---|---|---|
  | 16–19 | M50, R, RP, 250D, 90D, 850D, M6 II, M200, M50 II, R100, PowerShots | 71 | 136 |
  | 32, 33 | 1D X III, R5, R6 | 85 | 150 |
  | 34, 48 | R3, R7, R10, R50, R6 II, R8 | 105 | 205 |
  | 64–66 | R5 II, R50 V, V1, R6 III | 105 | 110 |

  ExifTool 13.50 does not know version 66 (R6 Mark III) and misreads its
  presets; the recorded temperatures show version 64's layout.

## Placing the matrices

Canon's Daylight preset is 5200 K and EXIF has no `LightSource` code there
(code 1 means 5500 K to the colour stage). The daylight matrix is therefore
placed under D55 (code 20, 5503 K) with the neutral interpolated in mired
between the Daylight (5200 K) and Cloudy (6000 K) presets, linear in each
component. Tungsten is recorded at 3200 K, exactly ISO studio tungsten
(code 24), and placed there directly. Both codes are already in the colour
stage's table and the fit script's.

## A pair must read back its lights

The first table failed `canon_jpeg_matrices_keep_white_neutral` on the R6
Mark III: the D55 matrix's own white read back at 4002 K. Its daylight matrix
reads the recorded Tungsten preset as 4146 K (every other Canon fit: 2661–
3727 K), so the pair disagrees with itself and the colour stage's fixed-point
solve has a slope of 1.02 at D55 (it settles only below 1; the others are
-0.2 to 0.73). The fit, from a sample shot under 7400 K light with 277 usable
blocks, is the likely culprit, not Canon's presets, which match the R5 Mark
II's ratios.

The script now checks every pair the way the colour stage reads it (mired
interpolation, iteration from 5003 K, McCamy's CCT approximation) and drops
the tungsten matrix when either light does not read back within 2 %, the
same tolerance the white tests hold. Only the R6 Mark III changes; it keeps
its D55 matrix alone. Its sample then reads 6192 K against Canon's own 7416 K
(the other camera-JPEG bodies read within about 200 K of Canon's
`ColorTempAsShot`: 90D 6625 / 6868, M50 3191 / 3200, M200 5728 / 5800,
R100 4785 / 4799, R7 4856 / 4803, 1D X III 4045 / 4171). All shipped
Panasonic pairs already pass the check.

## Proxies

Four bodies were rejected for a grey scene (a*b* spread under 6). Two share a
sensor with a fitted body and take its matrices, listed in
`gen-camera-matrices.py`'s `JPEG_PROXIES`:

- EOS M6 Mark II ← EOS 90D: the same 32.5 MP APS-C sensor; Canon announced
  both together.
- PowerShot G7 X Mark III ← PowerShot G5 X Mark II: the same 20.1 MP stacked
  1-inch sensor; announced together.

The R3 (its own 24 MP stacked sensor) and the PowerShot V1 (its own 1.4-inch
sensor) share no sensor with a fitted body and move to the follow-up issue #1579.
A body with its own accepted fit keeps it even when a sibling is fitted (the
R7 and 90D each keep their own).

## Validation

Where a CR3 body has both a spectral row and a usable sample, both were
compared against the camera JPEG on the sample's blocks (the spectral D65
matrix adapted from the as-shot white to D65 by Bradford; median / 90th
percentile chromaticity ΔE):

| Body | JPEG fit | Spectral | Spectral vs fit |
|---|---|---|---|
| EOS R | 0.6 / 1.6 | 1.2 / 5.2 | 1.4 / 3.4 |
| EOS R5 | 1.8 / 4.3 | 2.2 / 10.5 | 1.1 / 9.6 |
| EOS R6 | 1.1 / 2.7 | 3.2 / 7.1 | 2.3 / 9.1 |
| EOS R10 | 0.4 / 1.1 | 2.2 / 2.9 | 2.1 / 2.6 |
| Rebel SL3 (200D II) | 1.0 / 2.0 | 1.2 / 5.1 | 0.5 / 0.8 |
| EOS RP | 0.9 / 2.2 | 6.3 / 14.7 | 6.5 / 13.3 |
| EOS R50 (R10 alias) | 1.5 / 3.6 | 11.3 / 28.0 | 11.5 / 25.6 |

For most bodies the two methods agree to 0.5–2.3 ΔE median, which supports
the camera-JPEG fits for bodies without spectral data. Two existing rows sit
far from their camera's rendering: the RP's spectral fit and the R50, which
rawtoaces-data aliases to the R10 as the same sensor. They are recorded for
#1579, not changed here.

## Results

| Body | Sample | Blocks | Median / p90 ΔE | Verdict |
|---|---|---|---|---|
| EOS 850D | EOS_850D__IMG_1837.CR3 | 520 | 1.53 / 2.71 | fitted |
| EOS 90D | EOS_90D__Canon-90D-RAW-ISO-100.CR3 | 500 | 1.13 / 2.34 | fitted |
| EOS M200 | Canon_EOS_M200___MG_2231.CR3 | 509 | 1.37 / 2.20 | fitted |
| EOS M50 | EOS_M50__…CRAW_ISO_100_nocrop_nodual.CR3 | 323 | 2.23 / 3.74 | fitted (Kiss M sample 6.28 ΔE) |
| EOS M50 Mark II | EOS_M50_Mark_II__IMG_0051.CR3 | 257 | 3.19 / 6.56 | fitted |
| EOS R100 | Canon_EOS_R100___MG_2195[1].CR3 | 383 | 3.81 / 10.98 | fitted |
| EOS R50 V | Canon_EOS_R50_V__IMG_0034.CR3 | 455 | 1.74 / 4.63 | fitted |
| EOS R6 Mark III | Canon_EOS_R6_Mark_III__IMG_4241.CR3 | 277 | 2.45 / 4.15 | fitted, D55 alone |
| EOS R7 | EOS_R7__443A0157.CR3 | 516 | 2.65 / 5.50 | fitted |
| EOS-1D X Mark III | EOS-1D_X_Mark_III__…CRAW…CR3 | 499 | 1.33 / 2.55 | fitted |
| PowerShot G5 X Mark II | PowerShot_G5_X_Mark_II__…CRAW_(3:2).CR3 | 241 | 1.39 / 2.04 | fitted |
| PowerShot SX70 HS | PowerShot_SX70_HS__IMG_1000.CR3 | 374 | 1.28 / 3.78 | fitted |
| EOS M6 Mark II | two samples | 520 | — | too little colour; 90D proxy |
| PowerShot G7 X Mark III | IMG_0253.CR3 | 434 | — | too little colour; G5 X II proxy |
| EOS R3 | _C3A1200.CR3 | 492 | — | too little colour; #1579 |
| PowerShot V1 | IMG_1225.CR3 | 514 | — | too little colour; #1579 |

The samples are the raw.pixls.us files of `~/Downloads/cr3-1474` (#1474);
`camera-jpeg-fits.json` records every one with its SHA-256, the spectral
bodies' too.

## Keeping other fits

The Panasonic samples are no longer at hand. The script now keeps the
recorded entries of samples outside the run and replaces only those it
refits, so the Canon run leaves the Panasonic entries byte-identical, and
`gen-camera-matrices.py` regenerates the existing rows unchanged.
