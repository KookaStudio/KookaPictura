# Design: camera-colour

## Modules

- `crates/pictura-color/src/chroma.rs` — `Mat3` with product, inverse and
  matrix-vector helpers; `rgb_to_xyz(primaries, white)`; `bradford(from, to)`;
  `xy_to_temperature_tint` and `temperature_tint_to_xy`; the D50 and ProPhoto
  constants.
- `crates/pictura-color/src/lib.rs` — `Profile::linear_pro_photo()`: ProPhoto
  primaries and white with a gamma-1.0 curve.
- `crates/pictura-codec/src/dng/camera_color.rs` — `WhiteBalance`,
  `CameraTransform`, `ProPhotoRgb`, `as_shot_transform`,
  `white_balance_transform`, `to_pro_photo`.

## Colour temperature and tint

The Planckian locus in CIE 1960 (u, v) is Krystek's rational approximation
(1985), stated valid from 1000 K to 15000 K to within about 1e-4 in uv.
Temperatures are clamped to that range (`ponytail:` Camera Raw's slider goes to
50000 K).

- **xy → (T, tint).** Convert to uv. A golden-section search over reciprocal
  temperature (66.7–1000 mired) finds the locus point closest to it. `Duv` is
  the signed distance, positive on the green side (above the locus).
  `tint = −3000 · Duv`, so positive tint is magenta. The 3000 is chosen to give
  Camera Raw's ±150 slider about the same reach; the scale is approximate and
  not measured against Camera Raw.
- **(T, tint) → xy.** Take the locus point at T and step `Duv` along the unit
  normal there (the tangent comes from a central difference). Both directions
  use the same normal, so they round-trip.

## The camera transform

Notation follows DNG 1.6 chapter 6. `CM`, `CC` and `FM` are the interpolated
`ColorMatrix`, `CameraCalibration` (identity when absent) and `ForwardMatrix`.
`AB` is `diag(AnalogBalance)`, identity when absent.

**Calibration illuminants.** EXIF `LightSource` codes map to nominal
temperatures: A 2856, B 4874, C 6774, D50 5003, D55 5503, D65 6504, D75 7504,
tungsten 2850, ISO studio tungsten 3200, daylight / flash / fine weather 5500,
cloudy 6500, shade 7500, and fluorescents 2940–6430. When both matrices exist
and both illuminants are known and different, each matrix is interpolated at
temperature T. The weight is `g = (1/T − 1/T_high) / (1/T_low − 1/T_high)`,
clamped to 0–1. Otherwise the first present matrix is used alone
(`ponytail:` DNG 1.6's illuminant 255 with `IlluminantData` is not read).
`CameraCalibration` is applied without checking
`CameraCalibrationSignature` against `ProfileCalibrationSignature`
(`ponytail:` the reader does not read the signatures).

**Finding the as-shot white.** The camera neutral `n` is `AsShotNeutral`.
Starting from 5003 K: `XYZtoCamera = AB · CC · CM(T)`, then
`xyz = XYZtoCamera⁻¹ · n`, which gives xy and from it a new T. This repeats until
T moves less than 0.01 mired, or 30 times. The reported white balance is the
final (T, tint). Without `AsShotNeutral`, the white is D50
(`ponytail:` `AsShotWhiteXY` is not read).

**From a temperature and tint.** xy from (T, tint) gives `XYZ = (x/y, 1, z/y)`.
Then `n = AB · CC · CM(T) · XYZ`, normalized so its largest component is 1. A
non-positive component is `Invalid`.

**Matrix.** `n` is normalized to a largest component of 1. With a
`ForwardMatrix`: `r = (AB·CC)⁻¹ · n`, and
`CameraToXYZ_D50 = FM · diag(r)⁻¹ · (AB·CC)⁻¹`. Without one:
`CameraToXYZ_D50 = Bradford(xy(n) → D50) · XYZtoCamera⁻¹`. Then
`X = CameraToXYZ_D50 · diag(n)`, scaled by `1 / (X · [1,1,1])_Y`, and
`matrix = XYZ_D50→linear ProPhoto · X`. By construction
`matrix · [1,1,1] = [1,1,1]`: the camera neutral at full scale is ProPhoto
white. A missing `ColorMatrix` or any singular inverse is `Unsupported`.

**Pixels.** The white-balance gains are `1/n` (smallest gain 1). Per pixel,
`w = min(c / 65535 · gain, 1)` per channel, so clipped highlights stay neutral
(`ponytail:` no highlight recovery). Then `out = clamp(matrix · w, 0, 1)` ×
65535, rounded (`ponytail:` out-of-ProPhoto colours are clipped per channel).
The math is `f32` per pixel and runs on rows in parallel, in place over the
`CameraRgb` buffer, so it needs no further budget. `BaselineExposure` is not
applied; #400's exposure stage owns it.

## Decisions

- **Output space** is linear ProPhoto at 16 bits. It is wide enough that the
  stage itself clips little; #400 converts to the document's profile through
  `pictura_color::convert` from `Profile::linear_pro_photo()`.
- **White balance** is applied in this stage, and the transform reports the
  as-shot temperature and tint. Pictura Raw's relative ±100 sliders keep their
  meaning until #400 rebases them on absolute kelvin.
- **No matrix, no guess.** A DNG without `ColorMatrix1/2` is refused rather
  than shown in camera-native colour.
