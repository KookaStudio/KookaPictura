# Design: vendor-raw-rw2-colour

## Sources

| Source | Panasonic bodies | Usable |
|---|---|---|
| rawtoaces-data `e9b8503c` (Apache-2.0) | DC-GX9 | yes: spectral fit, A and D65 |
| Physlight, Winquist et al. 2022 (Apache-2.0) | none | — |
| ilia3101/SpectralMeasurements | GH1, GH2, GF3 | no: the repository has no licence |
| Solomatov & Akkaynak 2023 | many (estimated) | no: fitted to Adobe DNG Converter output |
| Jiang et al. 2013 (camspec) | none | — |

Every other body takes a camera-JPEG fit (owner decision).

## Camera-JPEG fit

`crates/pictura-codec/examples/raw_jpeg_blocks.rs` reads the raw file with
`read_raw` (our own decoder) and the embedded JPEG with the `image` crate
already in the codec, and prints both averaged over a 32 × 24 grid of the
default crop: camera RGB (0–1 over black–white), linear sRGB, and each
block's clipped share. `scripts/fit-camera-jpeg-matrices.py` (no dependency
beyond Python) then:

1. Keeps blocks inside an 8 % margin with no photosite at white, camera RGB
   in 0.002–0.9 and JPEG RGB in 0.004–0.8 (linear).
2. Divides camera RGB by the as-shot neutral and normalizes each block's
   camera and JPEG vectors to unit sum, so exposure and the JPEG's tone curve
   drop out and only chromaticity is fitted.
3. Solves each matrix row by least squares under the constraint that camera
   white maps to sRGB white, then refits four times on the best 85 % of
   blocks; texture the JPEG's lens correction moves is what gets trimmed.
4. Places the matrix under the file's own presets:
   `XYZ = Bradford(D65 → preset light) · sRGB→XYZ · M · diag(1 / preset neutral) · camera`,
   inverted and scaled like the spectral rows (largest camera response to the
   light's white is 1). Daylight presets use the CIE daylight locus, tungsten
   presets the Planckian locus.

The presets are read from WBInfo2 (`0x0027`: count, then code, R, G, B) or,
in older files, WBInfo (`0x0013`: count, then code, R, B against a green of
256), as ExifTool documents them.

Tried and dropped: a 1-D tone curve estimated and undone per channel (the
iteration diverged on textured scenes), a smoothness filter (it removed most
foliage, leaving greens to extrapolation; ZS100 foliage turned cyan), and a
64 × 48 grid (more sensitive to the JPEG's lens correction).

## Validation

- The DC-GX9 has both a spectral and a camera-JPEG fit. On its sample's
  colours the two agree to median 1.8 ΔE (90th percentile 8.4), and both are
  about equally far from the camera JPEG (median 4.6 and 5.0 ΔE). The JPEG fit
  is somewhat less saturated than the spectral one, and colours outside the
  sample scene are extrapolated. The table keeps the spectral fit for the
  GX9.
- Each accepted body was rendered through `decode_raw_source` and
  `develop_raw` at its as-shot white balance and compared beside its camera
  JPEG (manual row `rw2_render_matches_camera`).

## Results

| Body | Sample | Blocks | Median / p90 ΔE | Verdict |
|---|---|---|---|---|
| DC-GH5S | P1022081.RW2 | 502 | 1.88 / 3.77 | fitted |
| DC-GH6 | 1-1.RW2 | 518 | 2.49 / 6.96 | fitted |
| DC-S1 | P1033119.RW2 | 495 | 1.88 / 4.07 | fitted |
| DC-S5 | dc-s5_6k4k.RW2 | 520 | 0.34 / 2.17 | fitted (P1047510 too grey) |
| DMC-FZ18 | RAW_PANASONIC_LUMIX_FZ18.RAW | 483 | 2.68 / 5.11 | fitted |
| DMC-FZ28 | P1120635.RW2 | 442 | 2.13 / 5.04 | fitted |
| DMC-G1 | RAW_PANASONIC_G1.RW2 | 520 | 0.75 / 1.99 | fitted |
| DMC-GM1S | panasonic_DMC-GM1S.RW2 | 196 | 1.98 / 3.87 | fitted |
| DMC-ZS100 | panasonic_DMC-ZS100.RW2 | 453 | 3.79 / 7.79 | fitted |
| DC-GX9 | gx9_1_1.RW2 | 483 | 3.58 / 11.77 | spectral fit kept |
| DC-G9, DC-G9M2 | two samples each | — | — | too little colour |
| DMC-FZ8, FZ30, FZ50, L1, LX1, LX2 | one each | — | — | no embedded JPEG |

The rejected bodies and the ~110 not attempted (the sample servers ran at
7–20 KB/s) are #1558.

## Highlights

The FZ18 sample piles up at 3967 in all four CFA phases (thousands of
photosites each) under a white tag of 4095, and the FZ8 sample tops out at
3967 too; both are RawFormat 2, the only bodies using it. With white 4095,
clipped areas stop at 0.97 in green while red and blue clip at 1 after white
balance, a pink cast. RawFormat 2's white is now at most 3967. The other
plain codings show no such ceiling: the LX2's 16-bit words reach 4093.

## Table

`CameraMatrices` now holds `matrices: &[(LightSource, matrix)]` (one or two)
instead of fixed A and D65 fields; `vendor::camera_color` copies them in
order. The 39 existing rows regenerate to the same numbers.

## Open dialog

Routing was already by signature (`raw_sniff_path` runs before the extension
route), so listing `*.RAW` cannot send a Photoshop Raw file to the raw open;
only the filter strings change.
