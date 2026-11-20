# Proposal: camera colour matrices and the working-space transform (#398)

## Why

The demosaic stage (#397) yields linear camera-native RGB. Opening a camera raw
file (#86) needs that in a defined colour space, white-balanced, before the
File ▸ Open route (#400) converts it to the document's profile. A DNG carries
everything needed: `ColorMatrix1/2`, `ForwardMatrix1/2`, `CameraCalibration1/2`,
`AnalogBalance` and `AsShotNeutral`, already parsed by `read_dng` (#396). Adobe
DCP camera profiles are proprietary and are not used (`FILT-100` §Algorithms
steps 2–3).

## What changes

- `pictura_codec::as_shot_transform(&DngColor)` builds a `CameraTransform`
  from the DNG's own tags, following the DNG specification's camera-to-XYZ
  mapping: the matrices are interpolated between the two calibration
  illuminants by inverse colour temperature, `ForwardMatrix` is used when
  present and a Bradford adaptation to D50 otherwise. It reports the as-shot
  white balance as a colour temperature in kelvin and a tint, so #400 can seed
  the UI.
- `pictura_codec::white_balance_transform(&DngColor, WhiteBalance)` builds the
  transform for any other temperature and tint.
- `pictura_codec::to_pro_photo(CameraRgb, &CameraTransform)` applies the white
  balance and the matrix in place and returns `ProPhotoRgb`: linear ProPhoto
  (ROMM primaries, D50), 16 bits per channel.
- A DNG with no `ColorMatrix`, or whose matrices cannot be inverted, is refused
  with `PsdError::Unsupported`.
- `pictura_color::chroma` gains the shared colour math: 3×3 matrices, the
  RGB-to-XYZ matrix from primaries, Bradford adaptation, and conversion between
  colour temperature with tint and CIE xy.
- `pictura_color::Profile::linear_pro_photo()` is the ICC profile of the
  stage's output, so the existing lcms2 `convert` takes it to any working
  space.
- Nothing in the app changes yet; Pictura Raw's relative Temperature and Tint
  sliders are untouched until #400.

## Capabilities

- New: `codec/camera-colour`, `color/colour-temperature`.

## Provenance

The camera-to-XYZ mapping follows the public DNG 1.6 specification, chapter 6
("Mapping Camera Color Space to CIE XYZ Space"). The Planckian locus is
Krystek's 1985 rational approximation; Bradford is the published Lam (1985)
cone matrix. No code from the DNG SDK, dcraw, LibRaw or RawTherapee. The tint
scale and the result's match to Adobe Camera Raw are approximate and not
measured.
