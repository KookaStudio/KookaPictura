# Proposal: the raw demosaic stage (#397)

## Why

`read_dng` (#396) yields a normalized CFA sensor plane, one sample per
photosite. Opening a camera raw file (#86) needs full RGB at every pixel before
the colour transform (#398) and the File ▸ Open route (#400). Adobe's demosaic
is closed, so this stage uses published algorithms and claims approximate
parity only (`FILT-100` §Algorithms step 1,
`docs/00-overview/feasibility-and-non-goals.md`).

## What changes

- `pictura_codec::demosaic(image, method, budget)` turns a `DngImage` into
  `CameraRgb`: linear camera-native RGB, 16 bits per channel, the size of the
  active area. Default crop and orientation stay with the later stages so the
  border photosites still feed the interpolation.
- `DemosaicMethod::Bilinear` averages the nearest same-colour photosites and
  works for any red-green-blue repeat pattern (Bayer phases and 6×6
  X-Trans-shaped layouts alike).
- `DemosaicMethod::Ahd` is adaptive homogeneity-directed demosaicing
  (Hirakawa & Parks, 2005) for 2×2 Bayer patterns; any other pattern, or an
  image too small for its window, uses the bilinear result.
- Every photosite keeps its own sampled channel unchanged. The output is
  charged to `ImageBudget`; AHD works in row bands so its scratch memory does
  not scale with the image.
- Nothing in the app changes yet.

## Capabilities

- New: `codec/raw-demosaic`.

## Provenance

Bilinear is textbook. AHD is implemented from the Hirakawa–Parks paper
("Adaptive homogeneity-directed demosaicing algorithm", IEEE TIP 14(3), 2005);
no code from dcraw, LibRaw, RawTherapee or rawspeed. Parity with Adobe Camera
Raw is approximate and not measured.
