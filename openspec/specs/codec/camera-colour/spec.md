# codec/camera-colour Specification

## Purpose
The camera colour stage: demosaiced camera RGB white-balanced and mapped to linear ProPhoto through a DNG's own colour matrices, with the as-shot white balance reported as temperature and tint, as the third stage of opening camera raw files (#86).

## Requirements

### Requirement: Camera RGB maps to linear ProPhoto through the DNG's own matrices

`as_shot_transform` SHALL build the camera-to-working-space transform from a
DNG's `ColorMatrix1/2`, `ForwardMatrix1/2`, `CameraCalibration1/2`,
`AnalogBalance` and `AsShotNeutral`, following the DNG specification's
mapping. It SHALL use the `ForwardMatrix` when present, and otherwise a Bradford
adaptation from the camera white to D50. `to_pro_photo` SHALL apply the white
balance and matrix to a `CameraRgb` and return linear ProPhoto (ROMM primaries,
D50 white) at 16 bits per channel, the same size. No Adobe camera profile is
used. Parity with Adobe Camera Raw is approximate.

#### Scenario: A ProPhoto camera passes through [camera_colour_identity]

- **WHEN** a DNG's only `ColorMatrix1` is the XYZ-D50-to-linear-ProPhoto matrix with illuminant D50 and `AsShotNeutral` (1, 1, 1), and a random camera image is converted
- **THEN** the transform's matrix is the identity within 1e-6 and every output sample equals its input within 1

#### Scenario: An sRGB camera lands on ProPhoto [camera_colour_srgb_camera]

- **WHEN** a DNG's `ColorMatrix1` is the XYZ-to-linear-sRGB matrix with illuminant D65 and `AsShotNeutral` (1, 1, 1), and linear sRGB colours are converted
- **THEN** each output equals the published Bradford-adapted sRGB-to-XYZ-D50 matrix followed by XYZ-D50-to-ProPhoto, within 0.002 of full scale

#### Scenario: The camera neutral becomes grey [camera_colour_neutral]

- **WHEN** a DNG with two arbitrary matrices for illuminants A and D65 and `AsShotNeutral` (0.45, 1, 0.7) converts pixels equal to that neutral scaled by 0.1, 0.5 and 1, and a pixel at sensor saturation
- **THEN** each scaled neutral comes out with three equal channels at its scale within 2 counts, and the saturated pixel is ProPhoto white

#### Scenario: Calibration and analog balance are applied [camera_colour_calibration]

- **WHEN** the ProPhoto camera DNG also carries `CameraCalibration1` diag(1.1, 1, 0.9) and `AnalogBalance` (0.95, 1, 1.05), with `AsShotNeutral` the camera's response to D50 through all three
- **THEN** the transform's matrix is the identity within 1e-6

#### Scenario: ForwardMatrix is preferred [camera_colour_forward_matrix]

- **WHEN** a DNG carries a `ColorMatrix1` and a `ForwardMatrix1` that disagree, with no calibration or analog balance
- **THEN** the transform's matrix equals XYZ-D50-to-ProPhoto times the `ForwardMatrix`, scaled so the neutral maps to white, within 1e-6

### Requirement: The as-shot white balance is reported as temperature and tint

The transform SHALL report the white balance it applies as a colour temperature
in kelvin and a tint, found from `AsShotNeutral` by iterating the illuminant
interpolation. A DNG without `AsShotNeutral` SHALL use D50. The two colour
matrices SHALL be interpolated linearly in inverse temperature between their
calibration illuminants. `white_balance_transform` SHALL build the transform
for a given temperature and tint.

#### Scenario: Interpolated illuminants are found [camera_colour_interpolation]

- **WHEN** a DNG with different matrices for A and D65 has as its `AsShotNeutral` the D65 matrix's response to the D65 white, and then the A matrix's response to the A white
- **THEN** the reported white balance is 6504 K within 15 K, and then 2856 K within 10 K; and at 4000 K the transform equals that of a single-matrix DNG whose matrix is the inverse-temperature blend at 4000 K, within 1e-9

#### Scenario: White balance round trip [camera_colour_white_balance_round_trip]

- **WHEN** `white_balance_transform` is called with the as-shot transform's reported temperature and tint
- **THEN** its gains and matrix equal the as-shot ones within 1e-4 relative

#### Scenario: Default white [camera_colour_default_white]

- **WHEN** a DNG has a `ColorMatrix1` but no `AsShotNeutral`
- **THEN** the reported temperature is 5003 K within 15 K

### Requirement: DNGs without usable matrices are refused

A DNG with no `ColorMatrix1` or `ColorMatrix2`, or with a matrix that cannot
be inverted, SHALL fail with `PsdError::Unsupported`. An `AsShotNeutral`
with a component that is not positive SHALL fail with `PsdError::Invalid`.

#### Scenario: Refused colour tags [camera_colour_refused]

- **WHEN** transforms are built for a DNG with no colour matrix, one with an all-zero `ColorMatrix1`, and one with `AsShotNeutral` (0, 1, 1)
- **THEN** the first two fail with `PsdError::Unsupported` and the third with `PsdError::Invalid`

### Requirement: Camera-authored DNGs convert

A camera-authored DNG SHALL convert to linear ProPhoto when the user supplies
it; the check is opt-in because such files cannot be committed.

#### Scenario: Leica M9 samples [camera_colour_m9_corpus]

- **WHEN** `PICTURA_DNG_CORPUS` names a directory holding the Leica M9 `L1.dng` and `L2.dng`
- **THEN** each yields an as-shot temperature between 2000 K and 15000 K and converts to 5216 × 3472 linear ProPhoto that is not all black
