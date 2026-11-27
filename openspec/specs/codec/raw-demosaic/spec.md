# codec/raw-demosaic Specification

## Purpose
The raw demosaic stage: a DNG's normalized CFA plane turned into linear camera-native RGB by a published bilinear or AHD algorithm, as the second stage of opening camera raw files (#86).

## Requirements

### Requirement: The CFA plane is demosaiced to camera RGB

`demosaic` SHALL turn a `DngImage` into linear camera-native RGB with three
16-bit channels per pixel, on the reader's 0–65535 scale, with no white
balance or colour matrix applied. Without a `fuji_width` the output SHALL be
the size of the active area and each photosite SHALL keep its own sampled
channel unchanged; with one, the output SHALL be rotated upright as "A
SuperCCD grid is rotated upright after the demosaic" requires. Parity with
Adobe Camera Raw is approximate: the algorithms are published ones, not
Adobe's.

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
its neighbourhood. A 6×6 X-Trans pattern SHALL take the X-Trans method. Any
other pattern, or a Bayer image smaller than 8 × 8, SHALL produce the
bilinear result.

#### Scenario: Less false colour at an edge [demosaic_edge]

- **WHEN** grey images with a sharp vertical edge and a sharp horizontal edge are mosaiced and demosaiced with each method
- **THEN** AHD's largest channel difference from the grey value near the edge is under a quarter of bilinear's

#### Scenario: Fallback to bilinear [demosaic_fallback]

- **WHEN** a 3×3 red-green-blue plane and 1×1, 3×2 and 7×7 Bayer planes are demosaiced with AHD
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

### Requirement: X-Trans patterns demosaic along edges

`DemosaicMethod::Ahd` on a 6×6 pattern in which every 5 × 5 window holds red,
green and blue and every red or blue photosite has a green within three
photosites along its row or column SHALL use Pictura's own edge-directed
method: green at a red or blue photosite SHALL blend linear interpolations
between the nearest greens along the row and along the column, each weighted
by `1 / (1 + a)²` with `a` the mean absolute difference of same-colour
photosites one or two apart along that direction near the pixel; red and blue
SHALL be the pixel's green plus a mean of the colour differences (sample
minus green) of that colour's photosites in the 5 × 5 window, weighted down
by distance and by green difference, widened at the image border until the
colour appears. Each photosite SHALL keep its own sample. The method is not a
published one; parity with other converters is approximate.

#### Scenario: A ramp is reproduced [xtrans_gradient]

- **WHEN** grey ramps rising left to right and top to bottom are mosaiced through an X-Trans pattern and demosaiced with AHD
- **THEN** every channel is non-decreasing along the ramp and equals the ramp within 1 everywhere

#### Scenario: Less false colour at an X-Trans edge [xtrans_edge]

- **WHEN** grey images with a sharp vertical edge and a sharp horizontal edge are mosaiced through an X-Trans pattern and demosaiced with each method
- **THEN** AHD's largest channel difference from the grey value near the edge is under a quarter of bilinear's

#### Scenario: Patterns that are not X-Trans keep bilinear [xtrans_applies]

- **WHEN** a 6×6 pattern whose first four rows are only green (so some 5 × 5 window lacks red) is demosaiced with AHD
- **THEN** it equals the bilinear result

### Requirement: A SuperCCD grid is rotated upright after the demosaic

For an image with a `fuji_width` `fw`, `demosaic` SHALL demosaic the grid
with the chosen method, then rotate it as LibRaw 0.22.1's `fuji_rotate`
does: with `f = fw − 1` and `s = √½`, the output SHALL be `⌊f / s⌋` wide and
`⌊(H − f) / s⌋` high for a grid `H` high, and output pixel `(row, col)`
SHALL be the grid sampled bilinearly at row `f + (row − col)·s` and column
`(row + col)·s`, truncated to an integer, or black when that position is
negative or its integer part lies past the grid's second-last row or
column. The rotated output SHALL be charged against the image budget with
the demosaiced grid. Parity with LibRaw's rendering is by geometry; the
demosaic itself is ours.

#### Scenario: The rotated size [superccd_rotate_size]

- **WHEN** SuperCCD images of each layout bit with odd and even `fw` are demosaiced, and one whose rotated output exceeds the budget
- **THEN** each output has the formula's size, and the oversized one is refused

#### Scenario: Rotation samples LibRaw's positions [superccd_rotate_positions]

- **WHEN** a SuperCCD grid mosaiced from an RGB image linear in row and column is demosaiced with each method
- **THEN** every output pixel whose sample position lies at least six cells inside the grid equals that image at LibRaw's position within 2

#### Scenario: Real SuperCCD renders match LibRaw [superccd_rotate_libraw]

- **WHEN** `PICTURA_RAW_CORPUS` holds SuperCCD RAFs and python3 with rawpy is available, the SR files read with header offset `0x80` cleared so only their primary frame is read, as LibRaw reads them
- **THEN** each demosaiced and oriented image has the size of LibRaw's linear, unscaled camera-RGB rendering, and each channel's 8 × 8 block means correlate with LibRaw's at 0.99 or more
