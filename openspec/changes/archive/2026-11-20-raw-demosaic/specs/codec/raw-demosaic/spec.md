## ADDED Requirements

### Requirement: The CFA plane is demosaiced to camera RGB

`demosaic` SHALL turn a `DngImage` into linear camera-native RGB with three
16-bit channels per pixel, the size of the active area, on the reader's
0–65535 scale, with no white balance or colour matrix applied. Each photosite
SHALL keep its own sampled channel unchanged. Parity with Adobe Camera Raw is
approximate: the algorithms are published ones, not Adobe's.

#### Scenario: Sampled channels are kept [demosaic_identity]

- **WHEN** a random plane under each Bayer phase (RGGB, BGGR, GRBG, GBRG) and a 6×6 X-Trans-shaped pattern is demosaiced with each method
- **THEN** every pixel's channel of its own CFA colour equals its sample

#### Scenario: A flat field is unchanged [demosaic_flat]

- **WHEN** a plane mosaiced from one constant RGB colour is demosaiced with each method and pattern
- **THEN** every output pixel equals that colour

### Requirement: Bilinear demosaic averages same-colour neighbours

`DemosaicMethod::Bilinear` SHALL set each missing channel to the rounded mean
of the photosites of that colour within the smallest square window around the
pixel, clipped to the image, that holds one, for any red-green-blue repeat
pattern.

#### Scenario: A gradient stays monotonic [demosaic_gradient]

- **WHEN** a grey ramp rising left to right (and, separately, top to bottom) is mosaiced and demosaiced with each method
- **THEN** every channel is non-decreasing along the ramp, and away from the border equals the ramp within 1

### Requirement: AHD demosaic follows edges on Bayer patterns

`DemosaicMethod::Ahd` SHALL implement adaptive homogeneity-directed
demosaicing (Hirakawa & Parks, 2005) for 2×2 Bayer patterns: green is
interpolated horizontally and vertically, red and blue follow from colour
differences, and each pixel takes the direction more homogeneous in CIELab over
its neighbourhood. A pattern that is not 2×2 Bayer, or an image smaller than
8 × 8, SHALL produce the bilinear result.

#### Scenario: Less false colour at an edge [demosaic_edge]

- **WHEN** grey images with a sharp vertical edge and a sharp horizontal edge are mosaiced and demosaiced with each method
- **THEN** AHD's largest channel difference from the grey value near the edge is under a quarter of bilinear's

#### Scenario: Fallback to bilinear [demosaic_fallback]

- **WHEN** an X-Trans-shaped plane and 1×1, 3×2 and 7×7 Bayer planes are demosaiced with AHD
- **THEN** each equals the bilinear result and none panics

### Requirement: Invalid patterns and oversize outputs are refused

A CFA pattern missing red, green or blue SHALL fail with `PsdError::Invalid`.
The `width · height · 6` byte output SHALL be checked against `ImageBudget`
before it is allocated, and fail with `PsdError::Invalid` when it does not fit.

#### Scenario: Refused inputs [demosaic_refused]

- **WHEN** a plane with a red-green-only pattern is demosaiced, and a valid plane is demosaiced under a budget smaller than its output
- **THEN** each fails with `PsdError::Invalid`

### Requirement: Camera-authored DNGs demosaic

A camera-authored DNG SHALL demosaic when the user supplies it; the check is
opt-in because such files are not committable.

#### Scenario: Leica M9 samples [demosaic_m9_corpus]

- **WHEN** `PICTURA_DNG_CORPUS` names a directory holding the Leica M9 `L1.dng` and `L2.dng`
- **THEN** each demosaics with both methods to 5216 × 3472 RGB with every photosite's own channel equal to its sample
