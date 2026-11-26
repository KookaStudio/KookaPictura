# Test plan

This plan covers `camera-colour` (#398): the new capabilities
`codec/camera-colour` and `color/colour-temperature`.

**Fixtures.** Colour tags are built in the test as `DngColor` values; camera
images as `CameraRgb` from fixed values or a fixed-seed LCG. The matrices used as
oracles (the Bradford-adapted sRGB-to-XYZ-D50 matrix and the CIE illuminant
chromaticities) are published constants written into the tests, not computed
by the code under test. The real-file row uses the user's Leica M9 samples,
opt-in through `PICTURA_DNG_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cct_illuminants` | Convert A (0.44757, 0.40745) and D65 (0.31271, 0.32902) to temperature and tint | A 2856 ± 10 K, tint within ±1; D65 6504 ± 15 K, tint −11…−8 | `pictura-color::standard_illuminants` |
| `cct_round_trip` | 2000–15000 K in 500 K steps × tints −50, 0, 50 → xy → back | Temperature within 0.1 %, tint within 0.05 | `pictura-color::temperature_tint_round_trip` |
| `chroma_matrices` | sRGB primaries at D65, Bradford D65→D50; ProPhoto matrix · (1,1,1) | Published matrix within 1e-4; D50 white | `pictura-color::srgb_and_pro_photo_matrices` |
| `linear_pro_photo_profile` | `convert` 16-bit linear grey 0.25 and white, linear ProPhoto → ProPhoto, relative colorimetric | Grey 0.25^(1/1.8) within 2 counts; white stays white | `pictura-color::linear_pro_photo_encodes_to_romm` |
| `camera_colour_identity` | ColorMatrix1 = XYZ D50 → ProPhoto, D50, neutral (1,1,1); random 16×12 image | Identity matrix within 1e-6; samples within 1 | `pictura-codec::pro_photo_camera_passes_through` |
| `camera_colour_srgb_camera` | ColorMatrix1 = XYZ → linear sRGB, D65, neutral (1,1,1); a set of sRGB colours | Published adapted matrix then ProPhoto, within 0.002 | `pictura-codec::srgb_camera_lands_on_pro_photo` |
| `camera_colour_neutral` | Two arbitrary matrices (A, D65), neutral (0.45, 1, 0.7); pixels n × {0.1, 0.5, 1}, and saturation | Equal channels at the scale within 2; saturation → white | `pictura-codec::camera_neutral_becomes_grey` |
| `camera_colour_calibration` | ProPhoto camera with CameraCalibration1 diag(1.1, 1, 0.9) and AnalogBalance (0.95, 1, 1.05); neutral = their response to D50 | Identity matrix within 1e-6 | `pictura-codec::calibration_and_analog_balance_apply` |
| `camera_colour_forward_matrix` | ColorMatrix1 and a disagreeing ForwardMatrix1 | Matrix = normalized ProPhoto · FM within 1e-6 | `pictura-codec::forward_matrix_is_preferred` |
| `camera_colour_interpolation` | A and D65 matrices; neutral from each matrix at its own white; then 4000 K via `white_balance_transform` against a blended single matrix | 6504 ± 15 K, 2856 ± 10 K; blend equal within 1e-9 | `pictura-codec::illuminants_interpolate_in_inverse_temperature` |
| `camera_colour_white_balance_round_trip` | `white_balance_transform` at the as-shot temperature and tint | Gains and matrix within 1e-4 relative | `pictura-codec::white_balance_round_trips` |
| `camera_colour_default_white` | ColorMatrix1 only, no AsShotNeutral | 5003 ± 15 K | `pictura-codec::missing_neutral_defaults_to_d50` |
| `camera_colour_refused` | No matrix; zero ColorMatrix1; AsShotNeutral (0, 1, 1) | Unsupported, Unsupported, Invalid | `pictura-codec::unusable_colour_tags_are_refused` |
| `camera_colour_m9_corpus` | `PICTURA_DNG_CORPUS=~/Downloads cargo test -p pictura-codec --test camera_raw dng_camera_colour::leica` | 2000–15000 K; 5216 × 3472; not all black | `pictura-codec::leica_samples_convert_when_present` |
