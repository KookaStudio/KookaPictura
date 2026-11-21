# Design: vendor-camera-matrices

## Fitting (`scripts/gen-camera-matrices.py`)

Inputs, 380–780 nm at 5 nm: camera RGB sensitivities `S`, the CIE 1931
colour-matching functions, the 190 training reflectances `R_i`, and an
illuminant `L` (A from Planck's law at 2848 K with c2 = 1.435e-2 m·K; D65
from the CIE table).

For each illuminant: `cam_i = Σ S·L·R_i`, `XYZ_i = Σ cmf·L·R_i / Σ ȳ·L`
(so the perfect white has Y = 1). Each row of the camera → XYZ matrix `B` is
the least-squares fit of `B·cam_i ≈ XYZ_i` under the equality constraint that
the white's camera response maps exactly to the white's XYZ (closed form via
a Lagrange multiplier), so a neutral stays neutral. The DNG-style
`ColorMatrix` is `B⁻¹`, scaled so the white's largest camera component is 1.
The script prints each fit's mean and worst ΔE76 on the training set (mean
about 1.7–2.0 under D65 and 2.5–3.0 under A for the EOS bodies). It needs only
the Python standard library.

The output is `crates/pictura-codec/src/dng/camera_matrices.rs`: a `const`
slice of `{ make, models, illuminant_a, d65 }`, generated, not hand-edited.

The seed is the dataset's Canon bodies that write CR2: Digital Rebel XTi,
1D X Mark II, 1Ds Mark II, 1Ds Mark III, 100D, 200D, 5D, 5DS, 5D Mark II,
5D Mark III, 5D Mark IV, M, PowerShot S90, and the EOS RP (whose
dataset alias, the 6D Mark II, writes CR2). The 600D is left out: the
CR2 reader's "RGGB at the sensor corner" rule misreads its CFA phase (its
diagonal means show green at positions 0 and 3), so it would open with wrong
colour; it stays refused until #1476 fixes that. rawtoaces' `aliases.json` adds
the other market names of the same body.

## Matching

`model_key(make, model)`: upper-case, `-` as a space, drop a leading word
equal to the make's first word, drop the words `EOS` and `DIGITAL`, join
single-spaced. `Canon EOS 400D DIGITAL`, `EOS 400D`, `Canon EOS DIGITAL
REBEL XTi` and `Digital Rebel XTi` give `400D` / `REBEL XTI`; the make key
is the make's first word, upper-cased (`NIKON CORPORATION` → `NIKON`). The
Python and Rust copies must agree; the Rust tests pin the real Exif names.

`vendor::camera_color(make, model, as_shot_neutral)` returns the `DngColor`:
both matrices with `CalibrationIlluminant` 17 (A) and 21 (D65) when the
camera is known, otherwise no matrices; `AsShotNeutral` either way. The
existing #398 interpolation, Kelvin search and refusal apply unchanged.

## CR2 as-shot white balance

Canon maker-note tag `0x4001` (SHORT array). Entry 0 is a signed version
except in the two oldest layouts, which are named by length. The as-shot
RGGB multipliers start at:

| Layout | Selected by | Index |
|---|---|---|
| 20D, 350D | 582 entries | 25 |
| 1D/1Ds Mark II | 653 entries | 34 |
| versions 1–15 | entry 0 | 63 |
| versions 16–19, negative (EOS M / PowerShot) | entry 0 | 71 |
| versions 32–33 | entry 0 | 85 |
| versions 34, 48, 64–67 | entry 0 | 105 |

`AsShotNeutral = (G/R, 1, G/B)` with `G` the mean of the two greens. Any
other layout, a short array, or a non-positive multiplier leaves it empty.
Observed on real files (array length / version / multipliers agree with
`exiftool -WB_RGGBLevelsAsShot`): 350D 582, 1Ds Mark II 653, 400D/5D v1,
1Ds Mark III v4, 450D v5, 5D Mark II v6, 600D/100D/5D Mark III/M v10, 5DS
v12, 1D X Mark II/5D Mark IV v13, 200D/6D Mark II v15.

## Not changed

The CR2 white level stays the frame's precision ceiling (the ponytail in
`read_cr2`); the camera colour stage, develop and the raw open route are
untouched. Cameras outside the dataset stay refused (owner decision): no
generic fallback matrix.
